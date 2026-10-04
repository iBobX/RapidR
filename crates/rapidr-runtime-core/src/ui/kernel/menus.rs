//! Menus on the kernel host (the buttons and menus lane): a picked item's
//! OnClick, `QPOPUPMENU.Popup(X, Y)` and AutoPopup, and
//! `RAPIDR_DUMP_MENUS`.
//!
//! The kernel draws the in-window bar and its menus and the pop-up menus
//! where the host has no native ones (`rapidr_ui_kernel::components::
//! menubar` / `popupmenu`); the winit host shows macOS' menu bar and
//! macOS' / Windows' context menus (`rapidr_ui_host_winit::menu`). Either
//! way a pick comes back as `KernelEvent::MenuPick` after the pump — menu
//! tracking holds the pump, and no program code runs inside it.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::time::Duration;

use rapidr_value::objects::menu::{self, Kind};

use super::{dispatch_pending, form_shown, invalidate, pump, st, step, with_kern, Kern, RtStore, WinOp};
use crate::object::{form_of, get_children_of, rp_comp_get, rp_fire_event};

thread_local! {
    /// What `RAPIDR_DUMP_MENUS` printed last: the menu model's revision and
    /// the menus shown.
    static DUMPED: RefCell<Option<(u64, BTreeSet<String>)>> = const { RefCell::new(None) };
    /// Pop-up menus without a form of their own (DIMmed) that were shown.
    static LOOSE: RefCell<BTreeSet<String>> = const { RefCell::new(BTreeSet::new()) };
}

/// A menu item picked (from the bar, a menu, a pop-up, its ShortCut): its
/// OnClick; the menus are drawn again from the model after.
pub(super) fn picked(item: &str) {
    rp_fire_event(item, "onclick");
    invalidate();
}

/// The form `name` (a QPOPUPMENU) shows on: its own, else the frontmost.
fn popup_form(name: &str) -> Option<String> {
    form_of(name).filter(|f| form_shown(f)).or_else(|| with_kern(|k| k.desk.stacking().last().cloned()).flatten())
}

/// `PopupMenu.Popup(X, Y)` (screen coordinates): OnPopup, then the menu
/// there; the item picked fires its OnClick before Popup returns. A
/// kernel-drawn menu blocks until it closes on a real screen; the
/// headless host leaves it open (a test's script works it, a capture
/// shows it).
pub(super) fn popup(name: &str, x: i32, y: i32) {
    let name = name.to_lowercase();
    rp_fire_event(&name, "onpopup");
    let Some(form) = popup_form(&name) else { return };
    if form_of(&name).is_none() {
        LOOSE.with(|l| l.borrow_mut().insert(name.clone()));
    }
    // (the screen → the window's inside, its in-window bar included)
    let (left, top) = (rp_comp_get(&form, "left").to_i64(), rp_comp_get(&form, "top").to_i64());
    let (fw, fh) = rapidr_value::layout::form_frame(rp_comp_get(&form, "borderstyle").to_i64());
    open_at(&form, &name, i64::from(x) - left - fw / 2, i64::from(y) - top - (fh - fw / 2));
}

/// Pop-up menu `name` open at (x, y) of `form`'s window's inside.
fn open_at(form: &str, name: &str, x: i64, y: i64) {
    super::ensure_host();
    // (under a test's script nobody can pick from the system's menu, which
    // would hold the pump: the kernel draws it)
    let native = with_kern(|k| k.host.native_menus() && !k.desk.ignore_user).unwrap_or(false);
    if native {
        // (the host's context menu, inside the next pump; its pick comes
        // back as an event)
        st(|s| s.ops.push(WinOp::Popup(form.to_string(), name.to_string(), x, y)));
        pump(Some(Duration::ZERO));
        pump(Some(Duration::ZERO));
        dispatch_pending();
        return;
    }
    let opened = with_kern(|k: &mut Kern| {
        let f = k.desk.forms.get_mut(form)?;
        f.ui.sync(&RtStore);
        Some(f.ui.open_popup(name, x, y))
    })
    .flatten()
    .unwrap_or(false);
    invalidate();
    if !opened || with_kern(|k| k.host.headless()).unwrap_or(true) {
        return;
    }
    let open = || with_kern(|k| k.desk.forms.get(form).is_some_and(|f| f.ui.popup_open().is_some())).unwrap_or(false);
    while open() && form_shown(form) {
        step(None);
    }
}

/// A right press on `comp` at (x, y) in it: the pop-up menu its PopupMenu
/// names, when that menu's AutoPopup is on — instead of OnMouseDown.
pub(super) fn auto_popup(comp: &str, x: i64, y: i64) -> bool {
    let menu_name = rp_comp_get(comp, "popupmenu").to_string_val().to_lowercase();
    if menu_name.is_empty() || !menu::with(&menu_name, |n| n.kind == Kind::Popup && n.auto_popup).unwrap_or(false) {
        return false;
    }
    let Some((form, (ox, oy))) = super::place_of(comp) else { return false };
    rp_fire_event(&menu_name, "onpopup");
    open_at(&form, &menu_name, ox + x, oy + y);
    true
}

/// The menus a built form has (main and pop-up), and the loose pop-ups
/// shown, by name.
fn shown_menus() -> BTreeSet<String> {
    let mut out: BTreeSet<String> = LOOSE.with(|l| l.borrow().clone());
    for form in st(|s| s.built.iter().cloned().collect::<Vec<_>>()) {
        for (id, _) in get_children_of(&form) {
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
pub(super) fn dump_if_changed() {
    if std::env::var_os("RAPIDR_DUMP_MENUS").is_none() {
        return;
    }
    let menus = shown_menus();
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
