//! The value editors (docs/code-helpers-plan.md §1, E1): one colour editor,
//! one font editor, one file and picture editor, one list of choices and
//! one of links, opened as a popover over any component — the code editor's
//! helpers (Ctrl+. or a click on a mark) and the Properties panel — so they
//! look and work the same everywhere, on the desktop and in the browser.
//!
//! A popover belongs to a component (its node id) and hangs from a place
//! of it (the value's mark, a row). While it is open it has the keyboard
//! (Tab moves inside it, Enter keeps, Esc drops; the arrows and Page keys
//! move in its lists and sliders), a press outside it drops it, and a
//! screen reader finds it as a dialog inside its component with each part
//! named and the focused one marked — the focus never leaves the component
//! (no jump). What it picks goes back to its owner ([`Owner`]): the code
//! editor writes it into the code as one undo step, in the code's own form;
//! the Properties panel sets the property.

pub mod color;
pub mod field;
pub mod list;

use std::cell::RefCell;
use std::collections::HashMap;

use rapidr_editor::service::HelperSite;
use rapidr_lang::kinds::FileKind;
use rapidr_value::objects::a11y::{node_id, AccessNode, Role};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::{Place, Rect};
use rapidr_value::objects::text::text_size;
use rapidr_value::theme::Theme;

use self::color::ColorEd;
use self::field::{Accept, Field};
use self::list::{Item, ListEd};
use crate::input::Mods;
use crate::paint::Painter;
use crate::store::Store;
use crate::text::TextSystem;
use crate::tree::FormUi;

/// The colours a popover is drawn in: the theme's tokens, so it follows the
/// look (light, dark, high contrast) like the rest of Studio.
#[derive(Clone, Copy, Debug)]
pub struct Look {
    pub bg: u32,
    pub border: u32,
    pub border_strong: u32,
    pub text: u32,
    pub dim: u32,
    pub field: u32,
    pub focus: u32,
    pub accent: u32,
    /// Text in the accent's colour on `bg` (an action).
    pub accent_text: u32,
    /// Text and marks on the accent.
    pub accent_on: u32,
    pub selected: u32,
    pub selected_text: u32,
    pub unfocused: u32,
    pub error: u32,
    pub shadow: u32,
}

impl Look {
    pub fn of(t: &Theme) -> Look {
        let code = rapidr_value::code_scheme::for_theme(t);
        let legible = |c: u32| (0..=10).map(|k| blend(t.text, c, f64::from(k) / 10.0)).find(|&x| rapidr_value::theme::contrast(x, t.window) >= 4.5).unwrap_or(t.text);
        Look {
            bg: t.window,
            border: t.border,
            border_strong: t.border_strong,
            text: t.text,
            dim: legible(t.gray_text),
            field: t.window,
            focus: t.focus,
            accent: t.accent,
            accent_text: legible(t.accent),
            accent_on: t.accent_text,
            selected: t.highlight,
            selected_text: t.highlight_text,
            unfocused: t.unfocused,
            error: legible(code.error),
            shadow: t.shadow_ink,
        }
    }
}

fn blend(fg: u32, bg: u32, a: f64) -> u32 {
    let ch = |s: u32| (f64::from((fg >> s) & 0xFF) * a + f64::from((bg >> s) & 0xFF) * (1.0 - a)).round() as u32;
    (ch(16) << 16) | (ch(8) << 8) | ch(0)
}

/// What the user did in a popover.
#[derive(Clone, Debug, PartialEq)]
pub enum Picked {
    /// Handled; it stays open.
    Nothing,
    /// Dropped (Esc, Cancel, a press outside).
    Cancel,
    Color(i64),
    /// A choice's constant, or the flags ticked.
    Choice(Vec<String>),
    Bool(bool),
    /// A file's, a font's or a link's name.
    Text(String),
    /// A font with its size and style (the Properties panel's).
    Font { name: String, size: i64, bold: bool, italic: bool },
    /// "Create handler": a new SUB, its name.
    CreateHandler(String),
}

/// Whose a popover is: where what it picks goes.
#[derive(Clone, Debug, PartialEq)]
pub enum Owner {
    /// A code editor's helper (the site as it was when it opened).
    Code(HelperSite),
    /// A Properties panel's row (its key).
    Inspector(String),
}

/// A list's editor's job.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListKind {
    /// One constant.
    Choice,
    /// Flags, ticked.
    Flags,
    /// True or False.
    Bool,
    /// A component or a SUB (the first item may be "Create handler").
    Link,
    File(FileKind),
    Font,
}

