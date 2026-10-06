//! RTOOLBAR's model (docs/ide-components.md §3.8): its own buttons — icon
//! buttons, toggles, separators — laid out along it ([`ToolBar::layout`]:
//! square buttons, the ones that don't fit behind a "»" button whose menu
//! lists them, and with Customizable a check mark per button to show or
//! hide it), what the user did to them, and the program's members and
//! events through the runtime glue. The kernel draws it
//! (`rapidr-ui-kernel`'s `components/panels/toolbar.rs`).
//!
//! Components the program places on a toolbar (QCOOLBTNs …) stay a
//! container's children: they sit after its own buttons (the kernel's
//! client area starts there), and a toolbar without buttons is the strip it
//! always was.
//!
//! A click: a toggle turns over, then OnButtonClick (Name, Command), then
//! the component's OnClick with `ClickedButton` naming the button (empty
//! for a click on the strip itself).

use std::sync::Arc;

use super::runtime::{basic_bool, truth, Runtime};
use crate::objects::font::Font;
use crate::objects::ops::Rect;
use crate::objects::text::text_size;
use crate::Value;

/// What a toolbar item is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    #[default]
    Button,
    Toggle,
    Separator,
}

/// A picture file's pixels (a button whose Icon isn't one of RapidR's):
/// width, height, RGBA, and a revision naming them.
#[derive(Clone, Debug, PartialEq)]
pub struct Picture {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
    pub revision: u64,
}

/// A button or a separator.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Item {
    pub name: String,
    pub kind: Kind,
    /// RapidR's icon name (`save`, `file.save`, `QBUTTON` …) or a picture
    /// file's path.
    pub icon: String,
    pub hint: String,
    pub command: String,
    pub caption: String,
    pub enabled: bool,
    /// The program's ButtonVisible.
    pub visible: bool,
    /// The user hid it (Customizable).
    pub hidden: bool,
    /// A toggle that's down.
    pub down: bool,
    /// The picture file's pixels, when Icon is one.
    pub picture: Option<Arc<Picture>>,
}

impl Item {
    pub fn is_separator(&self) -> bool {
        self.kind == Kind::Separator
    }

    /// What it's called: its hint, else its caption, else its name.
    pub fn label(&self) -> String {
        [&self.hint, &self.caption, &self.name].into_iter().find(|s| !s.trim().is_empty()).cloned().unwrap_or_default()
    }

    /// On the bar or in its menu (the program shows it, the user didn't
    /// hide it).
    pub fn shown(&self) -> bool {
        self.visible && !self.hidden
    }
}

/// An entry of the "»" button's menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    /// A button that didn't fit: picking it clicks it.
    Overflow(usize),
    /// Customizable's: picking it shows or hides the button.
    Customize(usize),
    /// The line above Customizable's (picking it does nothing).
    Heading,
}

/// Where its buttons are (its own pixels).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layout {
    /// The items on the bar: index, rectangle.
    pub slots: Vec<(usize, Rect)>,
    /// The shown items that didn't fit (buttons only).
    pub overflow: Vec<usize>,
    /// The "»" button, when there is one.
    pub chevron: Option<Rect>,
    /// Where the strip's own part ends: the components on it start there.
    pub end: i64,
}

impl Layout {
    /// The item at (x, y), if a button's there.
    pub fn item_at(&self, x: i64, y: i64) -> Option<usize> {
        self.slots.iter().find(|(_, r)| x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3).map(|s| s.0)
    }

    pub fn rect_of(&self, item: usize) -> Option<Rect> {
        self.slots.iter().find(|s| s.0 == item).map(|s| s.1)
    }

    pub fn on_chevron(&self, x: i64, y: i64) -> bool {
        self.chevron.is_some_and(|r| x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3)
    }
}

/// The gap at the strip's ends and between buttons.
pub const PAD: i64 = 2;
pub const GAP: i64 = 1;
/// A separator's width.
pub const SEPARATOR: i64 = 9;
/// The check mark of a shown button in the customize menu.
pub const CHECK: &str = "√ ";
pub const NO_CHECK: &str = "    ";
/// The heading of Customizable's lines in the menu.
pub const SHOWN_HEADING: &str = "Buttons shown:";

