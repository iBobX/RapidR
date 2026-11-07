//! RNOTIFY drawn and driven by the kernel; its model is
//! `rapidr_value::panels::notify` (the web draws the same). The rules:
//! docs/design-guidelines.md, "Notifications".
//!
//! A transparent overlay: its rectangle is the part of the form it may
//! draw in, and it takes the mouse only where a toast (or the open history)
//! is ([`claims`](ComponentKind::claims)), so a click anywhere else goes to
//! what is under it. Toasts are cards in the theme's look — a 4-pixel mark
//! in the kind's colour, its icon, one or two lines of text, up to two
//! action links, a close button —, stacked from the corner its Position
//! names, sliding in and fading out (never with reduced motion). The time
//! stands still while the mouse is on a toast or the keyboard is; a toast
//! never takes the keyboard by itself. The history is a card of the last
//! messages, each with its time, its whole text and its action again (told
//! why when it can't work any more).

use std::time::Duration;

use rapidr_value::i18n::tr_ui;
use rapidr_value::ide_theme::{self, IdeColors, StatusColors};
use rapidr_value::input::{Button, Cursor};
use rapidr_value::objects::a11y::{part_id, AccessNode, Action, Role, PART_ITEM};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::{Place, Rect};
use rapidr_value::panels::notify::{self as model, plain, Hit, Notify as Model, User, MARK_W};
use rapidr_value::panels::rows::vk;
use rapidr_value::panels::User as PanelUser;
use rapidr_value::theme::{Look as Style, Theme};

