//! QMAINMENU / QPOPUPMENU / QMENUITEM (RapidQ manual, Appendix A): the menu
//! tree and its items' state, shared by the desktop and web runtimes, which
//! draw it (the kernel's menu bar / pop-up, the page's menus) again whenever
//! [`revision`] changes.
//!
//! Items hang under a menu or another item by CREATE nesting (`Parent`),
//! `AddItems`, `Insert`, and come off with `DelItems` / `DelIndex`. Each has
//! Caption ("-" is a separator), Checked, RadioItem (checking one unchecks
//! its radio siblings, as Delphi's group 0), Enabled, Visible, ShortCut
//! ("Ctrl+N", "F2", "Shift+Del", …), Hint, Tag, Command (a number of its
//! own), MenuIndex (its place; setting it moves the item) and Count.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use crate::{v_int, v_str, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Main,
    Popup,
    Item,
}

#[derive(Clone, Debug)]
pub struct MenuNode {
    pub kind: Kind,
    pub caption: String,
    pub checked: bool,
    pub enabled: bool,
    pub visible: bool,
    pub radio: bool,
    pub shortcut: String,
    pub hint: String,
    pub command: i64,
    pub auto_popup: bool,
    pub alignment: i64,
    pub parent: Option<String>,
    pub children: Vec<String>,
}

impl MenuNode {
    fn new(kind: Kind, command: i64) -> Self {
        MenuNode {
            kind,
            caption: String::new(),
            checked: false,
            enabled: true,
            visible: true,
            radio: false,
            shortcut: String::new(),
            hint: String::new(),
            command,
            auto_popup: true,
            alignment: 0,
            parent: None,
            children: Vec::new(),
        }
    }
}

thread_local! {
    static MENUS: RefCell<HashMap<String, MenuNode>> = RefCell::new(HashMap::new());
    static REVISION: Cell<u64> = const { Cell::new(0) };
    static NEXT_COMMAND: Cell<i64> = const { Cell::new(100) };
}

fn key(id: &str) -> String {
    id.to_lowercase()
}

fn changed() {
    REVISION.with(|r| r.set(r.get() + 1));
}

/// Goes up by one each time any menu changes (the runtimes then rebuild).
pub fn revision() -> u64 {
    REVISION.with(Cell::get)
}

/// A new menu or menu item; false for other types.
pub fn create(id: &str, type_name: &str) -> bool {
    let kind = match type_name.to_ascii_uppercase().as_str() {
        "RMAINMENU" => Kind::Main,
        "RPOPUPMENU" => Kind::Popup,
        "RMENUITEM" => Kind::Item,
        _ => return false,
    };
    let command = NEXT_COMMAND.with(|c| c.replace(c.get() + 1));
    MENUS.with(|m| {
        m.borrow_mut().entry(key(id)).or_insert_with(|| MenuNode::new(kind, command));
    });
    changed();
    true
}

pub fn is_menu(id: &str) -> bool {
    MENUS.with(|m| m.borrow().contains_key(&key(id)))
}

pub fn kind(id: &str) -> Option<Kind> {
    MENUS.with(|m| m.borrow().get(&key(id)).map(|n| n.kind))
}

pub fn with<R>(id: &str, f: impl FnOnce(&MenuNode) -> R) -> Option<R> {
    MENUS.with(|m| m.borrow().get(&key(id)).map(f))
}

/// The menu (main or pop-up) an item belongs to, or the menu itself.
pub fn root_of(id: &str) -> Option<String> {
    let mut cur = key(id);
    for _ in 0..64 {
        let (kind, parent) = with(&cur, |n| (n.kind, n.parent.clone()))?;
        if kind != Kind::Item {
            return Some(cur);
        }
        cur = parent?;
    }
    None
}

fn detach(m: &mut HashMap<String, MenuNode>, child: &str) {
    if let Some(p) = m.get(child).and_then(|n| n.parent.clone()) {
        if let Some(pn) = m.get_mut(&p) {
            pn.children.retain(|c| c != child);
        }
    }
    if let Some(n) = m.get_mut(child) {
        n.parent = None;
    }
}

