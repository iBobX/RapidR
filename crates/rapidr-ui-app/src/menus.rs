//! Menus, host-neutrally (the buttons and menus lane's; moved from
//! runtime-core's `ui/kernel/menus.rs`): a picked item's OnClick,
//! `QPOPUPMENU.Popup(X, Y)` and AutoPopup up to the menu shown — which is
//! the host's ([`Windows::open_popup`]: the system's context menu or the
//! kernel's) — and `RAPIDR_DUMP_MENUS`.
//!
//! The kernel draws the in-window bar and its menus and the pop-up menus
//! where the host has no native ones (`rapidr_ui_kernel::components::
//! menubar` / `popupmenu`). Either way a pick comes back as
//! `KernelEvent::MenuPick`, dispatched after the host's turn.

use std::cell::RefCell;
use std::collections::BTreeSet;

use rapidr_value::objects::menu::{self, Kind};

use crate::windows::invalidate;
use crate::{forms, Program, Windows};

thread_local! {
    /// What `RAPIDR_DUMP_MENUS` printed last: the menu model's revision and
    /// the menus shown.
    static DUMPED: RefCell<Option<(u64, BTreeSet<String>)>> = const { RefCell::new(None) };
    /// Pop-up menus without a form of their own (DIMmed) that were shown.
    static LOOSE: RefCell<BTreeSet<String>> = const { RefCell::new(BTreeSet::new()) };
}

/// A menu item picked (from the bar, a menu, a pop-up, its ShortCut): its
/// OnClick; the menus are drawn again from the model after.
pub fn picked<P: Program>(p: P, item: &str) {
    p.fire(item, "onclick");
    invalidate();
}

/// The form `name` (a QPOPUPMENU) shows on: its own, else the frontmost.
fn popup_form<R: Program + Windows>(rt: R, name: &str) -> Option<String> {
    rt.form_of(name).filter(|f| forms::form_shown(f)).or_else(|| rt.stacking().last().cloned())
}

/// `PopupMenu.Popup(X, Y)` (screen coordinates): OnPopup, then the menu
/// there; the item picked fires its OnClick before Popup returns. A
/// kernel-drawn menu blocks until it closes on a real screen; the
/// headless host leaves it open (a test's script works it, a capture
/// shows it).
pub fn popup<R: Program + Windows>(rt: R, name: &str, x: i32, y: i32) {
    let name = name.to_lowercase();
    rt.fire(&name, "onpopup");
    let Some(form) = popup_form(rt, &name) else { return };
    if rt.form_of(&name).is_none() {
        LOOSE.with(|l| l.borrow_mut().insert(name.clone()));
    }
    // (the screen → the window's inside, its in-window bar included)
    let (left, top) = (rt.get(&form, "left").to_i64(), rt.get(&form, "top").to_i64());
    let (fw, fh) = rapidr_value::layout::form_frame(rt.get(&form, "borderstyle").to_i64());
    rt.open_popup(&form, &name, i64::from(x) - left - fw / 2, i64::from(y) - top - (fh - fw / 2), true);
}

/// A right press on `comp` at (x, y) in it: the pop-up menu its PopupMenu
/// names, when that menu's AutoPopup is on — instead of OnMouseDown.
pub fn auto_popup<R: Program + Windows>(rt: R, comp: &str, x: i64, y: i64) -> bool {
    let menu_name = rt.get(comp, "popupmenu").to_string_val().to_lowercase();
    if menu_name.is_empty() || !menu::with(&menu_name, |n| n.kind == Kind::Popup && n.auto_popup).unwrap_or(false) {
        return false;
    }
    let Some((form, (ox, oy))) = rt.place_of(comp) else { return false };
    rt.fire(&menu_name, "onpopup");
    rt.open_popup(&form, &menu_name, ox + x, oy + y, false);
    true
}

/// The menus a built form has (main and pop-up), and the loose pop-ups
/// shown, by name.
fn shown_menus<P: Program>(p: P) -> BTreeSet<String> {
    let mut out: BTreeSet<String> = LOOSE.with(|l| l.borrow().clone());
    for form in forms::built_forms() {
        for (id, _) in p.children(&form) {
            if matches!(menu::kind(&id), Some(Kind::Main | Kind::Popup)) {
                out.insert(id.to_lowercase());
            }
        }
    }
    out
}

/// A menu's items as the dump spells them: a submenu by its
/// path ("&File", "&File/&Sub"), an item by its menu's path and "/" its
/// caption ("&File/&New", a pop-up's "/One"); `*` checked, `!` disabled.
pub fn dump_line(root: &str) -> String {
    let items: Vec<String> = menu::entries(root)
        .into_iter()
        .map(|e| {
            let mut path = e.path.join("/");
            if e.submenu {
                if !path.is_empty() {
                    path.push('/');
                }
            } else {
                path.push('/');
            }
            path.push_str(&e.caption);
            if e.checked && !e.submenu {
                path.push('*');
            }
            if !e.enabled {
                path.push('!');
            }
            path
        })
        .collect();
    format!("[menu {root}] {}", items.join(" | "))
}

/// `RAPIDR_DUMP_MENUS=1`: the menus shown, on stderr, each time the model
/// (or which menus show) changed.
pub fn dump_if_changed<P: Program>(p: P) {
    if crate::testhooks::var("RAPIDR_DUMP_MENUS").is_none() {
        return;
    }
    let menus = shown_menus(p);
    if menus.is_empty() {
        return;
    }
    let now = (menu::revision(), menus);
    if DUMPED.with(|d| d.borrow().as_ref() == Some(&now)) {
        return;
    }
    for m in &now.1 {
        eprintln!("{}", dump_line(m));
    }
    DUMPED.with(|d| *d.borrow_mut() = Some(now));
}