use super::common::{self, inside, mix};
use crate::a11y::AccessValue;
use crate::components::{ComponentKind, Cx, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::input::Clipboard;
use crate::paint::Painter;
use crate::store::Store;
use crate::tick::Instant;

pub struct Notify;

fn send(cx: &mut Cx, action: User) {
    super::send(cx, PanelUser::Notify(action));
}

fn model<R>(id: &str, f: impl FnOnce(&Model) -> R) -> R {
    model::with_mut(id, |m| f(m))
}

fn model_mut<R>(id: &str, f: impl FnOnce(&mut Model) -> R) -> R {
    model::with_mut(id, f)
}

// ------------------------------------------------------------------ time --

thread_local! {
    /// The kernel's clock at the first look: the toasts' milliseconds count
    /// from it.
    static EPOCH: Instant = crate::tick::now();
}

/// Milliseconds since the first look.
fn now_ms() -> u64 {
    let epoch = EPOCH.with(|e| *e);
    crate::tick::now().saturating_duration_since(epoch).as_millis() as u64
}

fn instant_at(ms: u64) -> Instant {
    EPOCH.with(|e| *e) + Duration::from_millis(ms)
}

/// Time passes for this component: the model advances, the ones that went
/// are told to the runtime, and the next look is asked for.
fn step(cx: &mut Cx) {
    let now = now_ms();
    let gone = model_mut(cx.id, |m| m.advance(now));
    if !gone.is_empty() {
        send(cx, User::Closed(gone));
    }
    cx.ui.wake = model(cx.id, |m| m.next_wake(now)).map(instant_at);
}

// ----------------------------------------------------------------- looks --

/// What the cards are drawn in, from the theme.
#[derive(Clone, Copy)]
struct Colors {
    classic: bool,
    contrast: bool,
    card: u32,
    border: u32,
    text: u32,
    dim: u32,
    link: u32,
    focus: u32,
    hover: u32,
    kinds: [StatusColors; 4],
}

fn colors(t: &Theme) -> Colors {
    let c: IdeColors = ide_theme::ide(t);
    let l = common::look(t);
    Colors { classic: t.look == Style::Classic, contrast: t.contrast, card: c.card, border: c.card_border, text: c.page_text, dim: c.page_dim, link: c.page_link, focus: l.focus, hover: l.hover, kinds: [c.info, c.success, c.warning, c.danger] }
}

/// The fonts: the text's, the history's time (a size down), the links'.
fn fonts(font: &Font) -> (Font, Font) {
    let small = Font { size: (font.size - 1).max(6), styles: font.styles & !1, ..font.clone() };
    (font.clone(), small)
}

// ------------------------------------------------------------- hit tests --

/// What of the toasts, or of the history, is at (x, y).
fn hit_at(m: &Model, x: i64, y: i64) -> Option<Hit> {
    if m.history_open {
        let g = m.shown_history.as_ref()?;
        if !inside(g.rect, x, y) {
            return None;
        }
        if inside(g.clear, x, y) {
            return Some(Hit::Clear);
        }
        if !inside(g.body, x, y) {
            return None;
        }
        for r in &g.rows {
            for (k, (_, a)) in r.actions.iter().enumerate() {
                if inside(grow(*a, 4), x, y) && m.entry(r.id).is_some_and(|e| e.works(k)) {
                    return Some(Hit::RowAction(r.id, k));
                }
            }
            if inside(r.rect, x, y) {
                return Some(Hit::Row(r.id));
            }
        }
        return None;
    }
    for c in m.shown.iter().rev() {
        if !inside(c.rect, x, y) {
            continue;
        }
        if inside(c.close, x, y) {
            return Some(Hit::Close(c.id));
        }
        for (k, (_, a)) in c.actions.iter().enumerate() {
            if inside(grow(*a, 4), x, y) {
                return Some(Hit::Action(c.id, k));
            }
        }
        return Some(Hit::Card(c.id));
    }
    None
}

fn grow((x, y, w, h): Rect, by: i64) -> Rect {
    (x - by, y - by, w + 2 * by, h + 2 * by)
}

/// Where the tab-ish parts of the stack are, in the order Tab visits them.
fn parts(m: &Model) -> Vec<Hit> {
    let mut out = Vec::new();
    if m.history_open {
        for e in &m.history {
            for k in 0..e.actions.len() {
                if e.works(k) {
                    out.push(Hit::RowAction(e.id, k));
                }
            }
        }
        out.push(Hit::Clear);
        return out;
    }
    // (the newest — nearest the corner — first)
    for t in m.toasts.iter().rev().filter(|t| t.leaving.is_none()) {
        for k in 0..t.actions.len() {
            out.push(Hit::Action(t.id, k));
        }
        out.push(Hit::Close(t.id));
    }
    out
}

/// Tab (Shift+Tab: `back`) with an RNOTIFY focused: the keyboard goes to the
/// next part — a toast's actions, then its close button, the next toast's —,
/// or the history's. Whether it moved inside; `false` past the last (or the
/// first): the focus moves on to the next control.
pub(crate) fn tab(f: &crate::FormUi, back: bool) -> bool {
    let Some(i) = f.focus.filter(|&i| f.nodes[i].type_name == "RNOTIFY") else { return false };
    let id = f.nodes[i].id.as_str();
    let list = model(id, parts);
    if list.is_empty() {
        return false;
    }
    let at = model(id, |m| m.focus).and_then(|c| list.iter().position(|p| *p == c));
    let next = match (at, back) {
        (None, false) => Some(0),
        (None, true) => Some(list.len() - 1),
        (Some(i), false) if i + 1 < list.len() => Some(i + 1),
        (Some(i), true) if i > 0 => Some(i - 1),
        _ => None,
    };
    match next {
        Some(n) => {
            model_mut(id, |m| m.focus = Some(list[n]));
            true
        }
        None => {
            model_mut(id, |m| m.focus = None);
            false
        }
    }
}

/// The focus came to node `i` by Tab or F6 (`back`: Shift+Tab): the keyboard
/// starts at the first part (the last, coming back) unless it is on one.
pub(crate) fn entered(f: &crate::FormUi, i: usize, back: bool) {
    if f.nodes[i].type_name != "RNOTIFY" {
        return;
    }
    let id = f.nodes[i].id.as_str();
    let list = model(id, parts);
    model_mut(id, |m| m.focus = if back { list.last().copied() } else { list.first().copied() });
}

/// The focus came to node `i` some other way (F6, SetFocus): on a first part
/// unless the keyboard is on one already.
pub(crate) fn arrived(f: &crate::FormUi, i: usize) {
    if f.nodes[i].type_name == "RNOTIFY" && model(f.nodes[i].id.as_str(), |m| m.focus).is_none() {
        entered(f, i, false);
    }
}

/// The overlay lost the focus: the keyboard's part goes (the toasts' time runs again).
pub(crate) fn blurred(id: &str) {
    model_mut(id, |m| m.focus = None);
}

/// A toast's whole fade: how far in and how far out it is (0..=1000).
fn alpha(m: &Model, id: i64, now: u64) -> i64 {
    match m.toast(id) {
        Some(t) => (m.entered(t, now)).min(1000 - m.left(t, now)).clamp(0, 1000),
        None => 1000,
    }
}

// ------------------------------------------------------------------ paint --

fn card_fill(p: &mut Painter, c: &Colors, rect: Rect, under: u32, a: f64, mark: u32, shadow: bool) {
    let (x, y, w, h) = rect;
    let f = |col: u32| mix(under, col, a);
    if c.classic {
        p.fill(rect, f(c.card));
        p.raised_edge(rect);
        p.fill((x + 1, y + 1, MARK_W, h - 2), f(mark));
    } else if c.contrast {
        p.fill(rect, c.card);
        p.frame(rect, c.border);
        p.frame((x + 1, y + 1, w - 2, h - 2), c.border);
        p.fill((x + 2, y + 2, MARK_W, h - 4), mark);
    } else {
        if shadow {
            p.elevate(rect, 8.0, 12.0);
        }
        p.round(rect, 8.0, Some(f(c.card)), Some(f(c.border)), 1.0);
        // (the mark: the card's left edge, its corners round)
        p.clipped((x, y, MARK_W, h), |p| p.round((x, y, 12, h), 8.0, Some(f(mark)), None, 1.0));
    }
}

/// A link's action: its words in the link colour, underlined under the
/// mouse, a ring with the keyboard on it.
fn action_link(p: &mut Painter, c: &Colors, rect: Rect, caption: &str, font: &Font, (hover, focus, off): (bool, bool, bool), under: u32, a: f64) {
    let ink = if off { c.dim } else { mix(under, c.link, a) };
    p.text(rect, &plain(caption), font, ink, Place::Left);
    if (hover && !off) || focus {
        p.fill((rect.0, rect.1 + rect.3 - 3, rect.2, 1), ink);
    }
    if focus {
        ring(p, c, grow(rect, 3), 4.0);
    }
}

fn ring(p: &mut Painter, c: &Colors, rect: Rect, radius: f64) {
    if c.classic {
        p.focus(rect);
    } else {
        p.ring(rect, radius, c.focus, if c.contrast { 2.0 } else { 1.5 });
    }
}

fn close_glyph(p: &mut Painter, c: &Colors, rect: Rect, hover: bool, focus: bool, under: u32, a: f64) {
    let (x, y, w, h) = rect;
    if hover && !c.classic {
        p.round(rect, 4.0, Some(c.hover), if c.contrast { Some(c.text) } else { None }, 1.0);
    }
    let (cx, cy) = (x as f64 + w as f64 / 2.0, y as f64 + h as f64 / 2.0);
    let ink = mix(under, if hover { c.text } else { c.dim }, a);
    p.stroke(&[(cx - 4.0, cy - 4.0), (cx + 4.0, cy + 4.0)], ink, 1.5);
    p.stroke(&[(cx - 4.0, cy + 4.0), (cx + 4.0, cy - 4.0)], ink, 1.5);
    if focus {
        ring(p, c, rect, 4.0);
    }
}

impl ComponentKind for Notify {
    fn name(&self) -> &'static str {
        "RNOTIFY"
    }

    /// Reached by Tab or F6 only while toasts, or the history, show.
    fn focusable(&self, _store: &dyn Store, id: &str) -> bool {
        model::with(id, |m| m.visible().next().is_some() || m.history_open).unwrap_or(false)
    }

    /// The mouse is its only where a toast is (anywhere, while the history
    /// is open: a click outside closes it).
    fn claims(&self, id: &str, _w: i64, _h: i64, x: i64, y: i64) -> bool {
        model::with(id, |m| m.history_open || hit_at(m, x, y).is_some()).unwrap_or(false)
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        step(cx);
        let theme = p.theme();
        let c = colors(theme);
        let (w, h) = (cx.width(), cx.height());
        let (font, small) = fonts(&cx.font);
        let now = now_ms();
        let under = crate::paint::behind(cx.store, cx.id);
        let cards = model(cx.id, |m| m.layout(w, h, &font, &font));
        let (bottom, focused) = (model(cx.id, |m| m.position.bottom()), cx.state.focused);
        let (hover, focus) = model(cx.id, |m| (m.hover, if focused { m.focus } else { None }));
        for g in &cards {
            let Some(t) = model(cx.id, |m| m.toast(g.id).cloned()) else { continue };
            let a = model(cx.id, |m| alpha(m, g.id, now));
            let af = a as f64 / 1000.0;
            // (sliding: in from the edge, 12 pixels)
            let enter = model(cx.id, |m| m.toast(g.id).map_or(1000, |t| m.entered(t, now)));
            let dy = (rapidr_value::panels::notify::SLIDE * (1000 - enter) / 1000) * if bottom { 1 } else { -1 };
            let at = |r: Rect| (r.0, r.1 + dy, r.2, r.3);
            let kind = c.kinds[t.kind.status()];
            card_fill(p, &c, at(g.rect), under, af, kind.border, true);
            if af > 0.5 {
                p.icon(t.kind.icon(), at(g.icon), Some(kind.text), false);
            }
            let ink = mix(under, c.text, af);
            let lh = g.text.3 / g.lines.len().max(1) as i64;
            for (i, line) in g.lines.iter().enumerate() {
                p.text((g.text.0, g.text.1 + dy + i as i64 * lh, g.text.2, lh), line, &font, ink, Place::Left);
            }
            for (k, (cap, r)) in g.actions.iter().enumerate() {
                let (on, kf) = (hover == Some(Hit::Action(g.id, k)), focus == Some(Hit::Action(g.id, k)));
                action_link(p, &c, at(*r), cap, &font, (on, kf, false), under, af);
            }
            close_glyph(p, &c, at(g.close), hover == Some(Hit::Close(g.id)), focus == Some(Hit::Close(g.id)), under, af);
            if matches!(focus, Some(Hit::Card(id)) if id == g.id) {
                ring(p, &c, grow(at(g.rect), 2), 10.0);
            }
        }
        model_mut(cx.id, |m| m.shown = cards);
        // (the history)
        let open = model(cx.id, |m| m.history_open);
        if open {
            let g = model(cx.id, |m| m.history_layout(w, h, &font, &font));
            self.paint_history(cx, p, &c, &g, (&font, &small), under);
            model_mut(cx.id, |m| m.shown_history = Some(g));
        } else {
            model_mut(cx.id, |m| m.shown_history = None);
        }
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let (x, y) = (m.x.floor() as i64, m.y.floor() as i64);
        let hit = model(cx.id, |mm| hit_at(mm, x, y));
        let out = MouseOut { press: false, focus: Some(false) };
        match m.kind {
            MouseKind::Move | MouseKind::Leave if !m.captured => {
                let now = if m.kind == MouseKind::Leave { None } else { hit };
                let moved = model_mut(cx.id, |mm| {
                    let before = mm.hover;
                    mm.hover = now;
                    if before != now {
                        // (the mouse left a toast: its time runs again, 2 s at least)
                        if let Some(id) = before.and_then(Hit::toast) {
                            if now.and_then(Hit::toast) != Some(id) {
                                mm.released(id);
                            }
                        }
                    }
                    before != now
                });
                if moved {
                    cx.ui.wake = model(cx.id, |mm| mm.next_wake(now_ms())).map(instant_at);
                    send(cx, User::Redraw);
                }
            }
            MouseKind::Down if m.button == Button::Left => {
                if hit.is_none() && model(cx.id, |mm| mm.history_open) {
                    // (a click outside the history closes it)
                    model_mut(cx.id, |mm| mm.history_open = false);
                    send(cx, User::History);
                    return out;
                }
                model_mut(cx.id, |mm| mm.pressed = hit);
            }
            MouseKind::Up if m.button == Button::Left => {
                let pressed = model_mut(cx.id, |mm| mm.pressed.take());
                if pressed.is_some() && pressed == hit && m.inside {
                    if let Some(h) = hit {
                        activate(cx, h);
                    }
                }
            }
            _ => {}
        }
        out
    }

    fn pointer(&self, cx: &mut Cx, x: i64, y: i64) -> Cursor {
        match model(cx.id, |m| hit_at(m, x, y)) {
            Some(Hit::Action(..) | Hit::Close(_) | Hit::RowAction(..) | Hit::Clear) => Cursor::Hand,
            _ => Cursor::Default,
        }
    }

    fn wheel(&self, cx: &mut Cx, _dx: f64, dy: f64, _mods: crate::input::Mods) -> bool {
        if !model(cx.id, |m| m.history_open) {
            return false;
        }
        model_mut(cx.id, |m| m.history_top = (m.history_top - (dy * 48.0) as i64).max(0));
        true
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, _clip: &mut dyn Clipboard) -> bool {
        if k.mods.ctrl || k.mods.alt || k.mods.command {
            return false;
        }
        let (open, any) = model(cx.id, |m| (m.history_open, m.visible().next().is_some()));
        if !open && !any {
            return false;
        }
        let list = model(cx.id, parts);
        let cur = model(cx.id, |m| m.focus);
        let at = cur.and_then(|c| list.iter().position(|p| *p == c));
        match k.vk {
            vk::ESCAPE => {
                if open {
                    model_mut(cx.id, |m| m.history_open = false);
                    send(cx, User::History);
                } else if let Some(id) = cur.and_then(Hit::toast).or_else(|| list.first().and_then(|h| h.toast())) {
                    let now = now_ms();
                    model_mut(cx.id, |m| m.leave(id, "user", now));
                    cx.ui.wake = Some(instant_at(now + 16));
                }
                true
            }
            vk::TAB => {
                if list.is_empty() {
                    return false;
                }
                let next = match (at, k.mods.shift) {
                    (None, false) => Some(0),
                    (None, true) => Some(list.len() - 1),
                    (Some(i), false) if i + 1 < list.len() => Some(i + 1),
                    (Some(i), true) if i > 0 => Some(i - 1),
                    // (past the last: the keyboard goes on to the next control)
                    _ => None,
                };
                match next {
                    Some(n) => {
                        model_mut(cx.id, |m| m.focus = Some(list[n]));
                        true
                    }
                    None => {
                        model_mut(cx.id, |m| m.focus = None);
                        false
                    }
                }
            }
            vk::UP | vk::DOWN if open => {
                model_mut(cx.id, |m| m.history_top = (m.history_top + if k.vk == vk::DOWN { 48 } else { -48 }).max(0));
                true
            }
            vk::PAGE_UP | vk::PAGE_DOWN if open => {
                model_mut(cx.id, |m| m.history_top = (m.history_top + if k.vk == vk::PAGE_DOWN { 240 } else { -240 }).max(0));
                true
            }
            vk::UP | vk::DOWN => {
                // (between toasts, newest first)
                let ids: Vec<i64> = model(cx.id, |m| m.toasts.iter().rev().filter(|t| t.leaving.is_none()).map(|t| t.id).collect());
                let i = cur.and_then(Hit::toast).and_then(|id| ids.iter().position(|&x| x == id));
                let n = match (i, k.vk == vk::DOWN) {
                    (None, _) => 0,
                    (Some(i), true) => (i + 1).min(ids.len().saturating_sub(1)),
                    (Some(i), false) => i.saturating_sub(1),
                };
                if let Some(&id) = ids.get(n) {
                    model_mut(cx.id, |m| m.focus = Some(Hit::Card(id)));
                }
                true
            }
            vk::ENTER | vk::SPACE => {
                match cur {
                    Some(Hit::Card(id)) => activate(cx, Hit::Action(id, 0)),
                    Some(h) => activate(cx, h),
                    None => {}
                }
                true
            }
            _ => false,
        }
    }

    fn tick(&self, cx: &mut Cx) {
        step(cx);
    }

    /// A toast's whole text when it was cut, an action's reason when it can't
    /// work, the close button's name.
    fn tip_at(&self, _store: &dyn Store, id: &str, x: f64, y: f64) -> Option<String> {
        let (x, y) = (x.floor() as i64, y.floor() as i64);
        model::with(id, |m| match hit_at(m, x, y)? {
            Hit::Close(_) => Some(tr_ui("Close")),
            Hit::Card(t) => {
                let t = m.toast(t)?;
                let g = m.shown.iter().find(|g| g.id == t.id)?;
                (g.lines.iter().any(|l| l.contains('…'))).then(|| t.text.clone())
            }
            Hit::RowAction(e, k) => m.entry(e)?.retired.get(k).cloned().flatten(),
            Hit::Row(e) => {
                let e = m.entry(e)?;
                // (an action greyed: why)
                let g = m.shown_history.as_ref()?;
                let r = g.rows.iter().find(|r| r.id == e.id)?;
                r.actions.iter().enumerate().find(|(_, (_, a))| inside(grow(*a, 4), x, y)).and_then(|(k, _)| e.retired.get(k).cloned().flatten())
            }
            _ => None,
        })
        .flatten()
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = crate::components::shared_describe(cx, self.name());
        n.name = tr_ui("Notifications");
        n.role = Role::Group;
        let (x0, y0) = (cx.rect.0, cx.rect.1);
        let (cards, history, open, focus, unseen) = model(cx.id, |m| (m.shown.clone(), m.shown_history.clone(), m.history_open, m.focus, m.unseen()));
        let toasts = model(cx.id, |m| m.toasts.clone());
        for g in &cards {
            let Some(t) = toasts.iter().find(|t| t.id == g.id) else { continue };
            let mut card = AccessNode::new(part_id(cx.id, PART_ITEM, 1000 + g.id as usize * 8), Role::Group);
            card.name = model::spoken(&t.text, &t.actions);
            card.description = match t.kind {
                model::Kind::Info => tr_ui("Information"),
                model::Kind::Success => tr_ui("Done"),
                model::Kind::Warning => tr_ui("Warning"),
                model::Kind::Error => tr_ui("Error"),
            };
            card.bounds = (x0 + g.rect.0, y0 + g.rect.1, g.rect.2, g.rect.3);
            card.states.focused = cx.state.focused && matches!(focus, Some(Hit::Card(id)) if id == g.id);
            for (k, (cap, r)) in g.actions.iter().enumerate() {
                let mut b = AccessNode::new(part_id(cx.id, PART_ITEM, 1000 + g.id as usize * 8 + 1 + k), Role::Button);
                b.name = plain(cap);
                b.bounds = (x0 + r.0, y0 + r.1, r.2, r.3);
                b.actions = vec![Action::Click, Action::Focus];
                b.states.focused = cx.state.focused && focus == Some(Hit::Action(g.id, k));
                card.children.push(b);
            }
            let mut close = AccessNode::new(part_id(cx.id, PART_ITEM, 1000 + g.id as usize * 8 + 3), Role::Button);
            close.name = tr_ui("Close");
            close.bounds = (x0 + g.close.0, y0 + g.close.1, g.close.2, g.close.3);
            close.actions = vec![Action::Click, Action::Focus];
            close.states.focused = cx.state.focused && focus == Some(Hit::Close(g.id));
            card.children.push(close);
            card.actions = vec![Action::Focus];
            n.children.push(card);
        }
        if let (true, Some(g)) = (open, history) {
            let mut list = AccessNode::new(part_id(cx.id, PART_ITEM, 99_998), Role::List);
            list.name = tr_ui("Notification history");
            list.bounds = (x0 + g.rect.0, y0 + g.rect.1, g.rect.2, g.rect.3);
            for r in &g.rows {
                let Some(e) = model(cx.id, |m| m.entry(r.id).cloned()) else { continue };
                let mut item = AccessNode::new(part_id(cx.id, PART_ITEM, 100_000 + r.id as usize * 8), Role::ListItem);
                item.name = format!("{} {}", e.clock, e.text);
                item.bounds = (x0 + r.rect.0, y0 + r.rect.1, r.rect.2, r.rect.3);
                for (k, (cap, a)) in r.actions.iter().enumerate() {
                    let mut b = AccessNode::new(part_id(cx.id, PART_ITEM, 100_000 + r.id as usize * 8 + 1 + k), Role::Button);
                    b.name = plain(cap);
                    b.bounds = (x0 + a.0, y0 + a.1, a.2, a.3);
                    if let Some(why) = e.retired.get(k).cloned().flatten() {
                        b.states.disabled = true;
                        b.description = why;
                    } else {
                        b.actions = vec![Action::Click, Action::Focus];
                    }
                    item.children.push(b);
                }
                list.children.push(item);
            }
            let mut clear = AccessNode::new(part_id(cx.id, PART_ITEM, 99_999), Role::Button);
            clear.name = tr_ui("Clear");
            clear.bounds = (x0 + g.clear.0, y0 + g.clear.1, g.clear.2, g.clear.3);
            clear.actions = vec![Action::Click, Action::Focus];
            list.children.push(clear);
            n.children.push(list);
        }
        // (the node's own place: the toasts' and the history's together)
        let all: Vec<Rect> = n.children.iter().map(|c| c.bounds).collect();
        n.bounds = match all.split_first() {
            Some((f, rest)) => rest.iter().fold(*f, |a, b| {
                let (x1, y1) = (a.0.min(b.0), a.1.min(b.1));
                let (x2, y2) = ((a.0 + a.2).max(b.0 + b.2), (a.1 + a.3).max(b.1 + b.3));
                (x1, y1, x2 - x1, y2 - y1)
            }),
            None => (x0, y0, 0, 0),
        };
        if unseen > 0 {
            n.description = format!("{unseen}");
        }
        n
    }

    fn access(&self, cx: &mut Cx, action: Action, part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        let Some(part) = part else { return false };
        let hit = model(cx.id, |m| hit_of_part(m, part));
        let Some(hit) = hit else { return false };
        match action {
            Action::Click => {
                activate(cx, hit);
                true
            }
            Action::Focus => {
                model_mut(cx.id, |m| m.focus = Some(hit));
                true
            }
            _ => false,
        }
    }
}