/// The editor in a popover.
#[derive(Clone, Debug, PartialEq)]
pub enum Editor {
    Color(ColorEd),
    List {
        list: ListEd,
        kind: ListKind,
        /// "Create handler"'s new SUB's name (the list's first item).
        create: Option<String>,
        /// A font's size and style (the Properties panel's font).
        size: Option<Field>,
        style: Option<(bool, bool)>,
        /// Which part has the keys: 0 the list (and its filter), 1 the
        /// size, 2 Bold, 3 Italic.
        part: usize,
        /// A file's folder (its preview read from there) and the previews.
        folder: String,
    },
}

/// An open popover.
#[derive(Clone, Debug, PartialEq)]
pub struct Popover {
    pub editor: Editor,
    pub owner: Owner,
    /// Its heading ("Colour", "Choose", "Handler" …).
    pub title: String,
    /// What it hangs from (absolute logical pixels of the form).
    pub anchor: Rect,
    /// Where it is (absolute), as last placed.
    pub rect: Rect,
}

const TITLE_H: i64 = 30;
const PAD: i64 = 10;
/// A font's preview line, its size and style row, a picture's preview.
const PREVIEW_H: i64 = 48;
const ROW_H: i64 = 28;
const PICTURE_H: i64 = 140;
/// Apply and Cancel's row, with its margins.
const BUTTONS_H: i64 = 8 + 30 + 8;

impl Popover {
    /// A colour editor of `value` (the project's colours its palette).
    pub fn color(owner: Owner, value: i64, project: &[i64], anchor: Rect) -> Popover {
        Popover { editor: Editor::Color(ColorEd::new(value, project)), owner, title: "Colour".into(), anchor, rect: (0, 0, 0, 0) }
    }

    /// A list editor.
    pub fn list(owner: Owner, kind: ListKind, items: Vec<Item>, selected: Option<usize>, anchor: Rect) -> Popover {
        let filter = matches!(kind, ListKind::File(_) | ListKind::Font) || items.len() > 12;
        let title = match kind {
            ListKind::Choice => "Choose a value",
            ListKind::Flags => "Choose the flags",
            ListKind::Bool => "True or False",
            ListKind::Link => "Link",
            ListKind::File(FileKind::Image) => "Picture",
            ListKind::File(FileKind::Sound) => "Sound",
            ListKind::File(FileKind::Any) => "File",
            ListKind::Font => "Font",
        };
        let mut list = ListEd::new(items, selected, filter);
        if let Some(f) = &mut list.filter {
            f.focus();
            f.all = false;
        }
        let editor = Editor::List { list, kind, create: None, size: None, style: None, part: 0, folder: String::new() };
        Popover { editor, owner, title: title.into(), anchor, rect: (0, 0, 0, 0) }
    }

    /// Its size (logical pixels).
    pub fn size(&self) -> (i64, i64) {
        match &self.editor {
            Editor::Color(c) => (color::WIDTH, TITLE_H + c.height()),
            Editor::List { list, kind, size, style, .. } => {
                let w = match kind {
                    ListKind::File(FileKind::Image) => 420,
                    ListKind::Font => 380,
                    _ => 320,
                };
                // (under the list: a font's preview, its size and style
                // row, Apply and Cancel; a picture's preview beside it)
                let list_bottom = 8 + list.height();
                let body = match kind {
                    ListKind::Font => list_bottom + 6 + PREVIEW_H + if size.is_some() || style.is_some() { 8 + ROW_H } else { 0 } + BUTTONS_H,
                    ListKind::Flags => list_bottom + BUTTONS_H,
                    ListKind::File(FileKind::Image) => (list_bottom + 8).max(8 + PICTURE_H + 6 + 20 + 8),
                    _ => list_bottom + 8,
                };
                (w, TITLE_H + body)
            }
        }
    }

    /// Placed under its anchor, else over it; where neither has room,
    /// beside it (its right, else its left), as high as fits — the value it
    /// edits stays in sight; else as whole as the form's `client` area
    /// allows.
    pub fn place(&mut self, client: (i64, i64)) {
        let (w, h) = self.size();
        let (ax, ay, aw, ah) = self.anchor;
        let below = ay + ah + 4;
        let above = ay - h - 4;
        let x = ax.clamp(0, (client.0 - w).max(0));
        let (x, y) = if below + h <= client.1 {
            (x, below)
        } else if above >= 0 {
            (x, above)
        } else {
            let y = (ay + ah / 2 - h / 2).clamp(0, (client.1 - h).max(0));
            if ax + aw + 8 + w <= client.0 {
                (ax + aw + 8, y)
            } else if ax - 8 - w >= 0 {
                (ax - 8 - w, y)
            } else {
                (x, y)
            }
        };
        self.rect = (x, y.max(0), w, h);
    }

