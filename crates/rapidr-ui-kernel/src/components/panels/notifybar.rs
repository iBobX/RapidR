//! RNOTIFYBAR drawn and driven by the kernel; its model is
//! `rapidr_value::panels::notifybar` (the web draws the same). The rules:
//! docs/design-guidelines.md, "Notifications", "The decision strip".
//!
//! One slim line across the top of what it asks about: the kind's 4-pixel
//! mark, an icon, the question in regular weight, then compact quiet
//! buttons at the right end — 24 px tall in the small font, each an icon of
//! RapidR's with its words (the main one soft-filled, the others plain),
//! icon only when the room is tight —, a "More" button (⋯) for the rest and
//! a close X. The question is cut with "…" before anything wraps; under it
//! when even that is not enough. It stays until a button is pressed. F6 /
//! Tab reach it, Left / Right and Tab move between its parts, Enter or
//! Space presses one, a button's letter (Alt optional) presses it, Escape
//! leaves it with the question still asked (OnEscape).

use rapidr_value::i18n::tr_ui;
use rapidr_value::ide_theme;
use rapidr_value::input::{Button, Cursor};
use rapidr_value::objects::a11y::{part_id, AccessNode, Action, Role, PART_ITEM};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::Place;
use rapidr_value::panels::notify::plain;
use rapidr_value::panels::notifybar::{self as model, split_spec, Bar as Model, BarGeom, Part, User, ICON, ICON_GAP, PAD};
use rapidr_value::panels::rows::vk;
use rapidr_value::panels::User as PanelUser;
use rapidr_value::theme::Look as Style;

