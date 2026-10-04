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
        let Some(panel) = self.place_panel(&menu_id.to_lowercase(), x, y, None) else { return false };
        self.menus.panels.push(panel);
        self.menus.popup = Some(menu_id.to_lowercase());
        self.menus.swallow_up = false;
        self.dirty = true;
        true
    }

    /// The pop-up menu open now.
    pub fn popup_open(&self) -> Option<&str> {
        self.menus.popup.as_deref().filter(|_| self.menu_open())
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
}