    /// What it shows, in words (the owner's HelperEditorText; its dialog's
    /// description).
    pub fn said(&self) -> String {
        match &self.editor {
            Editor::Color(c) => c.said(),
            Editor::List { list, kind, size, style, .. } => {
                let sel = list.selected().map(|i| list.items[i].said()).unwrap_or_default();
                match kind {
                    ListKind::Flags => format!("{}: {}", self.title, list.ticked().join(" + ")),
                    ListKind::Font => {
                        let mut s = format!("Font: {sel}");
                        if let Some(f) = size {
                            s.push_str(&format!(", {} pt", f.text));
                        }
                        if let Some((b, i)) = style {
                            if *b {
                                s.push_str(", bold");
                            }
                            if *i {
                                s.push_str(", italic");
                            }
                        }
                        s
                    }
                    _ => format!("{}: {sel}", self.title),
                }
            }
        }
    }

    /// A key: what it did (`None`: not its key).
    pub fn key(&mut self, vk: i64, text: &str, mods: Mods) -> Option<Picked> {
        if vk == 27 {
            return Some(Picked::Cancel);
        }
        match &mut self.editor {
            Editor::Color(c) => {
                if vk == 9 {
                    c.tab(mods.shift);
                    return Some(Picked::Nothing);
                }
                if vk == 13 {
                    return Some(if c.part() == color::Part::Cancel { Picked::Cancel } else { Picked::Color(c.value) });
                }
                c.key(vk, text, mods.shift).or(Some(Picked::Nothing))
            }
            Editor::List { list, kind, create, size, style, part, .. } => {
                let parts = 1 + usize::from(size.is_some()) + if style.is_some() { 2 } else { 0 };
                if vk == 9 {
                    *part = if mods.shift { (*part + parts - 1) % parts } else { (*part + 1) % parts };
                    if *part == 1 {
                        if let Some(f) = size {
                            f.focus();
                        }
                    }
                    return Some(Picked::Nothing);
                }
                if vk == 13 {
                    return Some(pick(list, *kind, create.as_deref(), size.as_ref(), *style));
                }
                match *part {
                    1 if size.is_some() => {
                        let f = size.as_mut()?;
                        let _ = f.key(vk, text);
                        return Some(Picked::Nothing);
                    }
                    p @ (2 | 3) if vk == 32 => {
                        if let Some((b, i)) = style {
                            if p == 2 {
                                *b = !*b;
                            } else {
                                *i = !*i;
                            }
                        }
                        return Some(Picked::Nothing);
                    }
                    0 => {}
                    _ => return Some(Picked::Nothing),
                }
                // (True / False: Space turns it over)
                if *kind == ListKind::Bool && vk == 32 {
                    let k = if list.sel == 0 { 1 } else { 0 };
                    list.sel = k;
                    return Some(Picked::Nothing);
                }
                list.key(vk, text);
                Some(Picked::Nothing)
            }
        }
    }

    /// A press at absolute (x, y) inside it.
    pub fn mouse_down(&mut self, x: i64, y: i64, clicks: u8) -> Picked {
        let (ox, oy, _, h) = self.rect;
        let (rx, ry) = (x - ox, y - oy - TITLE_H);
        let body_h = h - TITLE_H;
        match &mut self.editor {
            Editor::Color(c) => c.mouse_down(rx, ry, body_h).unwrap_or(Picked::Nothing),
            Editor::List { list, kind, create, size, style, part, .. } => {
                let lr = list_rect(list, (0, 0, self.rect.2, body_h), *kind);
                if let Some(k) = list.at(lr, rx, ry) {
                    list.sel = k;
                    *part = 0;
                    if list.ticks.is_some() {
                        list.toggle();
                        return Picked::Nothing;
                    }
                    // (a font, a file: picked by a second click or Enter, so
                    // the preview can be looked at first)
                    if matches!(kind, ListKind::Font | ListKind::File(_)) && clicks < 2 && !list.items[list.shown[k]].action {
                        return Picked::Nothing;
                    }
                    return pick(list, *kind, create.as_deref(), size.as_ref(), *style);
                }
                if let Some(f) = &mut list.filter {
                    if ry < list::ROW + 6 && ry >= 0 {
                        *part = 0;
                        f.all = false;
                        return Picked::Nothing;
                    }
                }
                // (the font's size and style; the buttons)
                let (apply, cancel) = buttons(self.rect.2, body_h);
                let inside = |r: Rect| rx >= r.0 && ry >= r.1 && rx < r.0 + r.2 && ry < r.1 + r.3;
                if matches!(kind, ListKind::Flags | ListKind::Font) && inside(apply) {
                    return pick(list, *kind, create.as_deref(), size.as_ref(), *style);
                }
                if matches!(kind, ListKind::Flags | ListKind::Font) && inside(cancel) {
                    return Picked::Cancel;
                }
                if *kind == ListKind::Font {
                    let (sr, br, ir) = font_parts(lr, self.rect.2);
                    if size.is_some() && inside(sr) {
                        *part = 1;
                        if let Some(f) = size {
                            f.focus();
                        }
                    } else if let Some((b, i)) = style {
                        if inside(br) {
                            *b = !*b;
                            *part = 2;
                        } else if inside(ir) {
                            *i = !*i;
                            *part = 3;
                        }
                    }
                }
                Picked::Nothing
            }
        }
    }

