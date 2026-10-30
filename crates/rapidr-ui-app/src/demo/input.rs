//! The demo's input, through the kernel's own routing as the user's is
//! (`Desktop::mouse_down` …: hit test, focus, the models, the program's
//! events) — what RAppHands plays (`crate::hands`), never the system's
//! mouse or keyboard. Targets are found as RAppHands finds them (a name,
//! a caption, the label beside it), and inside a component by what its
//! items show (the accessibility tree: a toolbox's rows, a tree's items,
//! the Properties pane's names).

use rapidr_ui_kernel::{Mods, Store};
use rapidr_value::input::Button;

use super::record;
use super::script::Target;
use crate::desktop::{Desktop, Source};
use crate::hands;

/// A target found: its window, its rectangle on the screen and the point
/// to act at (logical pixels, the screen's).
#[derive(Clone, Debug, PartialEq)]
pub struct Spot {
    pub form: String,
    pub comp: Option<String>,
    pub rect: (f64, f64, f64, f64),
    pub at: (f64, f64),
}

fn clean(s: &str) -> String {
    s.replace('&', "").trim().to_lowercase()
}

/// `t` found in this process's windows (not an `app:` one).
pub fn locate(desk: &mut Desktop, store: &dyn Store, t: &Target) -> Result<Spot, String> {
    if let Some((x, y)) = t.at {
        let form = crate::eyes::find_window(desk, t.window.as_deref().unwrap_or(""))?;
        let at = record::to_screen(desk, store, &form, (x, y)).ok_or("the window has gone")?;
        return Ok(Spot { form, comp: None, rect: (at.0, at.1, 0.0, 0.0), at });
    }
    let mut ht = hands::Target::parse(&t.what);
    if let Some(w) = &t.window {
        ht.window = Some(w.clone());
    }
    let f = hands::find(desk, store, &ht)?;
    let ui = &desk.forms[&f.form].ui;
    // (the component's rectangle in the window's inside, the menu bar's
    // included — what Found's point is in)
    let mut rect = match f.comp.as_deref().and_then(|c| ui.node(c)) {
        Some(n) => (n.abs.0 as f64, n.abs.1 as f64, n.abs.2 as f64, n.abs.3 as f64),
        // (a window: all of its inside, its menu bar and open menus too)
        None => (0.0, 0.0, ui.client.0 as f64, (ui.client.1 + ui.menu_offset) as f64),
    };
    if let Some(item) = &t.item {
        rect = match find_item(desk, store, &f.form, rect, item) {
            Ok(r) => r,
            Err(names) => return Err(format!("Nothing in {} shows \"{item}\" now (scroll it into view, or open what holds it). It shows: {}.", f.what, if names.is_empty() { "nothing with a name".into() } else { names.join(", ") })),
        };
    }
    let origin = record::to_screen(desk, store, &f.form, (0.0, 0.0)).ok_or("the window has gone")?;
    let rect = (rect.0 + origin.0, rect.1 + origin.1, rect.2, rect.3);
    let at = (rect.0 + rect.2 * t.fx, rect.1 + rect.3 * t.fy);
    Ok(Spot { form: f.form, comp: f.comp, rect, at })
}

