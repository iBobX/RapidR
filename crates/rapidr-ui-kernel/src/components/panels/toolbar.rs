//! RTOOLBAR drawn and driven by the kernel; its model is
//! `rapidr_value::panels::toolbar` (the web draws the same).
//!
//! Its own buttons along the strip (`ToolBar::layout`): flat square
//! buttons with RapidR's icons (or the program's pictures), filled under
//! the mouse, pressed and down in the fluent looks; Windows' toolbar
//! buttons in the classic one (a thin raised edge under the mouse, sunken
//! pressed or down, a down one's face dithered); ringed in high contrast.
//! Separators are thin lines (etched in classic). The buttons that don't
//! fit go behind a "»" button whose menu (`Runtime::drop_list`) lists
//! them — and with Customizable a check-marked line per button to show
//! or hide it.
//!
//! Components placed on it start after its buttons ([`client_area`]); a
//! toolbar without buttons is the strip it always was — the form's colour
//! (or its Color), its components laid out as on a panel.
//!
//! It never takes the keyboard (a toolbar isn't a Tab stop, as Windows');
//! screen readers see a group named "Toolbar" (or its Hint) with a button
//! per button (named by its hint, disabled, checked for a toggle) that
//! they can click.

use rapidr_value::objects::a11y::{part_id, AccessNode, Action, Role, PART_ITEM};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::{Op, Place, Rect};
use rapidr_value::objects::text::text_size;
use rapidr_value::panels::toolbar::{self as model, Kind, Layout};
use rapidr_value::panels::User;

use super::common::{self, look, mix, Look};
use crate::a11y::AccessValue;
use crate::components::{ComponentKind, Cx, MouseIn, MouseKind, MouseOut};
use crate::paint::Painter;
use crate::store::{self, Store};

pub struct ToolBar;

/// The "»" button's accessibility part.
const A11Y_CHEVRON: usize = 99_999;

/// How far right the components placed on it reach (their Left + Width):
/// the room they need after its buttons.
fn children_room(store: &dyn Store, id: &str) -> i64 {
    store
        .children(id)
        .iter()
        .filter(|(c, _)| store::flag(store, c, "visible", true))
        .map(|(c, _)| store::int(store, c, "left", 0) + store::int(store, c, "width", 0))
        .max()
        .unwrap_or(0)
}

/// Its buttons laid out for a `w` × `h` strip (none: the strip it was).
pub fn layout(store: &dyn Store, id: &str, w: i64, h: i64) -> Layout {
    let font = store.font(id);
    let room = children_room(store, id);
    model::with(id, |t| t.layout(w, h, room, &font)).unwrap_or_default()
}

/// Its own colour: the program's Color, else the theme's strip.
fn background(cx: &Cx, l: &Look) -> u32 {
    crate::paint::color_of(cx.store, cx.id).unwrap_or(l.chrome)
}

/// A button's face for its state; the ink its icon and caption take
/// (`None`: the icon's own colours).
fn face(p: &mut Painter, r: Rect, l: &Look, back: u32, hover: bool, pressed: bool, down: bool) -> Option<u32> {
    let (x, y, w, h) = r;
    if l.classic {
        if down && !pressed {
            p.op(Op::Checker { rect: r, a: back, b: l.body });
        }
        if pressed || down {
            p.thin_sunken(r);
        } else if hover {
            p.thin_raised(r);
        }
        return None;
    }
    if l.contrast {
        if down {
            p.round(r, 4.0, Some(l.selected), None, 1.0);
        }
        if pressed {
            p.ring(r, 4.0, l.focus, 2.0);
        } else if hover {
            p.ring((x + 1, y + 1, w - 2, h - 2), 4.0, l.link, 1.0);
        }
        return down.then_some(l.selected_text);
    }
    let fill = match (down, pressed, hover) {
        (_, true, _) => Some(mix(l.hover, l.text, 0.12)),
        (true, _, true) => Some(mix(l.selected, l.text, 0.06)),
        (true, ..) => Some(l.selected),
        (false, _, true) => Some(l.hover),
        _ => None,
    };
    if let Some(f) = fill {
        p.round(r, 4.0, Some(f), down.then_some(mix(l.accent, back, 0.45)), 1.0);
    }
    None
}

