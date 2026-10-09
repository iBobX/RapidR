//! RCOMMANDPALETTE drawn and driven by the kernel; its model is
//! `rapidr_value::panels::palette` (the web draws the same).
//!
//! An elevated card at the top of its form, over everything (its z-order:
//! [`stacked`]): a search box with the keyboard in it (an in-place editor,
//! open while the palette shows), and under it the commands found — each
//! with its icon, its category dimmed before its title, the letters typed
//! marked, its shortcut as key caps at the right; the disabled ones
//! dimmed. Up / Down / Page Up / Page Down move the highlight, Enter or a
//! click runs a command (the palette closes first, then OnCommand),
//! Escape or the focus leaving closes it (OnCancel).

use rapidr_value::input::{Button, Cursor};
use rapidr_value::objects::a11y::{part_id, AccessNode, Action, Role, PART_ITEM};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::{Place, Rect};
use rapidr_value::objects::text::text_size;
use rapidr_value::panels::palette::{self as model, Palette as Model, User, BOX, PAD, ROW};
use rapidr_value::panels::rows::vk;
use rapidr_value::panels::User as PanelUser;

use super::common::{self, inside, look, Look};
use crate::a11y::AccessValue;
use crate::components::edit::{self, Source};
use crate::components::form::Container;
use crate::components::list::{self, begin_edit, editing, end_edit, set_edit_text, InPlace};
use crate::components::{ComponentKind, Cx, Ime, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::input::{Clipboard, KernelEvent};
use crate::paint::Painter;
use crate::store::Store;

pub struct Palette;

/// The accessibility parts' indexes: the search box, the list, the
/// commands (1000 + their index in the model).
const PART_SEARCH: usize = 1;
const PART_LIST: usize = 2;
const PART_COMMANDS: usize = 1000;

/// The search box pressed (`NodeUi::part`): the mouse is its editor's
/// until let go.
const BOX_PART: usize = usize::MAX;

fn send(cx: &mut Cx, action: User) {
    super::send(cx, PanelUser::Palette(action));
}

fn model<R>(id: &str, f: impl FnOnce(&Model) -> R) -> R {
    model::with_mut(id, |m| f(m))
}

fn model_mut<R>(id: &str, f: impl FnOnce(&mut Model) -> R) -> R {
    model::with_mut(id, f)
}

/// The search box in a palette `w` wide.
fn search_rect(w: i64) -> Rect {
    (PAD, PAD, (w - 2 * PAD).max(0), BOX)
}

/// The text's own area in the search box (`common::search_box`'s).
fn text_area(sbox: Rect) -> Rect {
    let (x, y, w, h) = sbox;
    let s = (h - 8).clamp(10, 16);
    (x + 10 + s, y + 1, (w - 14 - s).max(0), (h - 2).max(0))
}

/// The in-place editor's box whose text area is `area`.
fn editor_box(area: Rect) -> Rect {
    (area.0 - 3, area.1 - 1, area.2 + 6, area.3 + 2)
}

/// Where the rows start.
const LIST_TOP: i64 = PAD + BOX + PAD / 2;

/// Row `k` shown (from the first shown) in a palette `w` wide.
fn row_rect(w: i64, k: usize) -> Rect {
    (PAD / 2, LIST_TOP + k as i64 * ROW, (w - PAD).max(0), ROW)
}

/// The row (of the matches) at (x, y).
fn row_at(m: &Model, w: i64, x: i64, y: i64, count: usize) -> Option<usize> {
    let shown = count.saturating_sub(m.top).min(m.max_rows.max(1));
    (0..shown).find(|&k| inside(row_rect(w, k), x, y)).map(|k| m.top + k)
}

/// The search box's editor for this showing (a new one at each Show, with
/// the filter as it is).
fn ensure_editor(id: &str) {
    let (shown, shows, filter) = model(id, |m| (m.shown, m.shows as usize, m.filter.clone()));
    if !shown {
        return;
    }
    if editing(id).is_some_and(|e| e.target.0 == shows) {
        return;
    }
    begin_edit(id, InPlace { target: (shows, 0), text: filter, rect: None });
}

/// Closes it from the kernel: the editor goes, then the runtime is told.
fn close(cx: &mut Cx, action: User) {
    end_edit(cx.id);
    cx.ui.edit = None;
    model_mut(cx.id, |m| {
        m.hover = None;
        m.pressed = None;
    });
    send(cx, action);
}

/// Runs the command at row `row` of the matches (if it can run).
fn run(cx: &mut Cx, row: usize) {
    let id = model(cx.id, |m| m.command_at(row).filter(|c| c.enabled).map(|c| c.id.clone()));
    if let Some(id) = id {
        close(cx, User::Run(id));
    }
}

/// A shortcut's key caps: its chords (split at spaces), each chord's keys
/// (split at "+": "Ctrl+Shift+P"; "Ctrl++" is Ctrl and +).
pub fn caps(shortcut: &str) -> Vec<Vec<String>> {
    shortcut
        .split_whitespace()
        .map(|chord| {
            let mut keys: Vec<String> = Vec::new();
            let mut cur = String::new();
            for c in chord.chars() {
                if c == '+' && !cur.is_empty() {
                    keys.push(std::mem::take(&mut cur));
                } else {
                    cur.push(c);
                }
            }
            if !cur.is_empty() {
                keys.push(cur);
            }
            keys
        })
        .filter(|k| !k.is_empty())
        .collect()
}

/// The caps' font: a size smaller.
fn cap_font(font: &Font) -> Font {
    Font { size: (font.size - 1).max(6), ..font.clone() }
}

/// How wide a shortcut's caps are.
fn caps_width(shortcut: &str, font: &Font) -> i64 {
    let f = cap_font(font);
    let chords = caps(shortcut);
    let mut w = 0;
    for (n, chord) in chords.iter().enumerate() {
        if n > 0 {
            w += 8;
        }
        for (k, key) in chord.iter().enumerate() {
            if k > 0 {
                w += 3;
            }
            w += text_size(key, &f).0.max(6) + 10;
        }
    }
    w
}

/// A shortcut's caps, right-aligned at `right`, centred on `mid`.
fn paint_caps(p: &mut Painter, l: &Look, shortcut: &str, (right, mid): (i64, i64), font: &Font, ink: u32, on_selection: bool) {
    let f = cap_font(font);
    let mut x = right - caps_width(shortcut, font);
    let h = 18;
    let y = mid - h / 2;
    for (n, chord) in caps(shortcut).iter().enumerate() {
        if n > 0 {
            x += 8;
        }
        for (k, key) in chord.iter().enumerate() {
            if k > 0 {
                x += 3;
            }
            let w = text_size(key, &f).0.max(6) + 10;
            let r = (x, y, w, h);
            if l.classic {
                p.fill(r, l.chrome);
                p.thin_raised(r);
                p.text(r, key, &f, l.text, Place::Center);
            } else if l.contrast {
                p.round(r, 4.0, None, Some(ink), 1.0);
                p.text(r, key, &f, ink, Place::Center);
            } else {
                let (fill, edge) = if on_selection { (common::mix(l.selected, l.selected_text, 0.12), common::mix(l.selected, l.selected_text, 0.35)) } else { (l.chrome, l.border) };
                p.round(r, 4.0, Some(fill), Some(edge), 1.0);
                // (a cap's lower edge: a key's depth)
                p.fill((x + 3, y + h - 1, w - 6, 1), edge);
                p.text((x, y, w, h - 1), key, &f, if on_selection { ink } else { l.dim }, Place::Center);
            }
            x += w;
        }
    }
}

/// `text` from `x` with its characters at `marks` (counted from `from` in
/// the label) bold in `mark`: where it ends.
fn marked_run(p: &mut Painter, (x, y, h): (i64, i64, i64), limit: i64, text: &str, (marks, from): (&[usize], usize), font: &Font, (color, mark): (u32, u32)) -> i64 {
    let local: Vec<usize> = marks.iter().filter_map(|&m| m.checked_sub(from)).collect();
    let shown = common::elide(text, font, (limit - x).max(0));
    let bold = Font { styles: font.styles | 1, ..font.clone() };
    let chars: Vec<char> = shown.chars().collect();
    let mut at = x;
    let mut i = 0;
    while i < chars.len() {
        let on = local.contains(&i) && chars[i] != '…';
        let mut j = i + 1;
        while j < chars.len() && (local.contains(&j) && chars[j] != '…') == on {
            j += 1;
        }
        let run: String = chars[i..j].iter().collect();
        let f = if on { &bold } else { font };
        let (rw, _) = text_size(&run, f);
        p.text((at, y, (limit - at).max(0), h), &run, f, if on { mark } else { color }, Place::Left);
        at += rw;
        i = j;
    }
    at
}

impl Palette {
    fn search_key(&self, cx: &mut Cx, k: &KeyIn, clip: &mut dyn Clipboard) -> bool {
        let ctrl = k.mods.ctrl || k.mods.command;
        match k.vk {
            vk::UP | vk::DOWN | vk::PAGE_UP | vk::PAGE_DOWN if !k.mods.alt => {
                model_mut(cx.id, |m| m.nav(k.vk));
                true
            }
            vk::ENTER if !ctrl && !k.mods.alt => {
                if let Some(a) = model(cx.id, |m| m.active) {
                    run(cx, a);
                }
                true
            }
            vk::ESCAPE => {
                close(cx, User::Cancel);
                true
            }
            // (the keyboard stays in it: Escape leaves)
            vk::TAB => true,
            _ => {
                let r = editor_box(text_area(search_rect(cx.width())));
                if list::edit_key(cx, k, clip, r).is_none() {
                    return false;
                }
                self.typed(cx);
                true
            }
        }
    }

    /// The editor's text may have changed: the matches follow.
    fn typed(&self, cx: &mut Cx) {
        let Some(ed) = editing(cx.id) else { return };
        let changed = model_mut(cx.id, |m| {
            if m.filter == ed.text {
                return false;
            }
            m.set_filter(&ed.text);
            true
        });
        if changed {
            send(cx, User::Filter(ed.text));
        }
    }
}

impl ComponentKind for Palette {
    fn name(&self) -> &'static str {
        "RCOMMANDPALETTE"
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        ensure_editor(cx.id);
        let l = look(p.theme());
        let (w, h) = (cx.width(), cx.height());
        // (the card)
        if l.classic {
            p.fill((0, 0, w, h), l.chrome);
            p.raised_edge((0, 0, w, h));
        } else if l.contrast {
            p.fill((0, 0, w, h), l.body);
            p.frame((0, 0, w, h), l.text);
            p.frame((1, 1, w - 2, h - 2), l.text);
        } else {
            p.round((0, 0, w, h), 8.0, Some(l.chrome), Some(l.border), 1.0);
        }
        let font = cx.font.clone();
        let (filter, placeholder, max_rows, top, active, hover, has_icons) = model(cx.id, |m| (m.filter.clone(), m.placeholder.clone(), m.max_rows.max(1), m.top, m.active, m.hover, m.commands.iter().any(|c| !c.icon.is_empty())));
        // (the search box, its editor over it)
        let sbox = search_rect(w);
        let typing = editing(cx.id).is_some();
        let empty = if typing { editing(cx.id).is_none_or(|e| e.text.is_empty()) } else { filter.is_empty() };
        common::search_box(p, sbox, &l, if typing { "" } else { &filter }, if empty { &placeholder } else { "" }, cx.state.focused, &font);
        if typing {
            let area = text_area(sbox);
            p.clipped(area, |p| edit::paint_line(cx, p, area, Source::InPlace));
        }
        // (the commands)
        let found = model(cx.id, |m| m.matches());
        if found.is_empty() {
            let r = row_rect(w, 0);
            let msg = if model(cx.id, |m| m.commands.is_empty()) { "No commands" } else { "No matching commands" };
            p.text((r.0 + 12, r.1, r.2 - 24, r.3), msg, &font, l.dim, Place::Left);
            return;
        }
        let cmds = model(cx.id, |m| m.commands.clone());
        let shown = found.len().saturating_sub(top).min(max_rows);
        let scrolls = found.len() > max_rows;
        let right_edge = w - PAD / 2 - if scrolls { 8 } else { 0 };
        for k in 0..shown {
            let row = top + k;
            let f = &found[row];
            let c = &cmds[f.index];
            let (rx, ry, _, rh) = row_rect(w, k);
            let rect = (rx, ry, right_edge - rx, rh);
            let on = active == Some(row);
            let ink = common::row(p, rect, &l, on, cx.state.focused, hover == Some(row) && !on && c.enabled);
            let ink = if c.enabled { ink } else { l.disabled };
            let dim = if on { ink } else if c.enabled { l.dim } else { l.disabled };
            let mark = if on || !c.enabled { ink } else { l.mark };
            let mut x = rx + 10;
            if has_icons {
                if !c.icon.is_empty() {
                    common::icon(p, &c.icon, x, ry + (rh - 16) / 2, 16, if on { Some(ink) } else { None }, !c.enabled);
                }
                x += 24;
            }
            let caps_w = if c.shortcut.is_empty() { 0 } else { caps_width(&c.shortcut, &font) + 16 };
            let limit = right_edge - 8 - caps_w;
            if !c.category.is_empty() {
                x = marked_run(p, (x, ry, rh), limit, &format!("{}: ", c.category), (&f.marks, 0), &font, (dim, mark));
            }
            marked_run(p, (x, ry, rh), limit, &c.title, (&f.marks, c.title_at()), &font, (ink, mark));
            if !c.shortcut.is_empty() {
                paint_caps(p, &l, &c.shortcut, (right_edge - 10, ry + rh / 2), &font, ink, on && !l.classic);
            }
        }
        // (more than fit: where the shown ones are among them)
        if scrolls {
            let track = (w - PAD / 2 - 6, LIST_TOP + 2, 4, shown as i64 * ROW - 4);
            let n = found.len() as i64;
            let th = (track.3 * shown as i64 / n).max(16);
            let ty = track.1 + (track.3 - th) * top as i64 / (n - shown as i64).max(1);
            p.round((track.0, ty, track.2, th), 2.0, Some(if l.contrast { l.text } else { l.border }), None, 1.0);
        }
    }

    /// The card's soft shadow on what it is over (modern and dark).
    fn paint_over(&self, store: &dyn Store, id: &str, w: i64, h: i64, p: &mut Painter) {
        let l = look(p.theme());
        if l.classic || l.contrast || !model::with(id, |m| m.shown).unwrap_or(false) {
            return;
        }
        let under = crate::paint::behind(store, id);
        for k in 1..=4 {
            let c = common::mix(l.shadow, under, 0.55 + 0.11 * k as f64);
            p.ring((-k, -k + 1, w + 2 * k, h + 2 * k), 8.0 + k as f64, c, 1.0);
        }
    }

    /// The search box: the I-beam.
    fn pointer(&self, cx: &mut Cx, x: i64, y: i64) -> Cursor {
        if inside(editor_box(text_area(search_rect(cx.width()))), x, y) {
            Cursor::IBeam
        } else {
            Cursor::Default
        }
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        ensure_editor(cx.id);
        let w = cx.width();
        let (x, y) = (m.x.floor() as i64, m.y.floor() as i64);
        let area = text_area(search_rect(w));
        // (a press in the search box is its editor's, and the drag after it)
        let in_box = match m.kind {
            MouseKind::Down => inside(editor_box(area), x, y),
            MouseKind::Move | MouseKind::Up => m.captured && cx.ui.part == Some(BOX_PART),
            MouseKind::Leave => false,
        };
        if in_box && list::editor_mouse(cx, m, editor_box(area)) {
            cx.ui.part = if m.kind == MouseKind::Up { None } else { Some(BOX_PART) };
            return MouseOut { press: false, focus: Some(true) };
        }
        let count = model(cx.id, |mm| mm.matches().len());
        let row = model(cx.id, |mm| row_at(mm, w, x, y, count));
        match m.kind {
            MouseKind::Move | MouseKind::Leave if !m.captured => {
                let hov = if m.kind == MouseKind::Leave { None } else { row };
                // (the row under the mouse changed: drawn again)
                if model_mut(cx.id, |mm| std::mem::replace(&mut mm.hover, hov) != hov) {
                    send(cx, User::Redraw);
                }
            }
            MouseKind::Down if m.button == Button::Left => {
                if let Some(r) = row {
                    model_mut(cx.id, |mm| {
                        if mm.command_at(r).is_some_and(|c| c.enabled) {
                            mm.active = Some(r);
                            mm.pressed = Some(r);
                        }
                    });
                }
            }
            MouseKind::Up if m.button == Button::Left => {
                let pressed = model_mut(cx.id, |mm| mm.pressed.take());
                if let (Some(p), Some(r)) = (pressed, row) {
                    if p == r && m.inside {
                        run(cx, r);
                    }
                }
            }
            _ => {}
        }
        MouseOut { press: false, focus: Some(true) }
    }

    fn wheel(&self, cx: &mut Cx, _dx: f64, dy: f64, _mods: crate::input::Mods) -> bool {
        let n = list::whole_notches(cx.id, dy);
        if n != 0 {
            model_mut(cx.id, |m| m.scroll(n * 3));
        }
        true
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, clip: &mut dyn Clipboard) -> bool {
        if !model(cx.id, |m| m.shown) {
            return false;
        }
        ensure_editor(cx.id);
        self.search_key(cx, k, clip)
    }

    fn ime(&self, cx: &mut Cx, ime: &Ime) -> bool {
        ensure_editor(cx.id);
        let took = list::editor_ime(cx, ime, editor_box(text_area(search_rect(cx.width()))));
        if took {
            self.typed(cx);
        }
        took
    }

    fn ime_area(&self, cx: &mut Cx) -> Option<Rect> {
        list::editor_ime_area(cx, editor_box(text_area(search_rect(cx.width()))))
    }

    fn wants_ime(&self, _store: &dyn Store, id: &str) -> bool {
        model::with(id, |m| m.shown).unwrap_or(false)
    }

    fn context_menu(&self, cx: &mut Cx) -> Option<crate::components::edit::MenuState> {
        list::editor_menu(cx, editor_box(text_area(search_rect(cx.width()))))
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = crate::components::shared_describe(cx, self.name());
        n.role = Role::Dialog;
        n.name = "Command palette".into();
        let (x0, y0, w) = (cx.rect.0, cx.rect.1, cx.width());
        let (filter, placeholder, top, max_rows, active, found, cmds) = model(cx.id, |m| (editing(cx.id).map(|e| e.text).unwrap_or_else(|| m.filter.clone()), m.placeholder.clone(), m.top, m.max_rows.max(1), m.active, m.matches(), m.commands.clone()));
        let sbox = search_rect(w);
        let mut s = AccessNode::new(part_id(cx.id, PART_ITEM, PART_SEARCH), Role::TextInput);
        s.name = placeholder;
        s.value = Some(filter);
        s.bounds = (x0 + sbox.0, y0 + sbox.1, sbox.2, sbox.3);
        s.actions = vec![Action::Focus, Action::SetValue];
        n.children.push(s);
        let mut list_node = AccessNode::new(part_id(cx.id, PART_ITEM, PART_LIST), Role::ListBox);
        list_node.name = "Commands".into();
        let shown = found.len().saturating_sub(top).min(max_rows);
        list_node.bounds = (x0 + PAD / 2, y0 + LIST_TOP, w - PAD, shown.max(1) as i64 * ROW);
        for (row, f) in found.iter().enumerate() {
            let c = &cmds[f.index];
            let mut o = AccessNode::new(part_id(cx.id, PART_ITEM, PART_COMMANDS + f.index), Role::ListBoxOption);
            o.name = c.label();
            o.description = c.shortcut.clone();
            let (rx, _, rw, rh) = row_rect(w, 0);
            o.bounds = (x0 + rx, y0 + LIST_TOP + (row as i64 - top as i64) * ROW, rw, rh);
            o.states.selected = Some(active == Some(row));
            o.states.disabled = !c.enabled;
            o.states.focused = cx.state.focused && active == Some(row);
            o.actions = if c.enabled { vec![Action::Click, Action::Focus] } else { Vec::new() };
            list_node.children.push(o);
        }
        n.children.push(list_node);
        n
    }

    fn access(&self, cx: &mut Cx, action: Action, part: Option<usize>, value: Option<&AccessValue>) -> bool {
        ensure_editor(cx.id);
        let Some(part) = part else { return false };
        if part == PART_SEARCH {
            match action {
                Action::Focus => {}
                Action::SetValue => {
                    let text = match value {
                        Some(AccessValue::Text(t)) => t.clone(),
                        Some(AccessValue::Number(n)) => n.to_string(),
                        None => String::new(),
                    };
                    set_edit_text(cx.id, &text);
                    self.typed(cx);
                }
                _ => return false,
            }
            return true;
        }
        let Some(index) = part.checked_sub(PART_COMMANDS) else { return false };
        let Some(row) = model(cx.id, |m| m.matches().iter().position(|f| f.index == index)) else { return false };
        match action {
            Action::Click => run(cx, row),
            Action::Focus => model_mut(cx.id, |m| {
                if m.command_at(row).is_some_and(|c| c.enabled) {
                    m.active = Some(row);
                    m.reveal();
                }
            }),
            _ => return false,
        }
        true
    }
}