/// Puts `child` under `parent` at `index` (the end when None); false if
/// either isn't a menu object, or the child would hold its own parent.
fn attach(parent: &str, child: &str, index: Option<usize>) -> bool {
    let (parent, child) = (key(parent), key(child));
    let ok = MENUS.with(|m| {
        let mut m = m.borrow_mut();
        if !m.contains_key(&parent) || m.get(&child).is_none_or(|n| n.kind != Kind::Item) || parent == child {
            return false;
        }
        // (no cycles: the parent mustn't be inside the child)
        let mut up = Some(parent.clone());
        while let Some(u) = up {
            if u == child {
                return false;
            }
            up = m.get(&u).and_then(|n| n.parent.clone());
        }
        detach(&mut m, &child);
        let pn = m.get_mut(&parent).expect("parent");
        let at = index.unwrap_or(pn.children.len()).min(pn.children.len());
        pn.children.insert(at, child.clone());
        m.get_mut(&child).expect("child").parent = Some(parent);
        true
    });
    if ok {
        changed();
    }
    ok
}

/// A property of a menu object (`None`: not one the model keeps — Parent
/// and the rest stay the runtime's).
pub fn get(id: &str, prop: &str) -> Option<Value> {
    let flag = |b: bool| v_int(b as i64);
    MENUS.with(|m| {
        let m = m.borrow();
        let n = m.get(&key(id))?;
        Some(match prop {
            "caption" => v_str(&n.caption),
            "checked" => flag(n.checked),
            "enabled" => flag(n.enabled),
            "visible" => flag(n.visible),
            "radioitem" => flag(n.radio),
            "shortcut" => v_str(&n.shortcut),
            "hint" => v_str(&n.hint),
            "command" => v_int(n.command),
            "count" | "itemcount" => v_int(n.children.len() as i64),
            "autopopup" => flag(n.auto_popup),
            "alignment" => v_int(n.alignment),
            "menuindex" => {
                let p = n.parent.as_ref()?;
                v_int(m.get(p).and_then(|pn| pn.children.iter().position(|c| *c == key(id))).map_or(-1, |i| i as i64))
            }
            _ => return None,
        })
    })
}

/// Sets a property; `Some(())` when the model keeps it. `Parent` hangs the
/// item under that menu (and is still the runtime's too: `None`).
pub fn set(id: &str, prop: &str, val: &Value) -> Option<()> {
    if !is_menu(id) {
        return None;
    }
    if prop == "parent" {
        let p = val.to_string_val();
        if is_menu(&p) {
            attach(&p, id, None);
        }
        return None;
    }
    if prop == "menuindex" {
        let parent = with(id, |n| n.parent.clone()).flatten()?;
        attach(&parent, id, Some(val.to_i64().max(0) as usize));
        return Some(());
    }
    let on = val.to_bool();
    let k = key(id);
    // (the item's own state; then a checked radio item's siblings go off)
    let radio_on = MENUS.with(|m| {
        let mut m = m.borrow_mut();
        let n = m.get_mut(&k)?;
        match prop {
            "caption" => n.caption = val.to_string_val(),
            "checked" => n.checked = on,
            "enabled" => n.enabled = on,
            "visible" => n.visible = on,
            "radioitem" => n.radio = on,
            "shortcut" => n.shortcut = val.to_string_val(),
            "hint" => n.hint = val.to_string_val(),
            "autopopup" => n.auto_popup = on,
            "alignment" => n.alignment = val.to_i64(),
            _ => return None,
        }
        let exclusive = matches!(prop, "checked" | "radioitem") && n.radio && n.checked;
        Some(exclusive.then(|| n.parent.clone()).flatten())
    })?;
    if let Some(parent) = radio_on {
        MENUS.with(|m| {
            let mut m = m.borrow_mut();
            let siblings = m.get(&parent).map(|pn| pn.children.clone()).unwrap_or_default();
            for s in siblings.iter().filter(|s| **s != k) {
                if let Some(sn) = m.get_mut(s) {
                    if sn.radio {
                        sn.checked = false;
                    }
                }
            }
        });
    }
    changed();
    Some(())
}

/// AddItems / DelItems / DelIndex / Insert; `None` for other methods.
pub fn call(id: &str, method: &str, args: &[Value]) -> Option<Value> {
    if !is_menu(id) {
        return None;
    }
    // (a property read the way a native build calls it: `Menu.Count()`)
    if args.is_empty() {
        if let Some(v) = get(id, method) {
            return Some(v);
        }
    }
    match method {
        "additems" | "add" => {
            for a in args {
                attach(id, &a.to_string_val(), None);
            }
        }
        "insert" => {
            let at = args.first().map_or(0, Value::to_i64).max(0) as usize;
            if let Some(item) = args.get(1) {
                attach(id, &item.to_string_val(), Some(at));
            }
        }
        "delitems" => {
            MENUS.with(|m| {
                let mut m = m.borrow_mut();
                for a in args {
                    let c = key(&a.to_string_val());
                    if m.get(&c).and_then(|n| n.parent.clone()) == Some(key(id)) {
                        detach(&mut m, &c);
                    }
                }
            });
            changed();
        }
        "delindex" => {
            let i = args.first().map_or(-1, Value::to_i64);
            MENUS.with(|m| {
                let mut m = m.borrow_mut();
                let child = m.get(&key(id)).and_then(|n| usize::try_from(i).ok().and_then(|i| n.children.get(i).cloned()));
                if let Some(c) = child {
                    detach(&mut m, &c);
                }
            });
            changed();
        }
        "clear" => {
            let kids = with(id, |n| n.children.clone()).unwrap_or_default();
            MENUS.with(|m| {
                let mut m = m.borrow_mut();
                for c in kids {
                    detach(&mut m, &c);
                }
            });
            changed();
        }
        _ => return None,
    }
    Some(Value::Null)
}