impl ToolBar {
    fn paint_item(cx: &Cx, p: &mut Painter, l: &Look, back: u32, t: &model::ToolBar, i: usize, r: Rect) {
        let it = &t.items[i];
        let (x, y, w, h) = r;
        if it.kind == Kind::Separator {
            let (top, len) = (y + 4, (h - 8).max(1));
            let mid = x + w / 2;
            if l.classic {
                p.fill((mid - 1, top, 1, len), l.border);
                p.fill((mid, top, 1, len), l.body);
            } else {
                p.fill((mid, top, 1, len), l.line);
            }
            return;
        }
        let enabled = it.enabled && cx.state.enabled;
        let hover = enabled && t.hover == Some(i);
        let pressed = enabled && t.pressed == Some(i) && hover;
        let ink = face(p, r, l, back, hover, pressed, it.kind == Kind::Toggle && it.down);
        let d = i64::from(l.classic && (pressed || it.down));
        let size = t.icon_size();
        let captioned = t.show_captions && !it.caption.is_empty();
        let ix = if captioned { x + (t.button_size - size) / 2 } else { x + (w - size) / 2 };
        let iy = y + (h - size) / 2;
        let drawn = match &it.picture {
            Some(pic) => {
                let px = crate::display::Picture { width: pic.width, height: pic.height, rgba: pic.rgba.clone() };
                p.picture(&format!("{}#tb{}", cx.id, pic.revision), pic.revision, px, (ix + d, iy + d, size, size));
                true
            }
            None => !it.icon.is_empty() && common::icon(p, &it.icon, ix + d, iy + d, size, ink, !enabled),
        };
        // (no icon: its caption, or the first letter of its name)
        let text_ink = if !enabled { l.disabled } else { ink.unwrap_or(l.text) };
        if captioned {
            let tx = ix + size + 4;
            p.text((tx + d, y + d, x + w - tx, h), &it.caption, &cx.font, text_ink, Place::Left);
        } else if !drawn {
            let label = it.label();
            let letter: String = label.chars().take(if it.caption.is_empty() { 1 } else { 3 }).collect();
            p.text((x + d, y + d, w, h), &letter, &Font { styles: 1, ..cx.font.clone() }, text_ink, Place::Center);
        }
    }

    /// The "»" button.
    fn paint_chevron(p: &mut Painter, l: &Look, back: u32, t: &model::ToolBar, r: Rect, enabled: bool) {
        let ink = face(p, r, l, back, enabled && t.chevron_hover, enabled && t.chevron_pressed, false).unwrap_or(if enabled { l.text } else { l.disabled });
        let (x, y, w, h) = r;
        let d = i64::from(l.classic && t.chevron_pressed);
        let (cx, cy) = (x as f64 + w as f64 / 2.0 + d as f64, y as f64 + h as f64 / 2.0 + d as f64);
        // (two small chevrons pointing right: »)
        p.chevron(cx - 2.0, cy, 6.0, false, ink);
        p.chevron(cx + 2.0, cy, 6.0, false, ink);
    }

    /// The click on item `i` (or the "»" button) as the user's: what the
    /// program hears.
    fn press(cx: &mut Cx, l: &Layout, item: Option<usize>, chevron: bool) {
        if chevron {
            let Some(r) = l.chevron else { return };
            let font = cx.font.clone();
            // (the menu: as wide as its longest entry, its right at the button's)
            let entries = model::with_mut(cx.id, |t| {
                t.overflow = l.overflow.clone();
                t.menu_entries(&l.overflow)
            });
            if entries.is_empty() {
                return;
            }
            let wide = entries.iter().map(|e| text_size(&e.0, &font).0).max().unwrap_or(0) + 24;
            let right = cx.rect.0 + r.0 + r.2;
            let x = (right - wide).max(0);
            super::send(cx, User::ToolBar(model::User::Menu((x, cx.rect.1 + r.1, wide, r.3))));
            return;
        }
        super::send(cx, User::ToolBar(model::User::Click(item)));
    }
}

/// The room its buttons take at its left (where its components start).
pub fn client_area(store: &dyn Store, id: &str, w: i64, h: i64) -> Rect {
    let l = layout(store, id, w, h);
    (l.end, 0, (w - l.end).max(0), h)
}