/// The part a screen reader's part index names.
fn hit_of_part(m: &Model, part: usize) -> Option<Hit> {
    if part == 99_999 {
        return Some(Hit::Clear);
    }
    if part >= 100_000 {
        let (id, sub) = (((part - 100_000) / 8) as i64, (part - 100_000) % 8);
        return match sub {
            0 => m.entry(id).map(|_| Hit::Row(id)),
            k => m.entry(id).filter(|e| e.works(k - 1)).map(|_| Hit::RowAction(id, k - 1)),
        };
    }
    if part >= 1000 {
        let (id, sub) = (((part - 1000) / 8) as i64, (part - 1000) % 8);
        m.toast(id)?;
        return Some(match sub {
            0 => Hit::Card(id),
            3 => Hit::Close(id),
            k => Hit::Action(id, k - 1),
        });
    }
    None
}

/// What a press (a click, Enter, a screen reader's click) on a part does.
fn activate(cx: &mut Cx, hit: Hit) {
    match hit {
        Hit::Action(id, k) => {
            let key = model(cx.id, |m| m.toast(id).map(|t| t.key.clone())).unwrap_or_default();
            send(cx, User::Action { id, key, index: k });
        }
        Hit::RowAction(id, k) => {
            let key = model(cx.id, |m| m.entry(id).map(|e| e.key.clone())).unwrap_or_default();
            send(cx, User::Action { id, key, index: k });
        }
        Hit::Close(id) => {
            let now = now_ms();
            model_mut(cx.id, |m| m.leave(id, "user", now));
            cx.ui.wake = Some(instant_at(now + 16));
            send(cx, User::Redraw);
        }
        Hit::Clear => {
            model_mut(cx.id, |m| {
                m.clear_history();
                m.history_open = false;
            });
            send(cx, User::History);
        }
        Hit::Card(_) | Hit::Row(_) => {}
    }
}