/// RTOOLBAR's state.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolBar {
    pub items: Vec<Item>,
    pub button_size: i64,
    pub show_captions: bool,
    pub customizable: bool,
    /// The button the last click was on (ClickedButton).
    pub clicked: String,
    /// (the kernel's) The item under the mouse, the one pressed; the "»"
    /// button's.
    pub hover: Option<usize>,
    pub pressed: Option<usize>,
    pub chevron_hover: bool,
    pub chevron_pressed: bool,
    /// The menu as last dropped (a pick names one of its entries).
    pub menu: Vec<(String, Entry)>,
    /// (the kernel's) The items that didn't fit, as last laid out.
    pub overflow: Vec<usize>,
    /// Goes up with each picture loaded.
    revision: u64,
}

impl Default for ToolBar {
    fn default() -> Self {
        ToolBar {
            items: Vec::new(),
            button_size: 28,
            show_captions: false,
            customizable: false,
            clicked: String::new(),
            hover: None,
            pressed: None,
            chevron_hover: false,
            chevron_pressed: false,
            menu: Vec::new(),
            overflow: Vec::new(),
            revision: 0,
        }
    }
}

crate::panel_models!(ToolBar);

impl ToolBar {
    /// Item `name`'s index (any case).
    pub fn find(&self, name: &str) -> Option<usize> {
        self.items.iter().position(|i| i.name.eq_ignore_ascii_case(name.trim()))
    }

    pub fn add(&mut self, item: Item) {
        self.items.push(item);
    }

    /// Removes item `name`; whether it was there.
    pub fn remove(&mut self, name: &str) -> bool {
        let Some(i) = self.find(name) else { return false };
        self.items.remove(i);
        self.hover = None;
        self.pressed = None;
        self.overflow.clear();
        true
    }

    pub fn clear(&mut self) {
        *self = ToolBar { button_size: self.button_size, show_captions: self.show_captions, customizable: self.customizable, revision: self.revision, ..ToolBar::default() };
    }

    /// The icons' size for the buttons' size: 16, 20 when they're 32 or
    /// more.
    pub fn icon_size(&self) -> i64 {
        if self.button_size >= 32 {
            20
        } else {
            16
        }
    }

    /// Item `i`'s width on the bar.
    pub fn item_width(&self, i: usize, font: &Font) -> i64 {
        let it = &self.items[i];
        if it.is_separator() {
            return SEPARATOR;
        }
        let bs = self.button_size;
        if self.show_captions && !it.caption.is_empty() {
            let (tw, _) = text_size(&it.caption, font);
            let side = (bs - self.icon_size()) / 2;
            side + self.icon_size() + 4 + tw + side + 2
        } else {
            bs
        }
    }

    /// The "»" button's width.
    pub fn chevron_width(&self) -> i64 {
        (self.button_size / 2 + 2).max(14)
    }

    /// Its buttons along a strip `w` × `h`, `taken` pixels at its right end
    /// kept for the components on it.
    pub fn layout(&self, w: i64, h: i64, taken: i64, font: &Font) -> Layout {
        let bs = self.button_size;
        let top = ((h - bs) / 2).max(0);
        let shown: Vec<usize> = (0..self.items.len()).filter(|&i| self.items[i].shown()).collect();
        if shown.is_empty() && !self.customizable {
            return Layout::default();
        }
        let room = w - taken - PAD;
        let widths: Vec<i64> = shown.iter().map(|&i| self.item_width(i, font)).collect();
        let all: i64 = PAD + widths.iter().map(|w| w + GAP).sum::<i64>();
        let fits = all <= room;
        let limit = if fits && !self.customizable { room } else { room - self.chevron_width() - GAP };
        let mut out = Layout::default();
        let mut x = PAD;
        let mut cut = false;
        for (k, &i) in shown.iter().enumerate() {
            let iw = widths[k];
            if !cut && x + iw <= limit {
                out.slots.push((i, (x, top, iw, bs.min(h))));
                x += iw + GAP;
            } else {
                // (from the first that doesn't fit on; separators dropped)
                cut = true;
                if !self.items[i].is_separator() {
                    out.overflow.push(i);
                }
            }
        }
        // (no separator at the bar's end)
        while out.slots.last().is_some_and(|s| self.items[s.0].is_separator()) {
            out.slots.pop();
        }
        x = out.slots.last().map_or(PAD, |s| s.1 .0 + s.1 .2 + GAP);
        if !out.overflow.is_empty() || self.customizable {
            let cw = self.chevron_width();
            out.chevron = Some((x, top, cw, bs.min(h)));
            x += cw + GAP;
        }
        out.end = x + PAD - GAP;
        out
    }

