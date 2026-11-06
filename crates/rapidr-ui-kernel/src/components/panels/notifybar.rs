//! RNOTIFYBAR drawn and driven by the kernel; its model is
//! `rapidr_value::panels::notifybar` (the web draws the same). The rules:
//! docs/design-guidelines.md, "Notifications", "The decision strip".
//!
//! One slim line across the top of what it asks about: the kind's 4-pixel
//! mark (the warning colour's by default), an icon, the question in bold,
//! compact buttons at the right end — under the question when there is no
//! room for both. It stays until a button is pressed. F6 / Tab reach it,
//! Left / Right and Tab move between its buttons, Enter or Space presses
//! one, a button's letter (Alt optional) presses it, Escape leaves it with
//! the question still asked (OnEscape).

use rapidr_value::i18n::tr_ui;
use rapidr_value::ide_theme;
use rapidr_value::input::{Button, Cursor};
use rapidr_value::objects::a11y::{part_id, AccessNode, Action, Role, PART_ITEM};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::{Place, Rect};
use rapidr_value::panels::notifybar::{self as model, Bar as Model, BarGeom, User};
use rapidr_value::panels::rows::vk;
use rapidr_value::panels::User as PanelUser;
use rapidr_value::theme::Look as Style;

use super::common::{self, inside};
use crate::a11y::AccessValue;
use crate::components::{ComponentKind, Cx, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::input::Clipboard;
use crate::paint::{caption, ink_of, Painter};
use crate::store::Store;

pub struct NotifyBar;

fn send(cx: &mut Cx, action: User) {
    super::send(cx, PanelUser::NotifyBar(action));
}

fn model<R>(id: &str, f: impl FnOnce(&Model) -> R) -> R {
    model::with_mut(id, |m| f(m))
}

fn model_mut<R>(id: &str, f: impl FnOnce(&mut Model) -> R) -> R {
    model::with_mut(id, f)
}

/// The question's font (bold) and the buttons'.
fn fonts(font: &Font) -> (Font, Font) {
    (Font { styles: font.styles | 1, ..font.clone() }, font.clone())
}

fn geometry(id: &str, w: i64, font: &Font) -> BarGeom {
    let (bold, plain) = fonts(font);
    model(id, |m| m.layout(w, &bold, &plain))
}

fn button_at(g: &BarGeom, x: i64, y: i64) -> Option<usize> {
    g.buttons.iter().position(|r| inside(*r, x, y))
}

impl ComponentKind for NotifyBar {
    fn name(&self) -> &'static str {
        "RNOTIFYBAR"
    }

    fn focusable(&self, _store: &dyn Store, id: &str) -> bool {
        model::with(id, |m| !m.buttons.is_empty()).unwrap_or(false)
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let t = p.theme();
        let ide = ide_theme::ide(t);
        let l = common::look(t);
        let (w, _) = (cx.width(), cx.height());
        let g = geometry(cx.id, w, &cx.font);
        let (bold, font) = fonts(&cx.font);
        let (kind, buttons, hover, pressed, focus) = model(cx.id, |m| (m.kind, m.buttons.clone(), m.hover, m.pressed, m.focus));
        let status = [ide.info, ide.success, ide.warning, ide.danger][kind.status()];
        let classic = t.look == Style::Classic;
        // (the strip: its own surface, a hairline under it, the mark at its left)
        p.fill((0, 0, w, g.height), ide.card);
        p.fill(g.line, ide.card_border);
        if t.contrast {
            p.fill((0, 0, model::MARK + 1, g.height), ide.card_border);
        } else {
            p.fill(g.mark, status.border);
        }
        p.icon(kind.icon(), g.icon, Some(status.text), false);
        p.text(g.text, &g.shown, &bold, ide.page_text, Place::Left);
        for (i, (cap, r)) in buttons.iter().zip(&g.buttons).enumerate() {
            let on = hover == Some(i);
            let down = pressed == Some(i) && on;
            if t.fluent() {
                let fill = if down { t.control_pressed } else if on { t.control_hot } else { t.control };
                let border = if on { t.border_hot } else { t.border };
                p.round(*r, t.radius, Some(fill), Some(border), 1.0);
                p.raised_rim(*r, border, !down);
                caption(p, *r, cap, &font, ink_of(cx, fill), Place::Center);
            } else {
                p.fill(*r, if on && !down { t.hot } else { t.face });
                if down {
                    p.frame(*r, t.shadow);
                } else {
                    p.button_edge(*r);
                }
                let sh = i64::from(down);
                caption(p, (r.0 + sh, r.1 + sh, r.2, r.3), cap, &font, ink_of(cx, t.face), Place::Center);
            }
            if cx.state.focused && focus == i {
                if classic {
                    p.focus((r.0 + 4, r.1 + 4, r.2 - 8, r.3 - 8));
                } else {
                    p.focus_ring(*r, false);
                }
            }
        }
        let _ = l;
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let g = geometry(cx.id, cx.width(), &cx.font);
        let hit = button_at(&g, m.x.floor() as i64, m.y.floor() as i64);
        let out = MouseOut { press: false, focus: Some(true) };
        match m.kind {
            MouseKind::Move | MouseKind::Leave if !m.captured => {
                let now = if m.kind == MouseKind::Leave { None } else { hit };
                if model_mut(cx.id, |b| std::mem::replace(&mut b.hover, now) != now) {
                    send(cx, User::Redraw);
                }
            }
            MouseKind::Down if m.button == Button::Left => {
                model_mut(cx.id, |b| {
                    b.pressed = hit;
                    if let Some(i) = hit {
                        b.focus = i;
                    }
                });
            }
            MouseKind::Up if m.button == Button::Left => {
                let pressed = model_mut(cx.id, |b| b.pressed.take());
                if let (Some(a), Some(b)) = (pressed, hit) {
                    if a == b && m.inside {
                        send(cx, User::Choose(a));
                    }
                }
            }
            _ => {}
        }
        out
    }

    fn pointer(&self, cx: &mut Cx, x: i64, y: i64) -> Cursor {
        let g = geometry(cx.id, cx.width(), &cx.font);
        if button_at(&g, x, y).is_some() {
            Cursor::Hand
        } else {
            Cursor::Default
        }
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, _clip: &mut dyn Clipboard) -> bool {
        let (n, focus) = model(cx.id, |m| (m.buttons.len(), m.focus));
        if n == 0 {
            return false;
        }
        match k.vk {
            vk::ESCAPE => {
                send(cx, User::Escape);
                true
            }
            vk::LEFT | vk::RIGHT => {
                let next = if k.vk == vk::RIGHT { (focus + 1).min(n - 1) } else { focus.saturating_sub(1) };
                model_mut(cx.id, |m| m.focus = next);
                true
            }
            vk::TAB => {
                let next = if k.mods.shift { focus.checked_sub(1) } else { (focus + 1 < n).then_some(focus + 1) };
                match next {
                    Some(i) => {
                        model_mut(cx.id, |m| m.focus = i);
                        true
                    }
                    // (past the ends: the keyboard goes on)
                    None => false,
                }
            }
            vk::ENTER | vk::SPACE if !k.mods.alt => {
                send(cx, User::Choose(focus.min(n - 1)));
                true
            }
            _ => {
                // (a button's letter, with or without Alt)
                let letter = k.text.chars().next().or_else(|| u8::try_from(k.vk).ok().filter(|b| b.is_ascii_alphanumeric()).map(|b| b as char));
                if k.mods.ctrl || k.mods.command {
                    return false;
                }
                match letter.and_then(|c| model(cx.id, |m| model::by_letter(&m.buttons, c))) {
                    Some(i) => {
                        send(cx, User::Choose(i));
                        true
                    }
                    None => false,
                }
            }
        }
    }

    /// The question whole when it was cut, a button's name.
    fn tip_at(&self, store: &dyn Store, id: &str, x: f64, y: f64) -> Option<String> {
        let font = store.font(id);
        let w = crate::store::int(store, id, "width", 0);
        let g = geometry(id, w, &font);
        let (x, y) = (x.floor() as i64, y.floor() as i64);
        if button_at(&g, x, y).is_some() {
            return None;
        }
        let text = model::with(id, |m| m.text.clone())?;
        (g.shown != text && inside(g.text, x, y)).then_some(text)
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = crate::components::shared_describe(cx, self.name());
        n.role = Role::Group;
        let bar = model(cx.id, Model::clone);
        n.name = model::spoken(&bar);
        let g = geometry(cx.id, cx.width(), &cx.font);
        let (x0, y0) = (cx.rect.0, cx.rect.1);
        for (i, (cap, r)) in bar.buttons.iter().zip(&g.buttons).enumerate() {
            let mut b = AccessNode::new(part_id(cx.id, PART_ITEM, 1 + i), Role::Button);
            b.name = rapidr_value::panels::notify::plain(cap);
            b.bounds = (x0 + r.0, y0 + r.1, r.2, r.3);
            b.actions = vec![Action::Click, Action::Focus];
            b.states.focused = cx.state.focused && bar.focus == i;
            b.shortcut = rapidr_value::panels::notify::mnemonic(cap);
            n.children.push(b);
        }
        n.description = tr_ui("A question: choose one answer.");
        n
    }

    fn access(&self, cx: &mut Cx, action: Action, part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        let Some(i) = part.and_then(|p| p.checked_sub(1)) else { return false };
        if i >= model(cx.id, |m| m.buttons.len()) {
            return false;
        }
        match action {
            Action::Click => {
                send(cx, User::Choose(i));
                true
            }
            Action::Focus => {
                model_mut(cx.id, |m| m.focus = i);
                true
            }
            _ => false,
        }
    }
}

