//! Dialogs the kernel draws itself, as forms in a [`MemStore`]: the
//! MESSAGEBOX / MESSAGEDLG / SHOWMESSAGE box (the text and the buttons
//! `rapidr_value::dialogs` gives, the first one the default), QCOLORDIALOG
//! (Windows' basic colours and custom colours) and QFONTDIALOG (font, style
//! and size lists, a sample) — the system has no colour or font dialog the
//! host could show without blocking (rfd has neither).
//!
//! A dialog is an ordinary kernel form: the host shows its window, routes
//! input into it (Tab, Enter on the default button, Escape, mnemonics,
//! screen readers) and queues its [`KernelEvent`]s; runtime-core hands them
//! to [`Dialog::event`] instead of the program, and steps until it answers.
//! Its components' ids start with [`PREFIX`], which no BASIC name can.
//! What only a dialog draws (a message's icon …) is a [`Part`]
//! (`RDLGPART`, a type no program can make).

use rapidr_value::dialogs::{icon_shapes, message_layout, MsgIcon};
use rapidr_value::objects::a11y::{node_id, AccessNode, Role};
use rapidr_value::objects::text::text_size;
use rapidr_value::Value;

use crate::components::{ComponentKind, Cx};
use crate::input::KernelEvent;
use crate::paint::Painter;
use crate::store::{self, MemStore, Store};

/// What every kernel dialog's ids start with (`:` can't be in a BASIC name).
pub const PREFIX: &str = "rapidr:";

/// Whether `id` is (a component of) a kernel-drawn dialog.
pub fn is_dialog(id: &str) -> bool {
    id.len() >= PREFIX.len() && id[..PREFIX.len()].eq_ignore_ascii_case(PREFIX)
}

/// What a dialog answered when it closed.
#[derive(Clone, Debug, PartialEq)]
pub enum Answer {
    /// A message box's button (its index; `None`: closed or Escape).
    Button(Option<usize>),
    /// A colour dialog's colour (&HBBGGRR; `None`: cancelled).
    Color(Option<i64>),
    /// A font dialog's font (`None`: cancelled).
    Font(Option<FontChoice>),
}

/// A font dialog's choice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontChoice {
    pub name: String,
    pub size: i64,
    pub bold: bool,
    pub italic: bool,
}

enum Kind {
    Message,
    Color { color: i64 },
    Font { choice: FontChoice, names: Vec<String> },
}

/// A kernel-drawn dialog: its form (`id`) and components in `store`.
pub struct Dialog {
    /// The form's id.
    pub id: String,
    pub title: String,
    /// Its window's inside (logical pixels).
    pub size: (i64, i64),
    pub store: MemStore,
    kind: Kind,
}

// Windows' dialog metrics (logical pixels).
const BUTTON: (i64, i64) = (75, 23);

/// A caption shown as is (`&` isn't a mnemonic in a message).
fn literal(text: &str) -> String {
    text.replace('&', "&&")
}

// (a message's lines and its buttons' captions: what every runtime shows)
pub use rapidr_value::dialogs::wrap;
use rapidr_value::dialogs::{button_caption, WRAP};

impl Dialog {
    fn new(n: u64, title: &str, kind: Kind) -> Dialog {
        let id = format!("{PREFIX}dlg{n}");
        let mut store = MemStore::new();
        store.add(&id, "RFORM", None);
        Dialog { id, title: title.to_string(), size: (0, 0), store, kind }
    }

    fn child(&self, part: &str) -> String {
        format!("{}:{part}", self.id)
    }

    /// Adds component `part` of `type_name` at `rect`.
    fn put(&mut self, part: &str, type_name: &str, (x, y, w, h): (i64, i64, i64, i64)) -> String {
        let id = self.child(part);
        let form = self.id.clone();
        self.store.add(&id, type_name, Some(&form));
        for (p, v) in [("left", x), ("top", y), ("width", w), ("height", h)] {
            self.store.set(&id, p, Value::Integer(v));
        }
        id
    }