/// One item as a runtime draws it.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub name: String,
    /// The captions of the items above it, outermost first.
    pub path: Vec<String>,
    pub caption: String,
    pub separator: bool,
    pub checked: bool,
    pub radio: bool,
    pub enabled: bool,
    /// Holds visible items (a submenu).
    pub submenu: bool,
    pub shortcut: Option<Shortcut>,
    /// Followed by a separator in the same menu.
    pub divider_after: bool,
}

/// The visible items of a menu, depth first (a submenu's own entry comes
/// before its items; separators are folded into `divider_after`).
pub fn entries(root: &str) -> Vec<Entry> {
    fn walk(m: &HashMap<String, MenuNode>, id: &str, path: &mut Vec<String>, out: &mut Vec<Entry>, depth: usize) {
        let Some(n) = m.get(id) else { return };
        if depth > 16 {
            return;
        }
        let kids: Vec<&String> = n.children.iter().filter(|c| m.get(*c).is_some_and(|k| k.visible)).collect();
        for (i, c) in kids.iter().enumerate() {
            let k = &m[*c];
            if k.caption == "-" {
                if let Some(last) = out.iter_mut().rev().find(|e| e.path == *path) {
                    last.divider_after = true;
                }
                continue;
            }
            let submenu = k.children.iter().any(|g| m.get(g).is_some_and(|x| x.visible));
            out.push(Entry {
                name: (*c).clone(),
                path: path.clone(),
                caption: k.caption.clone(),
                separator: false,
                checked: k.checked,
                radio: k.radio,
                enabled: k.enabled,
                submenu,
                shortcut: parse_shortcut(&k.shortcut),
                divider_after: false,
            });
            let _ = i;
            if submenu {
                path.push(k.caption.clone());
                walk(m, c, path, out, depth + 1);
                path.pop();
            }
        }
    }
    MENUS.with(|m| {
        let m = m.borrow();
        let mut out = Vec::new();
        walk(&m, &key(root), &mut Vec::new(), &mut out, 0);
        out
    })
}

/// The visible items right under `id`, in order (the web draws level by level).
pub fn children(id: &str) -> Vec<String> {
    MENUS.with(|m| {
        let m = m.borrow();
        m.get(&key(id)).map(|n| n.children.iter().filter(|c| m.get(*c).is_some_and(|k| k.visible)).cloned().collect()).unwrap_or_default()
    })
}

/// Every menu item whose ShortCut is this key with these modifiers (enabled
/// and shown, under the given menu).
pub fn item_for_shortcut(root: &str, vk: i64, ctrl: bool, shift: bool, alt: bool) -> Option<String> {
    entries(root).into_iter().find(|e| e.enabled && !e.submenu && e.shortcut == Some(Shortcut { vk, ctrl, shift, alt })).map(|e| e.name)
}

/// A ShortCut: the key's virtual key code and its modifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shortcut {
    pub vk: i64,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Shortcut {
    /// As Windows shows it in a menu ("Ctrl+Shift+N").
    pub fn text(&self) -> String {
        let mut s = String::new();
        if self.ctrl {
            s.push_str("Ctrl+");
        }
        if self.shift {
            s.push_str("Shift+");
        }
        if self.alt {
            s.push_str("Alt+");
        }
        s.push_str(&key_name(self.vk));
        s
    }
}

fn key_name(vk: i64) -> String {
    match vk {
        8 => "BkSp".into(),
        9 => "Tab".into(),
        13 => "Enter".into(),
        27 => "Esc".into(),
        32 => "Space".into(),
        33 => "PgUp".into(),
        34 => "PgDn".into(),
        35 => "End".into(),
        36 => "Home".into(),
        37 => "Left".into(),
        38 => "Up".into(),
        39 => "Right".into(),
        40 => "Down".into(),
        45 => "Ins".into(),
        46 => "Del".into(),
        112..=123 => format!("F{}", vk - 111),
        _ => char::from_u32(vk as u32).map(|c| c.to_string()).unwrap_or_default(),
    }
}