    /// The "»" menu: the buttons that didn't fit (`overflow`), then with
    /// Customizable a line per button, checked when it shows.
    pub fn menu_entries(&self, overflow: &[usize]) -> Vec<(String, Entry)> {
        let mut out = Vec::new();
        for &i in overflow {
            let it = &self.items[i];
            if it.is_separator() || !it.enabled {
                continue;
            }
            let label = it.label();
            out.push((if it.kind == Kind::Toggle && it.down { format!("{label} (on)") } else { label }, Entry::Overflow(i)));
        }
        if self.customizable {
            out.push((SHOWN_HEADING.to_string(), Entry::Heading));
            for (i, it) in self.items.iter().enumerate() {
                if it.is_separator() || !it.visible {
                    continue;
                }
                out.push((format!("{}{}", if it.hidden { NO_CHECK } else { CHECK }, it.label()), Entry::Customize(i)));
            }
        }
        out
    }

    /// The buttons the user hid, as text (Layout): their names, a comma
    /// between.
    pub fn layout_text(&self) -> String {
        self.items.iter().filter(|i| i.hidden && !i.is_separator()).map(|i| i.name.clone()).collect::<Vec<_>>().join(",")
    }

    /// Layout set back: the buttons it names hidden, the others shown.
    pub fn set_layout_text(&mut self, text: &str) {
        let names: Vec<String> = text.split([',', ';', '\n']).map(|s| s.trim().to_ascii_lowercase()).filter(|s| !s.is_empty()).collect();
        for it in &mut self.items {
            if !it.is_separator() {
                it.hidden = names.contains(&it.name.to_ascii_lowercase());
            }
        }
    }

    fn next_revision(&mut self) -> u64 {
        self.revision += 1;
        self.revision
    }
}

/// A picture file's pixels (BMP, PNG, JPEG, ICO, SVG), read through the
/// objects' file hooks.
fn load_picture(path: &str, revision: u64) -> Option<Arc<Picture>> {
    let bytes = crate::objects::read_file(path).ok()?;
    use crate::objects::codec;
    let (px, alpha) = if codec::is_svg(&bytes) {
        let (px, a) = codec::decode_svg(&bytes, 1.0).ok()?;
        (px, Some(a))
    } else {
        codec::decode_raster(&bytes).ok()?
    };
    let mut rgba = Vec::with_capacity(px.width * px.height * 4);
    for (k, c) in px.pixels.iter().enumerate() {
        // (RapidQ's colours are &HBBGGRR)
        rgba.extend_from_slice(&[(c & 0xFF) as u8, ((c >> 8) & 0xFF) as u8, ((c >> 16) & 0xFF) as u8, alpha.as_ref().map_or(255, |a| a.get(k).copied().unwrap_or(255))]);
    }
    Some(Arc::new(Picture { width: px.width, height: px.height, rgba, revision }))
}

/// What the user did to it (from the kernel).
#[derive(Clone, Debug, PartialEq)]
pub enum User {
    /// Item `i` clicked (`None`: the strip itself).
    Click(Option<usize>),
    /// The "»" button pressed: its menu drops under `anchor` (the form's
    /// client area), as wide as its entries.
    Menu(Rect),
}

/// Whether `name` is one of RapidR's icons (else it's a picture file).
fn is_icon(name: &str) -> bool {
    #[cfg(feature = "icons")]
    return rapidr_icons::get(name).is_some();
    #[cfg(not(feature = "icons"))]
    {
        let _ = name;
        false
    }
}

/// A button made by AddButton / AddToggle from their arguments.
fn button(kind: Kind, args: &[Value], revision: u64) -> Item {
    let s = |k: usize| args.get(k).map(Value::to_string_val).unwrap_or_default();
    let icon = s(1);
    let picture = if !icon.is_empty() && !is_icon(&icon) { load_picture(&icon, revision) } else { None };
    Item { name: s(0), kind, icon, hint: s(2), command: s(3), caption: s(4), enabled: true, visible: true, hidden: false, down: false, picture }
}