    fn set(&mut self, id: &str, prop: &str, v: Value) {
        self.store.set(id, prop, v);
    }

    /// The window's inside: `w` × `h`.
    fn finish(&mut self, w: i64, h: i64) {
        self.size = (w, h);
        let id = self.id.clone();
        self.set(&id, "clientwidth", Value::Integer(w));
        self.set(&id, "clientheight", Value::Integer(h));
        self.set(&id, "caption", Value::String(self.title.clone()));
    }

    /// MESSAGEBOX / MESSAGEDLG / SHOWMESSAGE: `icon` (if any) at the top
    /// left, `text` right of it, `labels`' buttons centred under both (the
    /// first the default: Enter), titled `title` — Delphi's MessageDlg
    /// layout (`rapidr_value::dialogs::message_layout`), as every runtime
    /// lays it out.
    pub fn message(n: u64, title: &str, text: &str, labels: &[&str], icon: Option<MsgIcon>) -> Dialog {
        let mut d = Dialog::new(n, title, Kind::Message);
        let font = d.store.font(&d.id);
        let lines = wrap(text, &font, WRAP);
        let line_h = text_size("Ag", &font).1.max(1);
        let text_w = lines.iter().map(|l| text_size(l, &font).0).max().unwrap_or(0);
        let layout = message_layout(text_w, lines.len() as i64 * line_h, labels.len(), icon.is_some());
        if let (Some(icon), Some(rect)) = (icon, layout.icon) {
            let id = d.put("icon", "RDLGPART", rect);
            d.set(&id, "part", Value::String("icon".into()));
            d.set(&id, "icon", Value::Integer(icon.code()));
        }
        let (tx, ty, _, _) = layout.text;
        for (i, line) in lines.iter().enumerate() {
            let id = d.put(&format!("t{i}"), "RLABEL", (tx, ty + i as i64 * line_h, text_w.max(1) + 2, line_h));
            d.set(&id, "caption", Value::String(literal(line)));
        }
        for (i, (label, rect)) in labels.iter().zip(&layout.buttons).enumerate() {
            let id = d.put(&format!("b{i}"), "RBUTTON", *rect);
            d.set(&id, "caption", Value::String(button_caption(label)));
            if i == 0 {
                d.set(&id, "default", Value::Integer(-1));
            }
        }
        d.finish(layout.size.0, layout.size.1);
        d
    }

    /// QCOLORDIALOG: Windows' 48 basic colours and 16 custom ones
    /// (`custom`, white where missing), `color` (&HBBGGRR) chosen first;
    /// OK / Cancel.
    pub fn color(n: u64, title: &str, color: i64, custom: &[i64]) -> Dialog {
        let mut d = Dialog::new(n, title, Kind::Color { color: color & 0xFF_FFFF });
        let label = |d: &mut Dialog, part: &str, text: &str, y: i64| {
            let id = d.put(part, "RLABEL", (6, y, 200, 14));
            d.set(&id, "caption", Value::String(text.into()));
        };
        label(&mut d, "basic", "&Basic colors:", 6);
        // (the selection's frame, under the swatches: drawn first)
        let sel = d.put("sel", "RPANEL", (0, 0, 0, 0));
        d.set(&sel, "bevelouter", Value::Integer(0));
        d.set(&sel, "color", Value::Integer(0));
        for (i, rgb) in BASIC_COLORS.iter().enumerate() {
            let (c, r) = (i as i64 % 8, i as i64 / 8);
            d.swatch(&format!("c{i}"), rgb_to_bgr(*rgb), (6 + c * 26, 22 + r * 22));
        }
        label(&mut d, "custom", "&Custom colors:", 162);
        for i in 0..16 {
            let (c, r) = (i as i64 % 8, i as i64 / 8);
            d.swatch(&format!("u{i}"), custom.get(i).copied().unwrap_or(0xFF_FFFF), (6 + c * 26, 178 + r * 22));
        }
        let define = d.put("define", "RBUTTON", (6, 226, 208, BUTTON.1));
        d.set(&define, "caption", Value::String("&Define Custom Colors >>".into()));
        d.set(&define, "enabled", Value::Integer(0));
        d.ok_cancel((6, 256), (82, 256));
        d.finish(220, 256 + BUTTON.1 + 6);
        d.mark_color();
        d
    }

