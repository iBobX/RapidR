//! QPOPUPMENU's `Popup(X, Y)` where the kernel draws it (Linux, the
//! headless host; macOS and Windows show the system's context menu): its
//! items in a panel over the form — the main menu's panels
//! (`menubar.rs`): the same mouse, keys, submenus, and a pick is
//! [`KernelEvent::MenuPick`](crate::KernelEvent::MenuPick). Alignment puts
//! the panel's left (paLeftAlign 0), right (paRightAlign 1) or middle
//! (paCenterAlign 2) at X. Runtime-core fires OnPopup before it opens.

use rapidr_value::objects::menu;

use super::menubar::{items, panel_size};
use crate::tree::FormUi;

impl FormUi {
    /// Pop-up menu `menu_id` opened at (x, y) of the window's inside
    /// (logical; the in-window bar's included): whether it has items to
    /// show. Any open menu closes first.
    pub fn open_popup(&mut self, menu_id: &str, x: i64, y: i64) -> bool {
        self.close_menus();
        let list = items(menu_id);
        if list.is_empty() {
            return false;
        }
        let (w, _) = panel_size(&list);
        let x = match menu::with(menu_id, |n| n.alignment).unwrap_or(0) {
            1 => x - w,
            2 => x - w / 2,
            _ => x,
        };
        let Some(mut panel) = self.place_panel(&menu_id.to_lowercase(), x, y, None) else { return false };
        // (CONTEXT-MENUS: asked for by the keyboard, its first item chosen)
        if self.menus.key_popup {
            panel.hot = super::menubar::first_item(&list);
        }
        self.menus.panels.push(panel);
        self.menus.popup = Some(menu_id.to_lowercase());
        self.menus.swallow_up = false;
        // (opened while a button is down — from OnMouseDown —: that press's
        // release is not the menu's to act on; the keyboard's never is)
        self.menus.opened_by_press = if self.menus.key_popup { None } else { self.menus.held_press };
        self.dirty = true;
        true
    }

    /// The pop-up menu open now.
    pub fn popup_open(&self) -> Option<&str> {
        self.menus.popup.as_deref().filter(|_| self.menu_open())
    }

    /// Where the pop-up menu open now is: its first panel's rectangle in
    /// the window's inside (logical) — for a test that checks it opened
    /// where the click was.
    pub fn popup_rect(&self) -> Option<crate::Rect> {
        self.popup_open()?;
        self.menus.panels.first().map(|p| p.rect)
    }
}

#[cfg(test)]
mod tests {
    use rapidr_value::input::Button;
    use rapidr_value::{v_int, v_str};

    use crate::{FormUi, KernelEvent, MemStore, Mods, TextSystem};

    #[test]
    fn a_popup_menu_opens_where_asked_and_picks() {
        let mut s = MemStore::new();
        s.add("pmform", "RFORM", None).set("pmform", "clientwidth", v_int(300)).set("pmform", "clientheight", v_int(200));
        s.add("pm", "RPOPUPMENU", Some("pmform"));
        for (id, caption) in [("pm1", "One"), ("pm2", "Two")] {
            s.add(id, "RMENUITEM", Some("pm")).set(id, "caption", v_str(caption)).set(id, "parent", v_str("pm"));
        }
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(&s, "pmform", true);
        f.paint(&s, &mut ts, 1.0);
        assert_eq!(f.menu_offset, 0, "no main menu: no bar");
        assert!(f.open_popup("pm", 50, 60));
        assert_eq!(f.popup_open(), Some("pm"));
        assert_eq!((f.menus.panels[0].rect.0, f.menus.panels[0].rect.1), (50, 60));
        // Its second row picked.
        let y = (60 + super::super::menubar::BORDER + super::super::menubar::ITEM_H + 5) as f64;
        f.mouse_move(&s, &mut ts, 70.0, y, Mods::NONE);
        f.mouse_down(&s, &mut ts, 70.0, y, Button::Left, Mods::NONE);
        f.mouse_up(&s, &mut ts, 70.0, y, Button::Left, Mods::NONE);
        assert_eq!(f.take_events(), vec![KernelEvent::MenuPick("pm2".into())]);
        assert_eq!(f.popup_open(), None);
        // Right-aligned: its right edge at X; kept inside the window.
        s.set("pm", "alignment", v_int(1));
        assert!(f.open_popup("pm", 20, 190));
        let r = f.menus.panels[0].rect;
        assert_eq!(r.0, 0);
        assert!(r.1 + r.3 <= 200);
    }

    /// A form with a label and a pop-up menu of two items.
    fn world() -> (MemStore, FormUi, TextSystem) {
        let mut s = MemStore::new();
        s.add("pmform", "RFORM", None).set("pmform", "clientwidth", v_int(300)).set("pmform", "clientheight", v_int(200));
        s.add("lbl", "RLABEL", Some("pmform")).set("lbl", "left", v_int(20)).set("lbl", "top", v_int(90)).set("lbl", "width", v_int(120)).set("lbl", "height", v_int(30)).set("lbl", "caption", v_str("Press me"));
        s.add("pm", "RPOPUPMENU", Some("pmform"));
        for (id, caption) in [("pm1", "One"), ("pm2", "Two")] {
            s.add(id, "RMENUITEM", Some("pm")).set(id, "caption", v_str(caption)).set(id, "parent", v_str("pm"));
        }
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(&s, "pmform", true);
        f.paint(&s, &mut ts, 1.0);
        (s, f, ts)
    }