    pub fn mouse_move(&mut self, x: i64, y: i64) -> bool {
        let (ox, oy, _, _) = self.rect;
        if let Editor::Color(c) = &mut self.editor {
            if c.drag.is_some() {
                c.drag_to(x - ox, y - oy - TITLE_H);
                return true;
            }
        }
        false
    }

    pub fn mouse_up(&mut self) -> bool {
        if let Editor::Color(c) = &mut self.editor {
            return c.drag.take().is_some();
        }
        false
    }

    pub fn wheel(&mut self, dy: f64) {
        if let Editor::List { list, .. } = &mut self.editor {
            list.wheel(dy);
        }
    }

    /// Draws it (its shadow, frame, heading and editor).
    pub fn paint(&self, p: &mut Painter, look: &Look, font: &Font) {
        let r = self.rect;
        p.op(rapidr_value::objects::ops::Op::Shadow { rect: r, radius: 6.0, size: 10.0, drop: 3.0, color: look.shadow, alpha: 60 });
        p.round(r, 6.0, Some(look.bg), Some(look.border_strong), 1.0);
        let bold = Font { styles: 1, ..font.clone() };
        let th = text_size("Ag", font).1;
        p.text((r.0 + PAD + 2, r.1 + (TITLE_H - th) / 2 + 2, r.2 - 2 * PAD, th), &self.title, &bold, look.text, Place::TopLeft);
        let keys = match &self.editor {
            Editor::Color(_) => "Enter keeps · Esc cancels",
            Editor::List { kind: ListKind::Flags, .. } => "Space ticks · Enter keeps",
            Editor::List { kind: ListKind::Bool, .. } => "Space turns · Enter keeps",
            _ => "Enter picks · Esc cancels",
        };
        // (the keys' reminder where it fits beside the heading — whole or
        // not at all; the dialog's description says them too)
        let kw = text_size(keys, font).0;
        if text_size(&self.title, &bold).0 + kw + 3 * PAD <= r.2 {
            p.text((r.0 + r.2 - PAD - kw, r.1 + (TITLE_H - th) / 2 + 2, kw + 2, th), keys, font, look.dim, Place::TopLeft);
        }
        p.fill((r.0 + 1, r.1 + TITLE_H - 1, r.2 - 2, 1), look.border);
        let body = (r.0, r.1 + TITLE_H, r.2, r.3 - TITLE_H);
        match &self.editor {
            Editor::Color(c) => c.paint(p, (body.0, body.1), body.3, look, font),
            Editor::List { list, kind, size, style, part, folder, .. } => {
                let lr = list_rect(list, (0, 0, body.2, body.3), *kind);
                let lr_abs = (body.0 + lr.0, body.1 + lr.1, lr.2, lr.3);
                list.paint(p, lr_abs, *part == 0, *part == 0 && list.filter.is_some(), look, font);
                match kind {
                    ListKind::File(FileKind::Image) => {
                        // (the selected picture's preview, beside the list)
                        let pv = (body.0 + lr.0 + lr.2 + 10, body.1 + lr.1, body.2 - lr.2 - lr.0 - 2 * PAD, PICTURE_H);
                        p.round(pv, 4.0, Some(look.field), Some(look.border), 1.0);
                        if let Some(i) = list.selected() {
                            let it = &list.items[i];
                            // (sharp at twice the preview's size: a screen's
                            // pixels at 2x)
                            match crate::components::codeeditor::helpers::thumbnail_sized(folder, &it.label, 2 * PICTURE_H as usize) {
                                Some(pic) => {
                                    let fit = fit_into((pv.0 + 6, pv.1 + 6, pv.2 - 12, pv.3 - 12), pic.width as i64, pic.height as i64);
                                    p.picture(&format!("value-edit/preview/{folder}/{}", it.label), 1, pic, fit);
                                }
                                None => {
                                    let msg = if it.missing { "Not in the project" } else { "No preview" };
                                    p.text((pv.0 + 8, pv.1 + pv.3 / 2 - th / 2, pv.2 - 16, th), msg, font, if it.missing { look.error } else { look.dim }, Place::Center);
                                }
                            }
                            p.text((pv.0, pv.1 + pv.3 + 6, pv.2, th), &it.label, font, look.dim, Place::Center);
                        }
                    }
                    ListKind::Font => {
                        let name = list.selected().map(|i| list.items[i].label.clone()).unwrap_or_default();
                        let pt = size.as_ref().and_then(|f| f.text.parse::<i64>().ok()).unwrap_or(14).clamp(6, 36);
                        let (b, it) = style.unwrap_or((false, false));
                        let sample = Font { name: name.clone(), size: pt, styles: u8::from(b) | (u8::from(it) << 1), ..Font::default() };
                        let pr = (body.0 + PAD, body.1 + lr.1 + lr.3 + 6, body.2 - 2 * PAD, PREVIEW_H);
                        p.round(pr, 4.0, Some(look.field), Some(look.border), 1.0);
                        p.clipped(pr, |p| p.text((pr.0 + 8, pr.1, pr.2 - 16, pr.3), "The quick brown fox 0123", &sample, look.text, Place::Left));
                        let (sr, br, ir) = font_parts(lr, body.2);
                        if let Some(f) = size {
                            f.paint(p, (body.0 + sr.0, body.1 + sr.1, sr.2, sr.3), 40, *part == 1, look, font);
                        }
                        if let Some((bo, itl)) = style {
                            toggle(p, (body.0 + br.0, body.1 + br.1, br.2, br.3), "Bold", *bo, *part == 2, look, font);
                            toggle(p, (body.0 + ir.0, body.1 + ir.1, ir.2, ir.3), "Italic", *itl, *part == 3, look, font);
                        }
                    }
                    _ => {}
                }
                if matches!(kind, ListKind::Flags | ListKind::Font) {
                    let (apply, cancel) = buttons(body.2, body.3);
                    button(p, (body.0 + cancel.0, body.1 + cancel.1, cancel.2, cancel.3), "Cancel", false, false, look, font);
                    button(p, (body.0 + apply.0, body.1 + apply.1, apply.2, apply.3), "Apply", true, false, look, font);
                }
            }
        }
    }