    fn swatch(&mut self, part: &str, bgr: i64, (x, y): (i64, i64)) {
        let id = self.put(part, "RPANEL", (x + 2, y + 2, 20, 16));
        self.set(&id, "color", Value::Integer(bgr & 0xFF_FFFF));
        self.set(&id, "bevelouter", Value::Integer(rapidr_value::objects::bevel::BV_LOWERED));
    }

    fn ok_cancel(&mut self, ok: (i64, i64), cancel: (i64, i64)) {
        let id = self.put("ok", "RBUTTON", (ok.0, ok.1, 66, BUTTON.1));
        self.set(&id, "caption", Value::String("OK".into()));
        self.set(&id, "default", Value::Integer(-1));
        let id = self.put("cancel", "RBUTTON", (cancel.0, cancel.1, 66, BUTTON.1));
        self.set(&id, "caption", Value::String("Cancel".into()));
        self.set(&id, "cancel", Value::Integer(-1));
    }

    /// The selection's frame around the swatch with the chosen colour
    /// (hidden when none has it).
    fn mark_color(&mut self) {
        let Kind::Color { color } = self.kind else { return };
        let sel = self.child("sel");
        let found = self.store.ids().iter().find(|id| {
            let part = id.rsplit(':').next().unwrap_or("");
            (part.starts_with('c') || part.starts_with('u')) && part[1..].parse::<u32>().is_ok() && store::int(&self.store, id, "color", -1) == color
        });
        let rect = found.map(|id| (store::int(&self.store, id, "left", 0) - 2, store::int(&self.store, id, "top", 0) - 2, 24, 20));
        let (x, y, w, h) = rect.unwrap_or((0, 0, 0, 0));
        for (p, v) in [("left", x), ("top", y), ("width", w), ("height", h)] {
            self.set(&sel, p, Value::Integer(v));
        }
    }

    /// QFONTDIALOG: `names`' fonts (the chosen one added if missing), the
    /// four styles, the usual sizes, a sample in the choice; OK / Cancel.
    pub fn font(n: u64, title: &str, chosen: &FontChoice, names: &[&str]) -> Dialog {
        let mut names: Vec<String> = names.iter().map(|s| s.to_string()).collect();
        if !chosen.name.is_empty() && !names.iter().any(|n| n.eq_ignore_ascii_case(&chosen.name)) {
            names.push(chosen.name.clone());
            names.sort_by_key(|n| n.to_lowercase());
        }
        let mut d = Dialog::new(n, title, Kind::Font { choice: chosen.clone(), names: names.clone() });
        let label = |d: &mut Dialog, part: &str, text: &str, x: i64| {
            let id = d.put(part, "RLABEL", (x, 6, 100, 14));
            d.set(&id, "caption", Value::String(text.into()));
        };
        label(&mut d, "lfont", "&Font:", 6);
        label(&mut d, "lstyle", "Font st&yle:", 162);
        label(&mut d, "lsize", "&Size:", 268);
        let list = |d: &mut Dialog, part: &str, x: i64, w: i64, items: Vec<String>| {
            let id = d.put(part, "RLISTBOX", (x, 22, w, 112));
            let args: Vec<Value> = items.into_iter().map(Value::String).collect();
            d.store.call(&id, "additems", &args);
        };
        list(&mut d, "font", 6, 150, names);
        list(&mut d, "style", 162, 100, STYLES.iter().map(|s| s.to_string()).collect());
        list(&mut d, "size", 268, 50, SIZES.iter().map(|s| s.to_string()).collect());
        let group = d.put("group", "RGROUPBOX", (162, 140, 156, 62));
        d.set(&group, "caption", Value::String("Sample".into()));
        let sample = d.put("sample", "RLABEL", (170, 158, 140, 40));
        d.set(&sample, "caption", Value::String("AaBbYyZz".into()));
        d.set(&sample, "alignment", Value::Integer(2));
        d.ok_cancel((324, 22), (324, 50));
        d.finish(396, 210);
        d.show_font();
        d
    }