impl ComponentKind for ToolBar {
    fn name(&self) -> &'static str {
        "RTOOLBAR"
    }

    fn focusable(&self, _store: &dyn Store, _id: &str) -> bool {
        false
    }

    fn client_area(&self, store: &dyn Store, id: &str, w: i64, h: i64) -> Rect {
        client_area(store, id, w, h)
    }

    /// (tooltip.rs) The button under the mouse: its hint (its shortcut in
    /// it, as the program wrote it); the "»" button's name.
    fn tip_at(&self, store: &dyn Store, id: &str, x: f64, y: f64) -> Option<String> {
        let (w, h) = (store::int(store, id, "width", 0), store::int(store, id, "height", 0));
        let lay = layout(store, id, w, h);
        let (x, y) = (x.floor() as i64, y.floor() as i64);
        if lay.on_chevron(x, y) {
            return Some("More buttons".into());
        }
        let i = lay.item_at(x, y)?;
        model::with(id, |t| t.items.get(i).filter(|it| it.kind != Kind::Separator).map(|it| it.hint.clone())).flatten().filter(|s| !s.is_empty())
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let l = look(p.theme());
        let back = background(cx, &l);
        let (w, h) = (cx.width(), cx.height());
        p.fill((0, 0, w, h), back);
        let lay = layout(cx.store, cx.id, w, h);
        model::with_mut(cx.id, |t| t.overflow = lay.overflow.clone());
        let Some(t) = model::with(cx.id, model::ToolBar::clone) else { return };
        for &(i, r) in &lay.slots {
            Self::paint_item(cx, p, &l, back, &t, i, r);
        }
        if let Some(r) = lay.chevron {
            Self::paint_chevron(p, &l, back, &t, r, cx.state.enabled);
        }
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let lay = layout(cx.store, cx.id, cx.width(), cx.height());
        let (x, y) = (m.x.floor() as i64, m.y.floor() as i64);
        let slot = lay.item_at(x, y);
        let item = slot.filter(|&i| model::with(cx.id, |t| t.items.get(i).is_some_and(|it| it.enabled && it.kind != Kind::Separator)).unwrap_or(false));
        let chevron = lay.on_chevron(x, y);
        let out = MouseOut { press: false, focus: Some(false) };
        match m.kind {
            MouseKind::Leave => model::with_mut(cx.id, |t| {
                t.hover = None;
                t.chevron_hover = false;
            }),
            MouseKind::Move => model::with_mut(cx.id, |t| {
                t.hover = if m.captured { t.pressed.filter(|&p| Some(p) == item) } else { item };
                t.chevron_hover = chevron && (!m.captured || t.chevron_pressed);
            }),
            MouseKind::Down if m.button == rapidr_value::input::Button::Left => {
                model::with_mut(cx.id, |t| {
                    t.pressed = item;
                    t.hover = item;
                    t.chevron_pressed = chevron;
                });
                // (the "»" menu drops at the press, as a menu button's)
                if chevron {
                    Self::press(cx, &lay, None, true);
                } else if slot.is_none() && m.double() {
                    // (the strip itself double-clicked, as a panel)
                    cx.events.push(crate::input::KernelEvent::DblClick(cx.id.to_string()));
                }
            }
            MouseKind::Up => {
                let (pressed, was_chevron) = model::with_mut(cx.id, |t| {
                    let was = (t.pressed.take(), t.chevron_pressed);
                    t.chevron_pressed = false;
                    was
                });
                if was_chevron {
                    // (its menu dropped at the press)
                } else if pressed.is_some() {
                    if pressed == item && m.inside {
                        Self::press(cx, &lay, item, false);
                    }
                } else if slot.is_none() && !chevron && m.inside && m.captured && !m.double() {
                    // (the strip itself clicked: OnClick, no button)
                    Self::press(cx, &lay, None, false);
                }
            }
            _ => {}
        }
        out
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = crate::components::shared_describe(cx, self.name());
        n.role = Role::Group;
        if n.name.is_empty() {
            n.name = "Toolbar".into();
        }
        let lay = layout(cx.store, cx.id, cx.width(), cx.height());
        let (x0, y0) = (cx.rect.0, cx.rect.1);
        let at = |r: Rect| (x0 + r.0, y0 + r.1, r.2, r.3);
        let Some(t) = model::with(cx.id, model::ToolBar::clone) else { return n };
        // (every button the user can reach: on the bar, or in the "»" menu)
        for (i, it) in t.items.iter().enumerate() {
            if it.kind == Kind::Separator || !it.shown() {
                continue;
            }
            let Some(r) = lay.rect_of(i).or(lay.chevron.filter(|_| lay.overflow.contains(&i))) else { continue };
            let mut b = AccessNode::new(part_id(cx.id, PART_ITEM, i), Role::Button);
            b.name = it.label();
            if !it.command.is_empty() && it.command != it.name {
                b.description = it.command.clone();
            }
            b.states.disabled = !it.enabled || !cx.state.enabled;
            if it.kind == Kind::Toggle {
                b.states.checked = Some(it.down);
            }
            b.actions = vec![Action::Click];
            b.bounds = at(r);
            n.children.push(b);
        }
        if let Some(r) = lay.chevron {
            let mut b = AccessNode::new(part_id(cx.id, PART_ITEM, A11Y_CHEVRON), Role::Button);
            b.name = if t.customizable { "More buttons and customize".into() } else { "More buttons".into() };
            b.states.expanded = Some(false);
            b.actions = vec![Action::Click];
            b.bounds = at(r);
            n.children.push(b);
        }
        n
    }

    fn access(&self, cx: &mut Cx, action: Action, part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        if action != Action::Click || !cx.state.enabled {
            return false;
        }
        let lay = layout(cx.store, cx.id, cx.width(), cx.height());
        match part {
            Some(A11Y_CHEVRON) => Self::press(cx, &lay, None, true),
            Some(i) => {
                let ok = model::with(cx.id, |t| t.items.get(i).is_some_and(|it| it.enabled && it.shown() && it.kind != Kind::Separator)).unwrap_or(false);
                if !ok {
                    return false;
                }
                Self::press(cx, &lay, Some(i), false);
            }
            None => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use rapidr_value::input::Button;
    use rapidr_value::objects::ops::Rect;
    use rapidr_value::panels::toolbar::{self as model, Item, Kind};
    use rapidr_value::panels::User;
    use rapidr_value::v_int;

    use crate::components::form::Container;
    use crate::{FormUi, KernelEvent, MemStore, Mods, TextSystem};

    fn store() -> MemStore {
        let mut s = MemStore::new();
        s.add("tbform", "RFORM", None);
        s.add("tb", "RTOOLBAR", Some("tbform")).set("tb", "width", v_int(300)).set("tb", "height", v_int(32));
        s.add("cb", "RCOOLBTN", Some("tb")).set("cb", "left", v_int(4)).set("cb", "top", v_int(4)).set("cb", "width", v_int(24)).set("cb", "height", v_int(24));
        s
    }

    fn panel_events(f: &mut FormUi) -> Vec<User> {
        f.take_events()
            .into_iter()
            .filter_map(|e| match e {
                KernelEvent::Container(Container::Panel { action, .. }) => Some(action),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn components_sit_after_its_buttons() {
        let s = store();
        let mut ts = TextSystem::new();
        model::remove("tb");
        let mut f = FormUi::build(&s, "tbform", false);
        f.paint(&s, &mut ts, 1.0);
        // (no buttons: the cool button where it always was)
        assert_eq!(f.node("cb").unwrap().abs, (4, 4, 24, 24));
        model::with_mut("tb", |t| {
            for n in ["new", "open"] {
                t.add(Item { name: n.into(), icon: n.into(), hint: n.into(), enabled: true, visible: true, ..Item::default() });
            }
            t.add(Item { name: "wrap".into(), kind: Kind::Toggle, icon: "wrap".into(), hint: "Word wrap".into(), enabled: true, visible: true, ..Item::default() });
        });
        f.sync(&s);
        f.paint(&s, &mut ts, 1.0);
        let end = 2 + 3 * 29 + 1;
        assert_eq!(f.node("cb").unwrap().abs, (end + 4, 4, 24, 24));
        // a click on "open" (second button): the program hears it
        f.mouse_down(&s, &mut ts, 31.0 + 10.0, 15.0, Button::Left, Mods::NONE);
        f.mouse_up(&s, &mut ts, 31.0 + 10.0, 15.0, Button::Left, Mods::NONE);
        assert_eq!(panel_events(&mut f), vec![User::ToolBar(model::User::Click(Some(1)))]);
        // the cool button still gets its own clicks
        f.mouse_down(&s, &mut ts, (end + 10) as f64, 15.0, Button::Left, Mods::NONE);
        f.mouse_up(&s, &mut ts, (end + 10) as f64, 15.0, Button::Left, Mods::NONE);
        assert!(f.take_events().contains(&KernelEvent::Click("cb".into())));
        // screen readers: a button per button, the toggle checkable
        let tree = f.access_tree(&s, &mut ts);
        let bar = tree.children.iter().find(|c| c.name == "Toolbar").expect("the toolbar's group");
        let names: Vec<_> = bar.children.iter().map(|c| (c.name.as_str(), c.states.checked)).collect();
        assert_eq!(&names[..3], &[("new", None), ("open", None), ("Word wrap", Some(false))]);
        model::remove("tb");
    }

    #[test]
    fn the_chevron_drops_the_overflow() {
        let mut s = store();
        s.set("tb", "width", v_int(80));
        let mut ts = TextSystem::new();
        model::remove("tb");
        model::with_mut("tb", |t| {
            for n in ["new", "open", "save", "run"] {
                t.add(Item { name: n.into(), icon: n.into(), hint: format!("{n}!"), enabled: true, visible: true, ..Item::default() });
            }
        });
        let mut f = FormUi::build(&s, "tbform", false);
        f.paint(&s, &mut ts, 1.0);
        let lay = super::layout(&s, "tb", 80, 32);
        let r: Rect = lay.chevron.expect("a chevron");
        f.mouse_down(&s, &mut ts, (r.0 + 4) as f64, 15.0, Button::Left, Mods::NONE);
        f.mouse_up(&s, &mut ts, (r.0 + 4) as f64, 15.0, Button::Left, Mods::NONE);
        let ev = panel_events(&mut f);
        assert!(matches!(ev.as_slice(), [User::ToolBar(model::User::Menu(_))]), "{ev:?}");
        assert_eq!(model::with("tb", |t| t.menu_entries(&t.overflow).len()), Some(lay.overflow.len()));
        model::remove("tb");
    }
}