    /// Its dialog node for screen readers (ids under `base`).
    pub fn access(&self, base: &str) -> AccessNode {
        let mut d = AccessNode::new(node_id(&format!("{base}$valueeditor")), Role::Dialog);
        d.name = format!("{} editor", self.title);
        d.description = self.said();
        d.bounds = self.rect;
        let origin = (self.rect.0, self.rect.1);
        match &self.editor {
            Editor::Color(c) => d.children = c.access(base, origin),
            Editor::List { list, kind, size, style, part, .. } => {
                if let Some(f) = &list.filter {
                    let mut n = AccessNode::new(node_id(&format!("{base}$filter")), Role::TextInput);
                    n.name = "Filter".into();
                    n.value = Some(f.text.clone());
                    n.bounds = self.rect;
                    d.children.push(n);
                }
                let mut lb = AccessNode::new(node_id(&format!("{base}$choices")), Role::ListBox);
                lb.name = self.title.clone();
                lb.bounds = self.rect;
                for (k, &i) in list.shown.iter().enumerate().take(400) {
                    let it = &list.items[i];
                    let mut o = AccessNode::new(node_id(&format!("{base}$choice${k}")), if list.ticks.is_some() { Role::CheckBox } else { Role::ListBoxOption });
                    o.name = it.said();
                    if let Some(t) = &list.ticks {
                        o.states.checked = Some(t.get(i).copied().unwrap_or(false));
                    } else {
                        o.states.selected = Some(k == list.sel);
                    }
                    o.states.focused = *part == 0 && k == list.sel;
                    o.bounds = self.rect;
                    lb.children.push(o);
                }
                d.children.push(lb);
                if let Some(f) = size {
                    let mut n = AccessNode::new(node_id(&format!("{base}$size")), Role::TextInput);
                    n.name = "Size".into();
                    n.value = Some(f.text.clone());
                    n.states.focused = *part == 1;
                    n.bounds = self.rect;
                    d.children.push(n);
                }
                if let Some((b, i)) = style {
                    for (k, (name, on)) in [("Bold", *b), ("Italic", *i)].into_iter().enumerate() {
                        let mut n = AccessNode::new(node_id(&format!("{base}${name}")), Role::CheckBox);
                        n.name = name.into();
                        n.states.checked = Some(on);
                        n.states.focused = *part == 2 + k;
                        n.bounds = self.rect;
                        d.children.push(n);
                    }
                }
                let _ = kind;
            }
        }
        d
    }
}