    /// The font lists' selections and the sample, from the choice.
    fn show_font(&mut self) {
        let Kind::Font { choice, names } = &self.kind else { return };
        let (choice, names) = (choice.clone(), names.clone());
        let font_index = names.iter().position(|n| n.eq_ignore_ascii_case(&choice.name)).map_or(-1, |i| i as i64);
        let style = i64::from(choice.italic) + 2 * i64::from(choice.bold);
        let size_index = SIZES.iter().position(|s| *s == choice.size).map_or(-1, |i| i as i64);
        for (part, index) in [("font", font_index), ("style", style), ("size", size_index)] {
            let id = self.child(part);
            self.set(&id, "itemindex", Value::Integer(index));
        }
        let sample = self.child("sample");
        self.set(&sample, "fontname", Value::String(choice.name.clone()));
        self.set(&sample, "fontsize", Value::Integer(choice.size));
        self.set(&sample, "fontbold", Value::Integer(-i64::from(choice.bold)));
        self.set(&sample, "fontitalic", Value::Integer(-i64::from(choice.italic)));
    }

    /// The font dialog's choice from its lists.
    fn read_font(&mut self) {
        let index = |d: &Dialog, part: &str| store::int(&d.store, &d.child(part), "itemindex", -1);
        let (font, style, size) = (index(self, "font"), index(self, "style"), index(self, "size"));
        if let Kind::Font { choice, names } = &mut self.kind {
            if let Some(n) = usize::try_from(font).ok().and_then(|i| names.get(i)) {
                choice.name = n.clone();
            }
            if (0..4).contains(&style) {
                choice.italic = style & 1 != 0;
                choice.bold = style & 2 != 0;
            }
            if let Some(s) = usize::try_from(size).ok().and_then(|i| SIZES.get(i)) {
                choice.size = *s;
            }
        }
        self.show_font();
    }

    /// The part of this dialog `id` names (`b0`, `ok` …).
    fn part<'a>(&self, id: &'a str) -> Option<&'a str> {
        let rest = id.get(self.id.len()..)?;
        id[..self.id.len()].eq_ignore_ascii_case(&self.id).then_some(())?;
        rest.strip_prefix(':')
    }

    /// The answer a cancel gives.
    fn cancelled(&self) -> Answer {
        match self.kind {
            Kind::Message => Answer::Button(None),
            Kind::Color { .. } => Answer::Color(None),
            Kind::Font { .. } => Answer::Font(None),
        }
    }

    fn accepted(&self) -> Answer {
        match &self.kind {
            Kind::Message => Answer::Button(Some(0)),
            Kind::Color { color } => Answer::Color(Some(*color)),
            Kind::Font { choice, .. } => Answer::Font(Some(choice.clone())),
        }
    }

    /// One of its form's events: the answer once the dialog closes. (What
    /// it changed is in its store: the form needs syncing.)
    pub fn event(&mut self, ev: &KernelEvent) -> Option<Answer> {
        match ev {
            KernelEvent::Close(_) => Some(self.cancelled()),
            KernelEvent::KeyDown { vk: 27, .. } => Some(self.cancelled()),
            KernelEvent::Set { id, prop, value } => {
                self.store.set(id, prop, Value::Integer(*value));
                None
            }
            KernelEvent::Click(id) => match self.part(id)? {
                "ok" => Some(self.accepted()),
                "cancel" => Some(self.cancelled()),
                p if matches!(self.kind, Kind::Message) => p.strip_prefix('b').and_then(|i| i.parse().ok()).map(|i| Answer::Button(Some(i))),
                "font" | "style" | "size" => {
                    self.read_font();
                    None
                }
                _ => None,
            },
            KernelEvent::Change(id) => {
                if matches!(self.part(id)?, "font" | "style" | "size") {
                    self.read_font();
                }
                None
            }
            KernelEvent::Mouse { id, kind: rapidr_value::input::Mouse::Down, .. } => {
                let part = self.part(id)?;
                let swatch = (part.starts_with('c') || part.starts_with('u')) && part[1..].parse::<u32>().is_ok();
                if let (true, Kind::Color { color }) = (swatch, &mut self.kind) {
                    *color = store::int(&self.store, id, "color", 0);
                    self.mark_color();
                }
                None
            }
            _ => None,
        }
    }

    /// Forgets its components and their models.
    pub fn close(&mut self) {
        self.store.clear();
    }
}