/// Its methods; `None`: not one of its own.
pub fn rt_method<R: Runtime>(_rt: R, name: &str, method: &str, args: &[Value]) -> Option<Value> {
    let s = |k: usize| args.get(k).map(Value::to_string_val).unwrap_or_default();
    let on = args.get(1).filter(|v| !matches!(v, Value::Null)).map(truth);
    // (ButtonEnabled / ButtonDown / ButtonVisible: read, or set with On)
    let flag = |get: fn(&Item) -> bool, set: fn(&mut Item, bool)| -> Value {
        with_mut(name, |t| match t.find(&s(0)) {
            Some(i) => {
                if let Some(on) = on {
                    set(&mut t.items[i], on);
                }
                basic_bool(get(&t.items[i]))
            }
            None => basic_bool(false),
        })
    };
    Some(match method {
        "addbutton" | "addtoggle" => {
            let kind = if method == "addtoggle" { Kind::Toggle } else { Kind::Button };
            with_mut(name, |t| {
                let rev = t.next_revision();
                t.add(button(kind, args, rev));
            });
            Value::Null
        }
        "addseparator" => {
            let n = s(0);
            with_mut(name, |t| t.add(Item { name: if n.is_empty() { "-".into() } else { n }, kind: Kind::Separator, enabled: true, visible: true, ..Item::default() }));
            Value::Null
        }
        "removebutton" => basic_bool(with_mut(name, |t| t.remove(&s(0)))),
        "clear" => {
            with_mut(name, ToolBar::clear);
            Value::Null
        }
        "button" => {
            let i = args.first().map_or(-1, Value::to_i64);
            Value::String(with(name, |t| usize::try_from(i).ok().and_then(|i| t.items.get(i)).map(|it| if it.is_separator() { "-".to_string() } else { it.name.clone() })).flatten().unwrap_or_default())
        }
        "buttonenabled" => flag(|i| i.enabled, |i, on| i.enabled = on),
        "buttondown" => flag(|i| i.down, |i, on| i.down = on),
        "buttonvisible" => flag(|i| i.visible, |i, on| i.visible = on),
        "buttonhint" => {
            let hint = args.get(1).filter(|v| !matches!(v, Value::Null)).map(Value::to_string_val);
            Value::String(with_mut(name, |t| match t.find(&s(0)) {
                Some(i) => {
                    if let Some(h) = hint {
                        t.items[i].hint = h;
                    }
                    t.items[i].hint.clone()
                }
                None => String::new(),
            }))
        }
        _ => return None,
    })
}

/// Its properties its model answers.
pub fn rt_get<R: Runtime>(_rt: R, name: &str, prop: &str) -> Option<Value> {
    let t = with(name, ToolBar::clone).unwrap_or_default();
    Some(match prop {
        "buttonsize" => Value::Integer(t.button_size),
        "showcaptions" => basic_bool(t.show_captions),
        "customizable" => basic_bool(t.customizable),
        "buttoncount" => Value::Integer(t.items.len() as i64),
        "layout" => Value::String(t.layout_text()),
        "clickedbutton" => Value::String(t.clicked.clone()),
        _ => return None,
    })
}

/// Its properties its model keeps: whether `prop` was one.
pub fn rt_set<R: Runtime>(_rt: R, name: &str, prop: &str, v: &Value) -> bool {
    match prop {
        "buttonsize" => with_mut(name, |t| t.button_size = v.to_i64().clamp(12, 128)),
        "showcaptions" => with_mut(name, |t| t.show_captions = truth(v)),
        "customizable" => with_mut(name, |t| t.customizable = truth(v)),
        "layout" => with_mut(name, |t| t.set_layout_text(&v.to_string_val())),
        _ => return false,
    }
    true
}

/// Item `i` clicked (by the mouse, its menu, a screen reader): a toggle
/// turns over, then OnButtonClick and OnClick.
fn click<R: Runtime>(rt: R, name: &str, i: Option<usize>) {
    let hit = with_mut(name, |t| {
        let Some(i) = i else {
            t.clicked.clear();
            return Some(None);
        };
        let it = t.items.get_mut(i).filter(|it| !it.is_separator() && it.enabled && it.shown())?;
        if it.kind == Kind::Toggle {
            it.down = !it.down;
        }
        let (n, c) = (it.name.clone(), it.command.clone());
        t.clicked = n.clone();
        Some(Some((n, c)))
    });
    match hit {
        Some(Some((n, c))) => {
            rt.fire(name, "onbuttonclick", &[Value::String(n), Value::String(c)]);
            rt.fire(name, "onclick", &[]);
        }
        Some(None) => rt.fire(name, "onclick", &[]),
        None => {}
    }
}

/// What the user did.
pub fn rt_user<R: Runtime>(rt: R, name: &str, action: User) {
    match action {
        User::Click(i) => click(rt, name, i),
        User::Menu(anchor) => {
            let items = with_mut(name, |t| {
                t.menu = t.menu_entries(&t.overflow.clone());
                t.menu.iter().map(|m| m.0.clone()).collect::<Vec<_>>()
            });
            if let Some(form) = rt.form_of(name) {
                rt.drop_list(&form, name, items, anchor);
            }
        }
    }
}