/// The list's place in a list editor's body (relative).
fn list_rect(list: &ListEd, body: Rect, kind: ListKind) -> Rect {
    let w = match kind {
        ListKind::File(FileKind::Image) => 250,
        _ => body.2 - 2 * PAD,
    };
    (PAD, 8, w, list.height())
}

/// A font editor's size field, Bold and Italic toggles (relative), under
/// its preview.
fn font_parts(lr: Rect, w: i64) -> (Rect, Rect, Rect) {
    let y = lr.1 + lr.3 + 6 + PREVIEW_H + 8;
    ((PAD, y, 110, ROW_H), (PAD + 120, y, 80, ROW_H), (PAD + 210, y, w - PAD - 210 - PAD, ROW_H))
}

/// Apply and Cancel at a body's bottom right (relative).
fn buttons(w: i64, h: i64) -> (Rect, Rect) {
    let y = h - 30 - 8;
    ((w - PAD - 84, y, 84, 30), (w - PAD - 84 - 8 - 84, y, 84, 30))
}

/// What a list editor's Enter picks.
fn pick(list: &ListEd, kind: ListKind, create: Option<&str>, size: Option<&Field>, style: Option<(bool, bool)>) -> Picked {
    if kind == ListKind::Flags {
        return Picked::Choice(list.ticked());
    }
    let Some(i) = list.selected() else { return Picked::Nothing };
    let it = &list.items[i];
    if it.action {
        return Picked::CreateHandler(create.unwrap_or_default().to_string());
    }
    match kind {
        ListKind::Bool => Picked::Bool(it.label.eq_ignore_ascii_case("true")),
        ListKind::Choice => Picked::Choice(vec![it.label.clone()]),
        ListKind::Font if size.is_some() || style.is_some() => {
            let (bold, italic) = style.unwrap_or((false, false));
            Picked::Font { name: it.label.clone(), size: size.and_then(|f| f.text.parse().ok()).unwrap_or(10), bold, italic }
        }
        _ => Picked::Text(it.label.clone()),
    }
}

/// `w` × `h` fitted into `r`, centred, its shape kept.
pub fn fit_into(r: Rect, w: i64, h: i64) -> Rect {
    if w <= 0 || h <= 0 {
        return r;
    }
    let k = (r.2 as f64 / w as f64).min(r.3 as f64 / h as f64).min(4.0);
    let (fw, fh) = ((w as f64 * k).round() as i64, (h as f64 * k).round() as i64);
    (r.0 + (r.2 - fw) / 2, r.1 + (r.3 - fh) / 2, fw.max(1), fh.max(1))
}

/// A button (the default one in the accent).
pub fn button(p: &mut Painter, r: Rect, label: &str, default: bool, focused: bool, look: &Look, font: &Font) {
    let (fill, ink) = if default { (look.accent, look.accent_on) } else { (look.field, look.text) };
    p.round(r, 4.0, Some(fill), Some(if default { look.accent } else { look.border_strong }), 1.0);
    if focused {
        p.round((r.0 - 2, r.1 - 2, r.2 + 4, r.3 + 4), 5.0, None, Some(look.focus), 2.0);
    }
    p.text(r, label, font, ink, Place::Center);
}

/// A toggle: a box with its label (ticked: in the accent).
fn toggle(p: &mut Painter, r: Rect, label: &str, on: bool, focused: bool, look: &Look, font: &Font) {
    let b = (r.0, r.1 + (r.3 - 16) / 2, 16, 16);
    p.round(b, 3.0, Some(if on { look.accent } else { look.field }), Some(if on { look.accent } else { look.border_strong }), 1.0);
    if on {
        p.stroke(&[(b.0 as f64 + 3.5, b.1 as f64 + 8.5), (b.0 as f64 + 6.5, b.1 as f64 + 11.5), (b.0 as f64 + 12.5, b.1 as f64 + 4.5)], look.accent_on, 1.8);
    }
    if focused {
        p.round((r.0 - 3, r.1, r.2 + 6, r.3), 4.0, None, Some(look.focus), 2.0);
    }
    p.text((r.0 + 22, r.1, r.2 - 22, r.3), label, font, look.text, Place::Left);
}

// ---- the open popovers, by their component's node id -----------------------

thread_local! {
    static OPEN: RefCell<HashMap<String, Popover>> = RefCell::new(HashMap::new());
}

/// Opens `pop` for component `id` (placed in `client`).
pub fn open(id: &str, mut pop: Popover, client: (i64, i64)) {
    pop.place(client);
    OPEN.with(|o| o.borrow_mut().insert(id.to_string(), pop));
}

pub fn close(id: &str) -> Option<Popover> {
    OPEN.with(|o| o.borrow_mut().remove(id))
}