/// "Ctrl+N", "Shift+Ctrl+F5", "Alt+X", "F2", "Del" (Delphi's ShortCut text,
/// case and order free).
pub fn parse_shortcut(text: &str) -> Option<Shortcut> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    let mut sc = Shortcut { vk: 0, ctrl: false, shift: false, alt: false };
    // (the key itself may be "+")
    let (mods, last) = match t.rfind('+') {
        Some(i) if i + 1 < t.len() => (&t[..i], &t[i + 1..]),
        Some(i) if i > 0 => (&t[..i - 1], "+"),
        _ => ("", t),
    };
    for m in mods.split('+').map(str::trim).filter(|m| !m.is_empty()) {
        match m.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => sc.ctrl = true,
            "shift" => sc.shift = true,
            "alt" => sc.alt = true,
            _ => return None,
        }
    }
    let k = last.trim().to_ascii_lowercase();
    sc.vk = match k.as_str() {
        "bksp" | "backspace" => 8,
        "tab" => 9,
        "enter" | "return" => 13,
        "esc" | "escape" => 27,
        "space" => 32,
        "pgup" | "pageup" => 33,
        "pgdn" | "pagedown" => 34,
        "end" => 35,
        "home" => 36,
        "left" => 37,
        "up" => 38,
        "right" => 39,
        "down" => 40,
        "ins" | "insert" => 45,
        "del" | "delete" => 46,
        f if f.len() > 1 && f.starts_with('f') && f[1..].parse::<i64>().is_ok_and(|n| (1..=12).contains(&n)) => 111 + f[1..].parse::<i64>().ok()?,
        c if c.chars().count() == 1 => {
            let ch = c.chars().next()?.to_ascii_uppercase();
            crate::input::vk_of_char(ch)?
        }
        _ => return None,
    };
    Some(sc)
}

/// Forgets every menu (a program was replaced).
pub fn clear() {
    MENUS.with(|m| m.borrow_mut().clear());
    changed();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(name: &str, caption: &str) {
        create(name, "RMENUITEM");
        set(name, "caption", &v_str(caption));
    }

    #[test]
    fn tree_radio_shortcuts_and_indexes() {
        create("mm", "RMAINMENU");
        item("file", "&File");
        item("new", "&New");
        item("sep", "-");
        item("quit", "&Quit");
        set("file", "parent", &v_str("mm"));
        call("file", "additems", &[v_str("new"), v_str("sep"), v_str("quit")]);
        set("new", "shortcut", &v_str("Ctrl+N"));
        set("quit", "shortcut", &v_str("Alt+F4"));
        let e = entries("mm");
        assert_eq!(e.iter().map(|e| e.caption.as_str()).collect::<Vec<_>>(), ["&File", "&New", "&Quit"]);
        assert!(e[0].submenu && e[1].divider_after && !e[2].divider_after);
        assert_eq!(e[1].path, ["&File"]);
        assert_eq!(e[1].shortcut, Some(Shortcut { vk: 78, ctrl: true, shift: false, alt: false }));
        assert_eq!(item_for_shortcut("mm", 115, false, false, true).as_deref(), Some("quit"));
        assert_eq!(get("quit", "menuindex"), Some(v_int(2)));
        set("quit", "menuindex", &v_int(0));
        assert_eq!(get("new", "menuindex"), Some(v_int(1)));
        assert_eq!(get("file", "count"), Some(v_int(3)));
        call("file", "delindex", &[v_int(0)]);
        assert_eq!(get("file", "count"), Some(v_int(2)));

        item("b", "Beginner");
        item("x", "Expert");
        for r in ["b", "x"] {
            set(r, "radioitem", &v_int(1));
            call("file", "additems", &[v_str(r)]);
        }
        set("b", "checked", &v_int(1));
        set("x", "checked", &v_int(1));
        assert_eq!((get("b", "checked"), get("x", "checked")), (Some(v_int(0)), Some(v_int(1))));
        assert_eq!(root_of("x").as_deref(), Some("mm"));
        assert_eq!(parse_shortcut("Shift+Ctrl+F5"), Some(Shortcut { vk: 116, ctrl: true, shift: true, alt: false }));
        assert_eq!(parse_shortcut("Del").map(|s| s.text()).as_deref(), Some("Del"));
    }
}