impl Notify {
    /// The history card: a header with its title and Clear, the rows (their
    /// time, their whole text, their action again), a scroll bar when they
    /// run past the card.
    fn paint_history(&self, cx: &mut Cx, p: &mut Painter, c: &Colors, g: &model::HistoryGeom, (font, small): (&Font, &Font), under: u32) {
        card_fill_plain(p, c, g.rect);
        let (hover, focus) = model(cx.id, |m| (m.hover, if cx.state.focused { m.focus } else { None }));
        let bold = Font { styles: font.styles | 1, ..font.clone() };
        p.text((g.head.0 + 16, g.head.1, g.head.2 - 100, g.head.3), &tr_ui("Notifications"), &bold, c.text, Place::Left);
        action_link(p, c, g.clear, &tr_ui("Clear"), font, (hover == Some(Hit::Clear), focus == Some(Hit::Clear), false), under, 1.0);
        p.fill((g.rect.0 + 1, g.head.1 + g.head.3 - 1, g.rect.2 - 2, 1), c.border);
        let entries = model(cx.id, |m| m.history.clone());
        p.clipped(g.body, |p| {
            if g.rows.is_empty() {
                p.text((g.body.0 + 16, g.body.1, g.body.2 - 32, g.body.3), &tr_ui("Nothing yet."), font, c.dim, Place::Left);
            }
            for (i, r) in g.rows.iter().enumerate() {
                let Some(e) = entries.iter().find(|e| e.id == r.id) else { continue };
                if hover == Some(Hit::Row(r.id)) && !c.classic {
                    p.round((r.rect.0 + 4, r.rect.1 + 1, r.rect.2 - 8, r.rect.3 - 2), 6.0, Some(c.hover), None, 1.0);
                }
                if i > 0 {
                    p.fill((r.rect.0 + 16, r.rect.1, r.rect.2 - 32, 1), mix(c.card, c.border, 0.6));
                }
                let kind = c.kinds[e.kind.status()];
                p.icon(e.kind.icon(), r.icon, Some(kind.text), false);
                // (unseen: a dot after the time)
                p.text(r.time, &e.clock, small, c.dim, Place::Left);
                if !e.seen {
                    let tw = rapidr_value::objects::text::text_size(&e.clock, small).0;
                    p.round((r.time.0 + tw + 6, r.time.1 + 6, 6, 6), 3.0, Some(kind.border), None, 1.0);
                }
                for (j, line) in r.lines.iter().enumerate() {
                    p.text((r.text.0, r.text.1 + j as i64 * model::LINE, r.text.2, model::LINE), line, font, c.text, Place::Left);
                }
                for (k, (cap, rect)) in r.actions.iter().enumerate() {
                    let works = e.works(k);
                    action_link(p, c, *rect, cap, font, (hover == Some(Hit::RowAction(r.id, k)), focus == Some(Hit::RowAction(r.id, k)), !works), under, 1.0);
                }
            }
        });
        // (more rows than fit: where the shown ones are)
        if g.content > g.body.3 {
            let n = g.content;
            let th = (g.body.3 * g.body.3 / n).max(20);
            let top = model(cx.id, |m| m.history_top.clamp(0, n - g.body.3));
            let ty = g.body.1 + (g.body.3 - th) * top / (n - g.body.3).max(1);
            p.round((g.body.0 + g.body.2 - 8, ty + 2, 4, th - 4), 2.0, Some(if c.contrast { c.text } else { c.border }), None, 1.0);
        }
    }
}

/// The history card's own surface (no mark).
fn card_fill_plain(p: &mut Painter, c: &Colors, rect: Rect) {
    let (x, y, w, h) = rect;
    if c.classic {
        p.fill(rect, c.card);
        p.raised_edge(rect);
    } else if c.contrast {
        p.fill(rect, c.card);
        p.frame(rect, c.border);
        p.frame((x + 1, y + 1, w - 2, h - 2), c.border);
    } else {
        p.elevate(rect, 8.0, 16.0);
        p.round(rect, 8.0, Some(c.card), Some(c.border), 1.0);
    }
}

/// A notification overlay over its neighbours (the last drawn, the first to
/// see the mouse where it draws): RNOTIFYs go to the end of their parent's
/// children.
pub fn stacked(_parent: &str, mut children: Vec<(String, String)>) -> Vec<(String, String)> {
    let mut moved: Vec<(String, String)> = Vec::new();
    children.retain(|c| {
        if c.1.eq_ignore_ascii_case("RNOTIFY") {
            moved.push(c.clone());
            false
        } else {
            true
        }
    });
    children.extend(moved);
    children
}