#[allow(dead_code)]
fn unused(_: Rect) {}

/// Tab (Shift+Tab: `back`) with an RNOTIFYBAR focused: the next button; whether
/// it moved inside (`false` past the ends: the focus moves on).
pub(crate) fn tab(f: &crate::FormUi, back: bool) -> bool {
    let Some(i) = f.focus.filter(|&i| f.nodes[i].type_name == "RNOTIFYBAR") else { return false };
    let id = f.nodes[i].id.as_str();
    let (n, focus) = model(id, |m| (m.buttons.len(), m.focus));
    if n == 0 {
        return false;
    }
    let next = if back { focus.checked_sub(1) } else { (focus + 1 < n).then_some(focus + 1) };
    match next {
        Some(k) => {
            model_mut(id, |m| m.focus = k);
            true
        }
        None => false,
    }
}

/// The focus came to node `i` by Tab or F6 (`back`: Shift+Tab): the first
/// button (the last, coming back).
pub(crate) fn entered(f: &crate::FormUi, i: usize, back: bool) {
    if f.nodes[i].type_name != "RNOTIFYBAR" {
        return;
    }
    let id = f.nodes[i].id.as_str();
    let n = model(id, |m| m.buttons.len());
    model_mut(id, |m| m.focus = if back { n.saturating_sub(1) } else { 0 });
}