pub fn is_open(id: &str) -> bool {
    OPEN.with(|o| o.borrow().contains_key(id))
}

pub fn with<R>(id: &str, f: impl FnOnce(&mut Popover) -> R) -> Option<R> {
    OPEN.with(|o| o.borrow_mut().get_mut(id).map(f))
}

/// What component `id`'s popover shows, in words ("" none).
pub fn said(id: &str) -> String {
    with(id, |p| p.said()).unwrap_or_default()
}

/// Component `id`'s popover's dialog node, when one is open.
pub fn access(id: &str) -> Option<AccessNode> {
    with(id, |p| p.access(id))
}

/// The UI font popovers write in (the theme's chrome font).
pub fn ui_font() -> Font {
    rapidr_value::ide_theme::chrome_font(rapidr_value::theme::current())
}

/// Every open popover of form `f`'s components (its popup layer).
pub fn paint_all(f: &FormUi, p: &mut Painter) {
    let t = rapidr_value::theme::current();
    let look = Look::of(t);
    let font = ui_font();
    let client = (f.client.0, f.client.1 + f.menu_offset);
    for n in f.nodes.iter().filter(|n| n.shown) {
        with(&n.id, |pop| {
            pop.place(client);
            pop.paint(p, &look, &font);
        });
    }
}

/// The node whose popover holds absolute (x, y).
fn popover_at(f: &FormUi, x: f64, y: f64) -> Option<usize> {
    (0..f.nodes.len()).find(|&i| {
        with(&f.nodes[i].id, |p| {
            let r = p.rect;
            x >= r.0 as f64 && y >= r.1 as f64 && x < (r.0 + r.2) as f64 && y < (r.1 + r.3) as f64
        })
        .unwrap_or(false)
    })
}

/// A press: on a popover, its (true); elsewhere, every popover drops
/// (the press goes on: false).
pub fn mouse_down(f: &mut FormUi, store: &dyn Store, ts: &mut TextSystem, x: f64, y: f64, clicks: u8) -> bool {
    if let Some(i) = popover_at(f, x, y) {
        let id = f.nodes[i].id.clone();
        let picked = with(&id, |p| p.mouse_down(x as i64, y as i64, clicks)).unwrap_or(Picked::Nothing);
        f.dirty = true;
        finish(f, store, ts, i, picked);
        return true;
    }
    let open: Vec<usize> = (0..f.nodes.len()).filter(|&i| is_open(&f.nodes[i].id)).collect();
    for i in open {
        finish(f, store, ts, i, Picked::Cancel);
        f.dirty = true;
    }
    false
}

/// The mouse moved: a popover's drag (its wheel's, its square's).
pub fn mouse_move(f: &mut FormUi, store: &dyn Store, ts: &mut TextSystem, x: f64, y: f64) -> bool {
    let Some(i) = (0..f.nodes.len()).find(|&i| with(&f.nodes[i].id, |p| matches!(&p.editor, Editor::Color(c) if c.drag.is_some())).unwrap_or(false)) else { return false };
    let id = f.nodes[i].id.clone();
    with(&id, |p| p.mouse_move(x as i64, y as i64));
    f.dirty = true;
    sync(f, store, ts, i);
    true
}

pub fn mouse_up(f: &mut FormUi) -> bool {
    let mut any = false;
    for n in &f.nodes {
        any |= with(&n.id, Popover::mouse_up).unwrap_or(false);
    }
    if any {
        f.dirty = true;
    }
    any
}

pub fn wheel(f: &mut FormUi, x: f64, y: f64, dy: f64) -> bool {
    let Some(i) = popover_at(f, x, y) else { return false };
    let id = f.nodes[i].id.clone();
    with(&id, |p| p.wheel(dy));
    f.dirty = true;
    true
}

/// A key: the focused component's popover's, while one is open.
pub fn key(f: &mut FormUi, store: &dyn Store, ts: &mut TextSystem, vk: i64, text: &str, mods: Mods) -> bool {
    let Some(i) = f.focus else { return false };
    let id = f.nodes[i].id.clone();
    if !is_open(&id) {
        return false;
    }
    // (a modifier alone does nothing)
    if matches!(vk, 16 | 17 | 18 | 91 | 92 | 93) {
        return true;
    }
    let picked = with(&id, |p| p.key(vk, text, mods)).flatten().unwrap_or(Picked::Nothing);
    f.dirty = true;
    finish(f, store, ts, i, picked);
    true
}