/// The item of window `form` inside `within` (its inside's coordinates)
/// whose accessible name is `name` (any case, `&` dropped; else the first
/// that starts with it): its rectangle — or the names there are.
fn find_item(desk: &mut Desktop, store: &dyn Store, form: &str, within: (f64, f64, f64, f64), name: &str) -> Result<(f64, f64, f64, f64), Vec<String>> {
    let Desktop { forms, text, .. } = desk;
    let Some(f) = forms.get_mut(form) else { return Err(Vec::new()) };
    let tree = f.ui.access_tree(store, text);
    let want = clean(name);
    let inside = |b: (i64, i64, i64, i64)| {
        let (x, y, w, h) = (b.0 as f64, b.1 as f64, b.2 as f64, b.3 as f64);
        w > 0.0 && h > 0.0 && x >= within.0 - 1.0 && y >= within.1 - 1.0 && x + w <= within.0 + within.2 + 1.0 && y + h <= within.1 + within.3 + 1.0
    };
    let mut exact = None;
    let mut prefix = None;
    let mut names = Vec::new();
    let mut stack = vec![&tree];
    while let Some(n) = stack.pop() {
        if inside(n.bounds) {
            let have = clean(&n.name);
            if exact.is_none() && have == want {
                exact = Some(n.bounds);
            } else if prefix.is_none() && !want.is_empty() && have.starts_with(&want) {
                prefix = Some(n.bounds);
            }
            if !n.name.trim().is_empty() && names.len() < 24 {
                names.push(format!("\"{}\"", n.name.trim()));
            }
        }
        stack.extend(n.children.iter().rev());
    }
    exact.or(prefix).map(|b| (b.0 as f64, b.1 as f64, b.2 as f64, b.3 as f64)).ok_or(names)
}

/// What the demo's hands do in one go.
#[derive(Clone, Debug, PartialEq)]
pub enum Input {
    Move,
    Press { button: Button, single: bool },
    Release { button: Button },
    Wheel { dy: f64 },
}

/// `input` at screen point `p` (the window under it gets it): false when
/// no window is there.
pub fn at_point(desk: &mut Desktop, store: &dyn Store, p: (f64, f64), input: &Input) -> bool {
    let Some((form, at)) = record::from_screen(desk, store, p) else { return false };
    match input {
        Input::Move => desk.mouse_move(store, &form, at.0, at.1, Mods::NONE, Source::Script),
        Input::Press { button, single } => {
            if *single {
                if let Some(f) = desk.form(&form) {
                    f.ui.forget_clicks();
                }
            }
            desk.mouse_down(store, &form, at, *button, Mods::NONE, Source::Script);
        }
        Input::Release { button } => desk.mouse_up(store, &form, at, *button, Mods::NONE, Source::Script),
        Input::Wheel { dy } => desk.mouse_wheel(store, &form, at, (0.0, *dy), Mods::NONE, Source::Script),
    }
    true
}

/// The window that has the keyboard: the front one that takes input.
pub fn key_window(desk: &Desktop) -> Option<String> {
    desk.stacking().into_iter().rev().find(|f| desk.accepts_input(f))
}

/// One character typed into what has the keyboard (a key when one types
/// it, else an input method's commit: é, 中, ¡).
pub fn type_char(desk: &mut Desktop, store: &dyn Store, c: char) {
    let Some(form) = key_window(desk) else { return };
    if let Some(f) = desk.form(&form) {
        f.ui.sync(store);
    }
    match c {
        '\n' => key(desk, store, &form, 13, "", Mods::NONE),
        '\t' => key(desk, store, &form, 9, "", Mods::NONE),
        _ => match rapidr_value::input::vk_of_char(c) {
            Some(vk) => {
                let shift = c.is_ascii_uppercase() || "!@#$%^&*()_+{}|:\"<>?~".contains(c);
                key(desk, store, &form, vk, &c.to_string(), if shift { Mods::SHIFT } else { Mods::NONE });
            }
            None => desk.ime_commit(store, &form, &c.to_string(), Source::Script),
        },
    }
}

/// A key pressed and let go in `form`.
pub fn key(desk: &mut Desktop, store: &dyn Store, form: &str, vk: i64, text: &str, mods: Mods) {
    desk.key_down(store, form, vk, text, mods, Source::Script);
    desk.key_up(form, vk, mods, Source::Script);
}

/// Keys as RAppHands writes them (`Enter`, `Ctrl+S`, `Tab Tab`), pressed in
/// what has the keyboard.
pub fn keys(desk: &mut Desktop, store: &dyn Store, spec: &str) -> Result<(), String> {
    let form = key_window(desk).ok_or("No window takes input now.")?;
    for k in spec.split_whitespace() {
        let (vk, text, mods) = hands::parse_key(k)?;
        key(desk, store, &form, vk, &text, mods);
    }
    Ok(())
}