    /// A pop-up an OnMouseDown opens (PopupAt at the press) lives through the
    /// release of that press, even a click's wobble over its first item: a
    /// press elsewhere closes it, a click on an item picks it.
    #[test]
    fn the_press_that_opened_a_popup_never_picks_or_closes_it() {
        let (s, mut f, mut ts) = world();
        // (4 px down: past the menu's border, so over its first item, and
        // still a click's wobble)
        assert!(super::super::menubar::BORDER < 4);
        // (pressed on the label, its OnMouseDown opens the menu at the press)
        f.mouse_down(&s, &mut ts, 60.0, 100.0, Button::Left, Mods::NONE);
        assert!(f.open_popup("pm", 60, 100));
        f.take_events();
        // (let go a few pixels off, over the first item: nothing picked, still open)
        f.mouse_up(&s, &mut ts, 62.0, 104.0, Button::Left, Mods::NONE);
        assert_eq!(f.popup_open(), Some("pm"), "the opening click's release keeps it open");
        assert!(!f.take_events().iter().any(|e| matches!(e, KernelEvent::MenuPick(_))));
        // (nothing stays pressed or captured by that press)
        assert!(f.pressed.is_none() && f.capture.is_none());
        // (a click on an item picks it, as ever)
        let y = (100 + super::super::menubar::BORDER + super::super::menubar::ITEM_H + 5) as f64;
        f.mouse_move(&s, &mut ts, 70.0, y, Mods::NONE);
        f.mouse_down(&s, &mut ts, 70.0, y, Button::Left, Mods::NONE);
        f.mouse_up(&s, &mut ts, 70.0, y, Button::Left, Mods::NONE);
        assert_eq!(f.take_events(), vec![KernelEvent::MenuPick("pm2".into())]);
        assert_eq!(f.popup_open(), None);

        // (the same again: a click outside closes it, and nothing else hears it)
        f.mouse_down(&s, &mut ts, 60.0, 100.0, Button::Left, Mods::NONE);
        assert!(f.open_popup("pm", 60, 100));
        f.mouse_up(&s, &mut ts, 60.0, 100.0, Button::Left, Mods::NONE);
        assert_eq!(f.popup_open(), Some("pm"));
        f.take_events();
        f.mouse_down(&s, &mut ts, 250.0, 20.0, Button::Left, Mods::NONE);
        assert_eq!(f.popup_open(), None, "a click outside, after it opened, closes it");
        f.mouse_up(&s, &mut ts, 250.0, 20.0, Button::Left, Mods::NONE);
        assert!(f.take_events().is_empty(), "that click was the menu's: the form under it heard nothing");
    }

    /// Pressed, dragged to an item and let go: a pick, as a system menu does.
    #[test]
    fn a_press_dragged_to_an_item_picks_it() {
        let (s, mut f, mut ts) = world();
        f.mouse_down(&s, &mut ts, 60.0, 100.0, Button::Left, Mods::NONE);
        assert!(f.open_popup("pm", 60, 100));
        f.take_events();
        let y = (100 + super::super::menubar::BORDER + super::super::menubar::ITEM_H + 5) as f64;
        f.mouse_move(&s, &mut ts, 70.0, y, Mods::NONE);
        f.mouse_up(&s, &mut ts, 70.0, y, Button::Left, Mods::NONE);
        assert_eq!(f.take_events(), vec![KernelEvent::MenuPick("pm2".into())]);
        assert_eq!(f.popup_open(), None);
    }

    /// A pop-up OnClick opens — after the click's release — is nobody's
    /// gesture: a release that comes after it (a press that began before the
    /// menu) neither picks nor closes it; a click of its own does.
    #[test]
    fn a_popup_opened_by_a_click_is_not_closed_by_that_click() {
        let (s, mut f, mut ts) = world();
        f.mouse_down(&s, &mut ts, 60.0, 100.0, Button::Left, Mods::NONE);
        f.mouse_up(&s, &mut ts, 60.0, 100.0, Button::Left, Mods::NONE);
        // (OnClick: the menu opens under the button)
        assert!(f.open_popup("pm", 20, 120));
        f.take_events();
        // (a second release of that click — a host's — over the first item)
        f.mouse_up(&s, &mut ts, 24.0, (120 + super::super::menubar::BORDER + 4) as f64, Button::Left, Mods::NONE);
        assert_eq!(f.popup_open(), Some("pm"));
        assert!(f.take_events().is_empty());
        // (a click on the first item picks it)
        let y = (120 + super::super::menubar::BORDER + 4) as f64;
        f.mouse_move(&s, &mut ts, 24.0, y, Mods::NONE);
        f.mouse_down(&s, &mut ts, 24.0, y, Button::Left, Mods::NONE);
        f.mouse_up(&s, &mut ts, 24.0, y, Button::Left, Mods::NONE);
        assert_eq!(f.take_events(), vec![KernelEvent::MenuPick("pm1".into())]);
        // (opened again: a click outside closes it)
        assert!(f.open_popup("pm", 20, 120));
        f.mouse_down(&s, &mut ts, 250.0, 20.0, Button::Left, Mods::NONE);
        assert_eq!(f.popup_open(), None);
    }

    /// Right button too: OnMouseDown(mbRight) opens the menu, the right
    /// release keeps it open.
    #[test]
    fn a_right_press_that_opens_a_popup_keeps_it_through_its_release() {
        let (s, mut f, mut ts) = world();
        f.mouse_down(&s, &mut ts, 60.0, 100.0, Button::Right, Mods::NONE);
        assert!(f.open_popup("pm", 60, 100));
        f.mouse_up(&s, &mut ts, 63.0, 104.0, Button::Right, Mods::NONE);
        assert_eq!(f.popup_open(), Some("pm"));
        assert!(!f.take_events().iter().any(|e| matches!(e, KernelEvent::ContextMenu { .. } | KernelEvent::MenuPick(_))));
    }
}