/// What only a dialog draws (`RDLGPART`, its `part` property): a message
/// box's `icon` (`rapidr_value::dialogs::icon_shapes`, its `icon` the
/// [`MsgIcon`]'s code).
pub struct Part;

impl ComponentKind for Part {
    fn name(&self) -> &'static str {
        "RDLGPART"
    }

    fn focusable(&self, _store: &dyn Store, _id: &str) -> bool {
        false
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        if store::string(cx.store, cx.id, "part") == "icon" {
            if let Some(icon) = MsgIcon::from_code(store::int(cx.store, cx.id, "icon", -1)) {
                for shape in icon_shapes(icon) {
                    p.shape(shape);
                }
            }
        }
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = AccessNode::new(node_id(cx.id), Role::Image);
        if let Some(icon) = MsgIcon::from_code(store::int(cx.store, cx.id, "icon", -1)) {
            n.name = icon.name().to_string();
        }
        n.bounds = cx.rect;
        n
    }
}

/// QFONTDIALOG's styles (the index: italic 1 + bold 2).
const STYLES: [&str; 4] = ["Regular", "Italic", "Bold", "Bold Italic"];
/// QFONTDIALOG's sizes (points).
const SIZES: [i64; 16] = [8, 9, 10, 11, 12, 14, 16, 18, 20, 22, 24, 26, 28, 36, 48, 72];

/// The fonts a font dialog offers: the faces RapidQ programs name, all
/// drawn with RapidR's built-in Liberation fonts
/// (`rapidr_value::objects::text::family_name`).
pub const FONT_NAMES: [&str; 8] = ["Arial", "Courier New", "Georgia", "MS Sans Serif", "Tahoma", "Times New Roman", "Trebuchet MS", "Verdana"];

fn rgb_to_bgr(rgb: u32) -> i64 {
    let (r, g, b) = ((rgb >> 16) & 0xFF, (rgb >> 8) & 0xFF, rgb & 0xFF);
    i64::from(b << 16 | g << 8 | r)
}