use super::common::inside;
use crate::a11y::AccessValue;
use crate::components::{ComponentKind, Cx, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::input::Clipboard;
use crate::paint::{caption, Painter};
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

/// The question's font (regular) and the buttons' (one step smaller).
fn fonts(font: &Font) -> (Font, Font) {
    (font.clone(), model::small(font))
}

fn geometry(id: &str, w: i64, font: &Font) -> BarGeom {
    let (question, small) = fonts(font);
    model(id, |m| m.layout(w, &question, &small))
}

fn part_at(g: &BarGeom, x: i64, y: i64) -> Option<usize> {
    g.rects().iter().position(|r| inside(*r, x, y))
}

/// What a part is called: its words, for the tooltip and a screen reader.
fn name_of(bar: &Model, part: Part) -> String {
    match part {
        Part::Button(i) => plain(model::caption_of(&bar.buttons[i])),
        Part::More => tr_ui("More"),
        Part::Close => tr_ui("Dismiss"),
    }
}

/// Presses part `i` of the strip at `g` (its menu goes under More's button).
fn activate(cx: &mut Cx, g: &BarGeom, i: usize) {
    let Some(part) = model(cx.id, |m| m.parts().get(i).copied()) else { return };
    match part {
        Part::Button(b) => send(cx, User::Choose(b)),
        Part::More => {
            let r = g.more.unwrap_or_default();
            send(cx, User::More(cx.rect.0 + r.0 + r.2, cx.rect.1 + r.1 + r.3));
        }
        Part::Close => send(cx, User::Close),
    }
}

impl ComponentKind for NotifyBar {
    fn name(&self) -> &'static str {
        "RNOTIFYBAR"
    }

    fn focusable(&self, _store: &dyn Store, id: &str) -> bool {
        model::with(id, |m| !m.parts().is_empty()).unwrap_or(false)
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let t = p.theme();
        let ide = ide_theme::ide(t);
        let w = cx.width();
        let g = geometry(cx.id, w, &cx.font);
        let (question, small) = fonts(&cx.font);
        let bar = model(cx.id, Model::clone);
        let status = [ide.info, ide.success, ide.warning, ide.danger][bar.kind.status()];
        let classic = t.look == Style::Classic;
        // (the strip: its own surface, a hairline under it, the mark at its left)
        p.fill((0, 0, w, g.height), ide.card);
        p.fill(g.line, ide.card_border);
        if t.contrast {
            p.fill((0, 0, model::MARK + 1, g.height), ide.card_border);
        } else {
            p.fill(g.mark, status.border);
        }
        p.icon(bar.kind.icon(), g.icon, Some(status.text), false);
        p.text(g.text, &g.shown, &question, ide.page_text, Place::Left);
        for (i, (part, r)) in bar.parts().into_iter().zip(g.rects()).enumerate() {
            let on = bar.hover == Some(i);
            let down = bar.pressed == Some(i) && on;
            let (icon, words, only, main) = match part {
                Part::Button(b) => {
                    let (icon, words) = split_spec(&bar.buttons[b]);
                    (icon, words.to_string(), g.icon_only[b], bar.primary == b + 1)
                }
                Part::More => ("more", String::new(), true, false),
                Part::Close => ("close-small", String::new(), true, false),
            };
            // (the main button soft-filled, the others plain until the mouse is on them)
            let ink = if main { ide.info.text } else { ide.page_text };
            if classic {
                p.fill(r, if on && !down { t.hot } else { t.face });
                if down {
                    p.frame(r, t.shadow);
                } else {
                    p.button_edge(r);
                }
            } else {
                let fill = if down {
                    Some(t.control_pressed)
                } else if main {
                    Some(ide.info.back)
                } else if on {
                    Some(t.control_hot)
                } else {
                    None
                };
                let border = if t.contrast { Some(t.border) } else if main && on { Some(ide.info.border) } else { None };
                if fill.is_some() || border.is_some() {
                    p.round(r, t.radius, fill, border, 1.0);
                }
            }
            let sh = i64::from(classic && down);
            if only {
                let ir = (r.0 + (r.2 - ICON) / 2 + sh, r.1 + (r.3 - ICON) / 2 + sh, ICON, ICON);
                p.icon(icon, ir, Some(ink), false);
            } else {
                let mut tx = r.0 + PAD + sh;
                if !icon.is_empty() {
                    p.icon(icon, (tx, r.1 + (r.3 - ICON) / 2 + sh, ICON, ICON), Some(ink), false);
                    tx += ICON + ICON_GAP;
                    caption(p, (tx, r.1 + sh, (r.0 + r.2 - tx - PAD).max(0), r.3), &words, &small, ink, Place::Left);
                } else {
                    caption(p, (r.0 + sh, r.1 + sh, r.2, r.3), &words, &small, ink, Place::Center);
                }
            }
            if cx.state.focused && bar.focus == i {
                if classic {
                    p.focus((r.0 + 4, r.1 + 4, (r.2 - 8).max(1), r.3 - 8));
                } else {
                    p.focus_ring(r, false);
                }
            }
        }
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let g = geometry(cx.id, cx.width(), &cx.font);
        let hit = part_at(&g, m.x.floor() as i64, m.y.floor() as i64);
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
                        activate(cx, &g, a);
                    }
                }
            }
            _ => {}
        }
        out
    }

    fn pointer(&self, cx: &mut Cx, x: i64, y: i64) -> Cursor {
        let g = geometry(cx.id, cx.width(), &cx.font);
        if part_at(&g, x, y).is_some() {
            Cursor::Hand
        } else {
            Cursor::Default
        }
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, _clip: &mut dyn Clipboard) -> bool {
        let (n, focus) = model(cx.id, |m| (m.parts().len(), m.focus));
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
                let g = geometry(cx.id, cx.width(), &cx.font);
                activate(cx, &g, focus.min(n - 1));
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

    /// The question whole when it was cut, the name of a button that shows
    /// its icon only (and of More and the close X).
    fn tip_at(&self, store: &dyn Store, id: &str, x: f64, y: f64) -> Option<String> {
        let font = store.font(id);
        let w = crate::store::int(store, id, "width", 0);
        let g = geometry(id, w, &font);
        let (x, y) = (x.floor() as i64, y.floor() as i64);
        let bar = model::with(id, Model::clone)?;
        if let Some(i) = part_at(&g, x, y) {
            let part = *bar.parts().get(i)?;
            let only = match part {
                Part::Button(b) => g.icon_only[b],
                _ => true,
            };
            return only.then(|| name_of(&bar, part));
        }
        (g.shown != bar.text && inside(g.text, x, y)).then_some(bar.text)
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = crate::components::shared_describe(cx, self.name());
        n.role = Role::Group;
        let bar = model(cx.id, Model::clone);
        n.name = model::spoken(&bar);
        let g = geometry(cx.id, cx.width(), &cx.font);
        let (x0, y0) = (cx.rect.0, cx.rect.1);
        for (i, (part, r)) in bar.parts().into_iter().zip(g.rects()).enumerate() {
            let mut b = AccessNode::new(part_id(cx.id, PART_ITEM, 1 + i), Role::Button);
            b.name = name_of(&bar, part);
            b.bounds = (x0 + r.0, y0 + r.1, r.2, r.3);
            b.actions = vec![Action::Click, Action::Focus];
            b.states.focused = cx.state.focused && bar.focus == i;
            match part {
                Part::Button(k) => b.shortcut = rapidr_value::panels::notify::mnemonic(model::caption_of(&bar.buttons[k])),
                Part::More => b.description = tr_ui("Opens a menu with more choices"),
                Part::Close => {}
            }
            n.children.push(b);
        }
        n.description = tr_ui("A question: choose one answer.");
        n
    }

    fn access(&self, cx: &mut Cx, action: Action, part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        let Some(i) = part.and_then(|p| p.checked_sub(1)) else { return false };
        if i >= model(cx.id, |m| m.parts().len()) {
            return false;
        }
        match action {
            Action::Click => {
                let g = geometry(cx.id, cx.width(), &cx.font);
                activate(cx, &g, i);
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

/// Tab (Shift+Tab: `back`) with an RNOTIFYBAR focused: the next part; whether
/// it moved inside (`false` past the ends: the focus moves on).
pub(crate) fn tab(f: &crate::FormUi, back: bool) -> bool {
    let Some(i) = f.focus.filter(|&i| f.nodes[i].type_name == "RNOTIFYBAR") else { return false };
    let id = f.nodes[i].id.as_str();
    let (n, focus) = model(id, |m| (m.parts().len(), m.focus));
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
/// part (the last, coming back).
pub(crate) fn entered(f: &crate::FormUi, i: usize, back: bool) {
    if f.nodes[i].type_name != "RNOTIFYBAR" {
        return;
    }
    let id = f.nodes[i].id.as_str();
    let n = model(id, |m| m.parts().len());
    model_mut(id, |m| m.focus = if back { n.saturating_sub(1) } else { 0 });
}