/// A child's z-order: a palette that shows over its neighbours (Show
/// builds the trees again: its search box's editor opens with it, so the
/// focus leaving it closes it, painted or not).
pub fn stacked(_parent: &str, mut children: Vec<(String, String)>) -> Vec<(String, String)> {
    let open = |(c, t): &(String, String)| t.eq_ignore_ascii_case("RCOMMANDPALETTE") && model::with(c, |m| m.shown).unwrap_or(false);
    let mut moved: Vec<(String, String)> = Vec::new();
    children.retain(|c| {
        if open(c) {
            ensure_editor(&c.0);
            moved.push(c.clone());
            false
        } else {
            true
        }
    });
    children.extend(moved);
    children
}

/// The focus left it while it showed (`super::focus_left`): it closes
/// (OnCancel).
pub fn focus_left(id: &str, _ed: InPlace) -> Vec<KernelEvent> {
    if !model::with(id, |m| m.shown).unwrap_or(false) {
        return Vec::new();
    }
    model::with_mut(id, |m| {
        m.hover = None;
        m.pressed = None;
    });
    vec![KernelEvent::Container(Container::Panel { id: id.to_string(), action: PanelUser::Palette(User::Cancel) })]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcuts_split_into_key_caps() {
        assert_eq!(caps("Ctrl+Shift+P"), vec![vec!["Ctrl", "Shift", "P"]]);
        assert_eq!(caps("Ctrl+K Ctrl+S"), vec![vec!["Ctrl", "K"], vec!["Ctrl", "S"]]);
        assert_eq!(caps("Ctrl++"), vec![vec!["Ctrl", "+"]]);
        assert_eq!(caps("F5"), vec![vec!["F5"]]);
        assert!(caps("").is_empty());
    }

    #[test]
    fn a_shown_palette_goes_over_its_neighbours() {
        let kids = || vec![("pal_z".to_string(), "RCOMMANDPALETTE".to_string()), ("b".to_string(), "RBUTTON".to_string())];
        model::with_mut("pal_z", |m| m.shown = true);
        assert_eq!(stacked("f", kids())[1].0, "pal_z");
        // (its search box's editor opened with it)
        assert!(editing("pal_z").is_some());
        model::with_mut("pal_z", |m| m.shown = false);
        assert_eq!(stacked("f", kids())[0].0, "pal_z");
        end_edit("pal_z");
    }
}