/// What a popover picked, to its owner (closing it unless it stays open).
fn finish(f: &mut FormUi, store: &dyn Store, ts: &mut TextSystem, i: usize, picked: Picked) {
    let id = f.nodes[i].id.clone();
    if picked == Picked::Nothing {
        sync(f, store, ts, i);
        return;
    }
    let Some(pop) = close(&id) else { return };
    match pop.owner.clone() {
        Owner::Code(site) => {
            f.with_cx(store, ts, i, |_, cx| crate::components::codeeditor::helpers::apply(cx, &site, &picked));
        }
        Owner::Inspector(key) => {
            f.with_cx(store, ts, i, |_, cx| crate::components::panels::inspector::apply_picked(cx, &key, &picked));
        }
    }
}

/// The owner told what its popover shows now (a code editor's
/// HelperEditorText, its screen reader's).
fn sync(f: &mut FormUi, store: &dyn Store, ts: &mut TextSystem, i: usize) {
    let id = f.nodes[i].id.clone();
    let said = said(&id);
    if f.nodes[i].type_name == "RCODEEDITOR" {
        f.with_cx(store, ts, i, |_, cx| crate::components::codeeditor::helpers::editor_said(cx, &said));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_popover_keeps_its_value_in_sight() {
        let mut p = Popover::color(Owner::Inspector("color".into()), 255, &[], (100, 100, 60, 20));
        let (w, h) = p.size();
        // (room below)
        p.place((1200, 1400));
        assert_eq!((p.rect.0, p.rect.1), (100, 124));
        // (room above only)
        p.anchor = (100, 900, 60, 20);
        p.place((1200, 1000));
        assert_eq!(p.rect.1, 900 - h - 4);
        // (neither: beside it, its line in sight)
        p.anchor = (100, 400, 60, 20);
        p.place((1200, 800));
        assert_eq!(p.rect.0, 168);
        assert!(p.rect.1 + h <= 800 && p.rect.1 >= 0 && !(p.rect.0 < 160 && p.rect.0 + w > 100));
    }

    /// Every part of a list editor inside it, none over another (a font's
    /// preview, its size and style row, Apply and Cancel; a picture's
    /// preview with its name, with a single picture in the list).
    #[test]
    fn a_list_editor_holds_its_parts() {
        let inside = |r: Rect, w: i64, h: i64| r.0 >= 0 && r.1 >= 0 && r.0 + r.2 <= w && r.1 + r.3 <= h;
        let apart = |a: Rect, b: Rect| a.1 + a.3 <= b.1 || b.1 + b.3 <= a.1 || a.0 + a.2 <= b.0 || b.0 + b.2 <= a.0;
        for (sized, n) in [(false, 1), (true, 1), (false, 30), (true, 30)] {
            let items = (0..n).map(|i| list::Item::new(format!("Font {i}"))).collect();
            let mut p = Popover::list(Owner::Inspector("font".into()), ListKind::Font, items, Some(0), (0, 0, 10, 10));
            if sized {
                if let Editor::List { size, style, .. } = &mut p.editor {
                    *size = Some(size_field(10));
                    *style = Some((false, false));
                }
            }
            let (w, h) = p.size();
            let body = (w, h - TITLE_H);
            let Editor::List { list, .. } = &p.editor else { unreachable!() };
            let lr = list_rect(list, (0, 0, body.0, body.1), ListKind::Font);
            let preview = (PAD, lr.1 + lr.3 + 6, body.0 - 2 * PAD, PREVIEW_H);
            let (apply, cancel) = buttons(body.0, body.1);
            let mut parts = vec![lr, preview, apply, cancel];
            if sized {
                let (sr, br, ir) = font_parts(lr, body.0);
                parts.extend([sr, br, ir]);
            }
            for (i, a) in parts.iter().enumerate() {
                assert!(inside(*a, body.0, body.1), "part {i} {a:?} outside {body:?} (sized {sized}, {n} fonts)");
                for b in &parts[i + 1..] {
                    assert!(apart(*a, *b), "{a:?} over {b:?} (sized {sized}, {n} fonts)");
                }
            }
        }
        let p = Popover::list(Owner::Inspector("picture".into()), ListKind::File(FileKind::Image), vec![list::Item::new("logo.svg")], Some(0), (0, 0, 10, 10));
        let (w, h) = p.size();
        let Editor::List { list, .. } = &p.editor else { unreachable!() };
        let lr = list_rect(list, (0, 0, w, h - TITLE_H), ListKind::File(FileKind::Image));
        // (the preview and its name under it)
        assert!(lr.1 + PICTURE_H + 6 + 20 <= h - TITLE_H, "{h}");
        assert!(lr.0 + lr.2 + 10 + 2 * PAD <= w);
    }
}

/// Field helpers for owners: a size field.
pub fn size_field(pt: i64) -> Field {
    Field::new("Size", &pt.to_string(), Accept::Digits(2))
}