/// Windows' ChooseColor basic colours (RGB), by rows.
const BASIC_COLORS: [u32; 48] = [
    0xFF8080, 0xFFFF80, 0x80FF80, 0x00FF80, 0x80FFFF, 0x0080FF, 0xFF80C0, 0xFF80FF, //
    0xFF0000, 0xFFFF00, 0x80FF00, 0x00FF40, 0x00FFFF, 0x0080C0, 0x8080C0, 0xFF00FF, //
    0x804040, 0xFF8040, 0x00FF00, 0x008080, 0x004080, 0x8080FF, 0x800040, 0xFF0080, //
    0x800000, 0xFF8000, 0x008000, 0x008040, 0x0000FF, 0x0000A0, 0x800080, 0x8000FF, //
    0x400000, 0x804000, 0x004000, 0x004040, 0x000080, 0x000040, 0x400040, 0x400080, //
    0x000000, 0x808000, 0x808040, 0x808080, 0x408080, 0xC0C0C0, 0x400040, 0xFFFFFF,
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{Clipboard, MemClipboard, Mods};
    use rapidr_value::objects::font::Font;
    use crate::text::TextSystem;
    use crate::tree::FormUi;
    use rapidr_value::input::Button;

    fn ui(d: &Dialog) -> FormUi {
        FormUi::build(&d.store, &d.id, false)
    }

    /// The events `f` makes the dialog answer, in order (the first answer).
    fn answer(d: &mut Dialog, f: &mut FormUi) -> Option<Answer> {
        f.take_events().iter().find_map(|e| d.event(e))
    }

    fn click(d: &mut Dialog, f: &mut FormUi, ts: &mut TextSystem, part: &str) -> Option<Answer> {
        f.sync(&d.store);
        let n = f.node(&d.child(part)).expect("the part");
        let (x, y) = (n.abs.0 as f64 + 4.5, n.abs.1 as f64 + 4.5);
        f.mouse_down(&d.store, ts, x, y, Button::Left, Mods::NONE);
        f.mouse_up(&d.store, ts, x, y, Button::Left, Mods::NONE);
        answer(d, f)
    }

    fn key(d: &mut Dialog, f: &mut FormUi, ts: &mut TextSystem, vk: i64) -> Option<Answer> {
        let mut clip: Box<dyn Clipboard> = Box::new(MemClipboard::default());
        f.key_down(&d.store, ts, vk, "", Mods::NONE, clip.as_mut());
        f.key_up(vk, Mods::NONE);
        answer(d, f)
    }

    #[test]
    fn message_box_buttons_enter_escape_and_close() {
        let mut ts = TextSystem::new();
        let mut d = Dialog::message(1, "Question", "Save the changes\nbefore closing?", &["Yes", "No", "Cancel"], None);
        assert!(is_dialog(&d.id) && is_dialog("RAPIDR:dlg1:b0") && !is_dialog("form"));
        // (two lines, three buttons centred under them)
        assert!(d.store.ids().contains(&"rapidr:dlg1:t1".to_string()));
        let (w, h) = d.size;
        assert!(w >= 3 * 75 + 2 * 6 + 24 && h > 23 + 24, "{w}x{h}");
        assert!(!d.store.ids().contains(&"rapidr:dlg1:icon".to_string()));
        let mut f = ui(&d);
        assert_eq!(click(&mut d, &mut f, &mut ts, "b1"), Some(Answer::Button(Some(1))));
        // Enter: the focused (first, default) button; Escape: none
        let mut f = ui(&d);
        assert_eq!(key(&mut d, &mut f, &mut ts, 13), Some(Answer::Button(Some(0))));
        assert_eq!(key(&mut d, &mut f, &mut ts, 27), Some(Answer::Button(None)));
        f.close_box();
        assert_eq!(answer(&mut d, &mut f), Some(Answer::Button(None)));
        // (a message's & is shown, not a mnemonic)
        let d2 = Dialog::message(2, "", "R&D", &["OK"], None);
        assert_eq!(store::string(&d2.store, "rapidr:dlg2:t0", "caption"), "R&&D");
        d.close();
        assert!(d.store.ids().is_empty());
    }

    #[test]
    fn message_box_icon_left_of_the_text() {
        let mut ts = TextSystem::new();
        let d = Dialog::message(6, "Warning", "Disk full", &["OK"], Some(MsgIcon::Warning));
        let int = |id: &str, p: &str| store::int(&d.store, id, p, -1);
        // the icon at Delphi's margins, the text 15 right of it, the button
        // under the icon (the text is shorter than it)
        assert_eq!((int("rapidr:dlg6:icon", "left"), int("rapidr:dlg6:icon", "top"), int("rapidr:dlg6:icon", "width")), (12, 13, 32));
        assert_eq!(int("rapidr:dlg6:t0", "left"), 12 + 32 + 15);
        assert_eq!(int("rapidr:dlg6:b0", "top"), 13 + 32 + 16);
        // drawn as the shared shapes; a screen reader hears its name
        let mut f = ui(&d);
        let list = f.paint(&d.store, &mut ts, 1.0);
        let shapes = list.items.iter().filter(|i| matches!(i, crate::display::Item::Op { op: rapidr_value::objects::ops::Op::Shape(_), origin: (12, 13) })).count();
        assert_eq!(shapes, icon_shapes(MsgIcon::Warning).len());
        let tree = f.access_tree(&d.store, &mut ts).to_json();
        assert!(tree.contains(r#""role":"img","name":"Warning""#), "{tree}");
    }

    #[test]
    fn long_lines_wrap() {
        let font = Font { name: "Arial".into(), size: 10, color: 0, styles: 0 };
        let lines = wrap(&"word ".repeat(100), &font, 200);
        assert!(lines.len() > 5 && lines.iter().all(|l| text_size(l, &font).0 <= 200));
        assert_eq!(wrap("a\r\nb\rc", &font, 200), ["a", "b", "c"]);
    }

    #[test]
    fn color_dialog_picks_a_swatch() {
        let mut ts = TextSystem::new();
        // (red, &H0000FF, is the second row's first basic colour)
        let mut d = Dialog::color(3, "Color", 0x0000FF, &[]);
        assert_eq!(store::int(&d.store, "rapidr:dlg3:sel", "left", -1), 6 + 2 - 2);
        assert_eq!(store::int(&d.store, "rapidr:dlg3:sel", "top", -1), 22 + 22 + 2 - 2);
        let mut f = ui(&d);
        assert_eq!(click(&mut d, &mut f, &mut ts, "c26"), None);
        assert_eq!(click(&mut d, &mut f, &mut ts, "ok"), Some(Answer::Color(Some(0x008000))));
        let mut f = ui(&d);
        assert_eq!(key(&mut d, &mut f, &mut ts, 27), Some(Answer::Color(None)));
        // (Enter: OK)
        assert_eq!(key(&mut d, &mut f, &mut ts, 13), Some(Answer::Color(Some(0x008000))));
    }

    #[test]
    fn font_dialog_lists_and_sample() {
        let mut ts = TextSystem::new();
        let chosen = FontChoice { name: "Arial".into(), size: 10, bold: false, italic: false };
        let mut d = Dialog::font(4, "Font", &chosen, &FONT_NAMES);
        assert_eq!(store::int(&d.store, "rapidr:dlg4:size", "itemindex", -1), 2);
        assert_eq!(store::int(&d.store, "rapidr:dlg4:font", "itemindex", -1), 0);
        // the style list's "Bold" row, picked by the mouse
        let mut f = ui(&d);
        f.sync(&d.store);
        let n = f.node("rapidr:dlg4:style").expect("the style list");
        let (x, y) = (n.abs.0 as f64 + 10.0, n.abs.1 as f64 + 2.0 + 2.5 * 16.0);
        f.mouse_down(&d.store, &mut ts, x, y, Button::Left, Mods::NONE);
        f.mouse_up(&d.store, &mut ts, x, y, Button::Left, Mods::NONE);
        assert_eq!(answer(&mut d, &mut f), None);
        assert_eq!(store::int(&d.store, "rapidr:dlg4:sample", "fontbold", 0), -1);
        let picked = click(&mut d, &mut f, &mut ts, "ok");
        assert_eq!(picked, Some(Answer::Font(Some(FontChoice { bold: true, ..chosen.clone() }))));
        // (a font the list hasn't: added)
        let d2 = Dialog::font(5, "Font", &FontChoice { name: "Comic Sans MS".into(), ..chosen }, &FONT_NAMES);
        assert_eq!(store::int(&d2.store, "rapidr:dlg5:font", "itemindex", -1), 1);
    }
}