/// An item picked from its overflow menu.
pub fn rt_picked<R: Runtime>(rt: R, name: &str, item: String) {
    let entry = with_mut(name, |t| {
        t.chevron_pressed = false;
        t.menu.iter().find(|m| m.0 == item).map(|m| m.1.clone())
    });
    match entry {
        Some(Entry::Overflow(i)) => click(rt, name, Some(i)),
        Some(Entry::Customize(i)) => with_mut(name, |t| {
            if let Some(it) = t.items.get_mut(i) {
                it.hidden = !it.hidden;
            }
            t.overflow.clear();
        }),
        Some(Entry::Heading) | None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn font() -> Font {
        Font { name: "Arial".into(), size: 9, color: 0, styles: 0 }
    }

    fn bar(names: &[&str]) -> ToolBar {
        let mut t = ToolBar::default();
        for n in names {
            if *n == "-" {
                t.add(Item { name: "-".into(), kind: Kind::Separator, enabled: true, visible: true, ..Item::default() });
            } else {
                t.add(Item { name: n.to_string(), icon: n.to_string(), hint: format!("{n} hint"), enabled: true, visible: true, ..Item::default() });
            }
        }
        t
    }

    #[test]
    fn buttons_are_laid_out_square() {
        let t = bar(&["new", "open", "-", "save"]);
        let l = t.layout(400, 32, 0, &font());
        assert_eq!(l.slots, vec![(0, (2, 2, 28, 28)), (1, (31, 2, 28, 28)), (2, (60, 2, 9, 28)), (3, (70, 2, 28, 28))]);
        assert!(l.overflow.is_empty() && l.chevron.is_none());
        assert_eq!(l.end, 100);
        assert_eq!(l.item_at(40, 10), Some(1));
        assert_eq!(l.item_at(40, 31), None);
        // (none: the strip as it was)
        assert_eq!(ToolBar::default().layout(400, 32, 0, &font()), Layout::default());
    }

    #[test]
    fn what_doesnt_fit_goes_behind_the_chevron() {
        let t = bar(&["new", "open", "-", "save", "run", "stop"]);
        // room for 2 buttons and the chevron
        let l = t.layout(2 + 29 * 2 + 16 + 2 + 1, 32, 0, &font());
        assert_eq!(l.slots.iter().map(|s| s.0).collect::<Vec<_>>(), vec![0, 1]);
        assert_eq!(l.overflow, vec![3, 4, 5], "the separator at the cut is dropped");
        assert_eq!(l.chevron, Some((60, 2, 16, 28)));
        let menu = t.menu_entries(&l.overflow);
        assert_eq!(menu.iter().map(|m| m.0.as_str()).collect::<Vec<_>>(), vec!["save hint", "run hint", "stop hint"]);
        // (components on it take room too)
        let l2 = t.layout(400, 32, 300, &font());
        assert!(!l2.overflow.is_empty());
    }

    #[test]
    fn customizable_hides_and_layout_keeps_it() {
        let mut t = bar(&["new", "open", "save"]);
        t.customizable = true;
        let l = t.layout(400, 32, 0, &font());
        assert!(l.chevron.is_some(), "a customizable bar always has its menu");
        let menu = t.menu_entries(&l.overflow);
        assert_eq!(menu[0], (SHOWN_HEADING.to_string(), Entry::Heading));
        assert_eq!(menu[2], (format!("{CHECK}open hint"), Entry::Customize(1)));
        t.items[1].hidden = true;
        assert_eq!(t.layout_text(), "open");
        let l = t.layout(400, 32, 0, &font());
        assert_eq!(l.slots.iter().map(|s| s.0).collect::<Vec<_>>(), vec![0, 2]);
        assert_eq!(t.menu_entries(&[])[2].0, format!("{NO_CHECK}open hint"));
        t.set_layout_text("NEW, save");
        assert_eq!(t.layout_text(), "new,save");
    }

    #[test]
    fn captions_widen_their_buttons() {
        let mut t = bar(&["run"]);
        t.items[0].caption = "Run".into();
        assert_eq!(t.item_width(0, &font()), 28);
        t.show_captions = true;
        assert!(t.item_width(0, &font()) > 40);
        t.button_size = 32;
        assert_eq!(t.icon_size(), 20);
    }
}
