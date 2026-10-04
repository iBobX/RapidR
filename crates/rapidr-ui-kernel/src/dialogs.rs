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

use rapidr_value::color_dialog::{self as cd, layout as cl, State as ColorState};
use rapidr_value::font_dialog as fd;
use rapidr_value::objects::font::Font;
use rapidr_value::dialogs::{icon_shapes, message_layout, MsgIcon};
use rapidr_value::objects::a11y::{node_id, AccessNode, Role};
use rapidr_value::objects::text::text_size;
use rapidr_value::Value;

use std::collections::HashMap;
use std::sync::Arc;

use rapidr_value::objects::ops::Op;

use crate::components::{ComponentKind, Cx};
use crate::display::Picture;
use crate::input::KernelEvent;
use crate::paint::{Painter, LIGHT, SHADOW};
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
    /// A colour dialog's colour (&HBBGGRR; `None`: cancelled) and its
    /// custom colours (what the user added stays, OK or Cancel).
    Color(Option<i64>, [i64; 16]),
    /// A font dialog's font (`None`: cancelled).
    Font(Option<Font>),
}

/// A font dialog's lists and what it shows (`rapidr_value::font_dialog`).
struct FontState {
    /// The font chosen, and the options.
    req: fd::Request,
    names: Vec<String>,
    sizes: Vec<i64>,
    colors: Vec<(String, i64)>,
    /// The font, style and size lists show no selection until picked in
    /// (fdNoFaceSel, fdNoStyleSel, fdNoSizeSel).
    unselected: [bool; 3],
}

enum Kind {
    Message,
    /// The colour dialog's state (`rapidr_value::color_dialog`, shared
    /// with the web's), and what the mouse drags (the spectrum, the
    /// luminance bar).
    Color { state: ColorState, drag: Option<&'static str> },
    Font(Box<FontState>),
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
    /// A font dialog's Apply pressed: the font to store before OnApply
    /// fires (`take_applied`).
    applied: Option<Font>,
}

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
        Dialog { id, title: title.to_string(), size: (0, 0), store, kind, applied: None }
    }

    /// A font dialog's Apply, since the last call: the font then.
    pub fn take_applied(&mut self) -> Option<Font> {
        self.applied.take()
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

    /// QCOLORDIALOG (`rapidr_value::color_dialog`): Windows' 48 basic
    /// colours and the 16 custom ones, the chosen one framed; "Define
    /// Custom Colors >>" (enabled only when the Style lets the editor open
    /// later); with the editor (cdFullOpen / cdNormal, or after Define):
    /// the hue / saturation field, the luminance bar, the colour's sample,
    /// the Hue / Sat / Lum and Red / Green / Blue boxes and "Add to Custom
    /// Colors". OK / Cancel.
    pub fn color(n: u64, title: &str, state: ColorState) -> Dialog {
        let full = state.full();
        let mut d = Dialog::new(n, title, Kind::Color { state, drag: None });
        let label = |d: &mut Dialog, part: &str, text: &str, rect: cd::Rect| {
            let id = d.put(part, "RLABEL", rect);
            d.set(&id, "caption", Value::String(text.into()));
        };
        label(&mut d, "basic", "&Basic colors:", cl::BASIC_LABEL);
        // (the selection's frame, under the swatches: drawn first)
        let sel = d.put("sel", "RPANEL", (0, 0, 0, 0));
        d.set(&sel, "bevelouter", Value::Integer(0));
        d.set(&sel, "color", Value::Integer(0));
        for (i, rgb) in cd::BASIC_COLORS.iter().enumerate() {
            d.swatch(&format!("c{i}"), cd::swap_rb(i64::from(*rgb)), cl::basic(i));
        }
        label(&mut d, "custom", "&Custom colors:", cl::CUSTOM_LABEL);
        for i in 0..16 {
            d.swatch(&format!("u{i}"), 0, cl::custom(i));
        }
        let define = d.put("define", "RBUTTON", cl::DEFINE);
        d.set(&define, "caption", Value::String("&Define Custom Colors >>".into()));
        let id = d.put("ok", "RBUTTON", cl::OK);
        d.set(&id, "caption", Value::String("OK".into()));
        d.set(&id, "default", Value::Integer(-1));
        let id = d.put("cancel", "RBUTTON", cl::CANCEL);
        d.set(&id, "caption", Value::String("Cancel".into()));
        d.set(&id, "cancel", Value::Integer(-1));
        if full {
            d.color_editor();
        }
        let (w, h) = if full { cl::FULL } else { cl::COMPACT };
        d.finish(w, h);
        d.show_color(None);
        d
    }

    /// The custom colour editor's parts, right of the swatches.
    fn color_editor(&mut self) {
        for (part, rect) in [("spectrum", cl::SPECTRUM), ("lum", cl::LUM), ("preview", cl::PREVIEW)] {
            let id = self.put(part, "RDLGPART", rect);
            self.set(&id, "part", Value::String(part.into()));
        }
        let id = self.put("lpreview", "RLABEL", cl::PREVIEW_LABEL);
        self.set(&id, "caption", Value::String("Color|S&olid".into()));
        self.set(&id, "alignment", Value::Integer(2));
        for (i, caption) in cd::FIELDS.iter().enumerate() {
            let (label, edit) = cl::field(i);
            let id = self.put(&format!("l{i}"), "RLABEL", label);
            self.set(&id, "caption", Value::String(caption.to_string()));
            let id = self.put(&format!("f{i}"), "REDIT", edit);
            self.set(&id, "maxlength", Value::Integer(3));
        }
        let id = self.put("add", "RBUTTON", cl::ADD);
        self.set(&id, "caption", Value::String("&Add to Custom Colors".into()));
    }

    fn swatch(&mut self, part: &str, bgr: i64, rect: cd::Rect) {
        let id = self.put(part, "RPANEL", rect);
        self.set(&id, "color", Value::Integer(bgr & 0xFF_FFFF));
        self.set(&id, "bevelouter", Value::Integer(rapidr_value::objects::bevel::BV_LOWERED));
    }

    /// What the colour dialog shows, from its state: the custom swatches,
    /// the frame around the chosen colour's swatch, Define's state, and the
    /// editor's field, bar, sample and boxes (but box `typing`, being
    /// typed in).
    fn show_color(&mut self, typing: Option<usize>) {
        let Kind::Color { state, .. } = &self.kind else { return };
        let state = state.clone();
        for (i, c) in state.custom.iter().enumerate() {
            let id = self.child(&format!("u{i}"));
            self.set(&id, "color", Value::Integer(*c));
        }
        let frame = state.marked().map(|(custom, i)| cl::frame(if custom { cl::custom(i) } else { cl::basic(i) }));
        let (x, y, w, h) = frame.unwrap_or((0, 0, 0, 0));
        let sel = self.child("sel");
        for (p, v) in [("left", x), ("top", y), ("width", w), ("height", h)] {
            self.set(&sel, p, Value::Integer(v));
        }
        let define = self.child("define");
        self.set(&define, "enabled", Value::Integer(-i64::from(state.mode == cd::Mode::Compact)));
        if !state.full() {
            return;
        }
        let (hue, lum, sat) = state.hls;
        for part in ["spectrum", "lum", "preview"] {
            let id = self.child(part);
            for (p, v) in [("hue", hue), ("lum", lum), ("sat", sat), ("color", state.color)] {
                self.set(&id, p, Value::Integer(v));
            }
        }
        for (i, v) in state.fields().iter().enumerate() {
            if typing != Some(i) {
                let id = self.child(&format!("f{i}"));
                self.set(&id, "text", Value::String(v.to_string()));
            }
        }
    }

    /// Changes the colour dialog's state and shows it.
    fn with_color(&mut self, typing: Option<usize>, f: impl FnOnce(&mut ColorState)) {
        if let Kind::Color { state, .. } = &mut self.kind {
            f(state);
        }
        self.show_color(typing);
    }

    /// QFONTDIALOG (`rapidr_value::font_dialog`, after Windows'
    /// ChooseFont): the faces `all` offers (fdFixedPitchOnly: the
    /// fixed-pitch ones; the chosen one added if missing), the four styles,
    /// the sizes (within Min / MaxFontSize with fdLimitSize); with
    /// fdEffects Strikeout, Underline and the colour list; a sample in the
    /// choice; OK, Cancel, Apply (fdApplyButton), Help (fdShowHelp,
    /// disabled: there's no help to show).
    pub fn font(n: u64, title: &str, req: fd::Request, all: &[String]) -> Dialog {
        use rapidr_value::font_dialog::layout as fl;
        let effects = req.has(fd::FD_EFFECTS);
        let state = FontState {
            names: req.names(all),
            sizes: req.sizes(),
            colors: req.colors(),
            unselected: [req.has(fd::FD_NO_FACE_SEL), req.has(fd::FD_NO_STYLE_SEL), req.has(fd::FD_NO_SIZE_SEL)],
            req,
        };
        let (names, sizes, colors) = (state.names.clone(), state.sizes.clone(), state.colors.clone());
        let (apply, help) = (state.req.has(fd::FD_APPLY_BUTTON), state.req.has(fd::FD_SHOW_HELP));
        let mut d = Dialog::new(n, title, Kind::Font(Box::new(state)));
        let put_caption = |d: &mut Dialog, part: &str, kind: &str, text: &str, rect: fl::Rect| {
            let id = d.put(part, kind, rect);
            d.set(&id, "caption", Value::String(text.into()));
            id
        };
        put_caption(&mut d, "lfont", "RLABEL", "&Font:", fl::FONT_LABEL);
        put_caption(&mut d, "lstyle", "RLABEL", "Font st&yle:", fl::STYLE_LABEL);
        put_caption(&mut d, "lsize", "RLABEL", "&Size:", fl::SIZE_LABEL);
        let list = |d: &mut Dialog, part: &str, kind: &str, rect: fl::Rect, items: Vec<String>| {
            let id = d.put(part, kind, rect);
            let args: Vec<Value> = items.into_iter().map(Value::String).collect();
            d.store.call(&id, "additems", &args);
            id
        };
        list(&mut d, "font", "RLISTBOX", fl::FONT_LIST, names);
        list(&mut d, "style", "RLISTBOX", fl::STYLE_LIST, fd::STYLES.iter().map(|s| s.to_string()).collect());
        list(&mut d, "size", "RLISTBOX", fl::SIZE_LIST, sizes.iter().map(|s| s.to_string()).collect());
        if effects {
            put_caption(&mut d, "effects", "RGROUPBOX", "Effects", fl::EFFECTS);
            put_caption(&mut d, "strike", "RCHECKBOX", "Stri&keout", fl::STRIKEOUT);
            put_caption(&mut d, "under", "RCHECKBOX", "&Underline", fl::UNDERLINE);
            put_caption(&mut d, "lcolor", "RLABEL", "&Color:", fl::COLOR_LABEL);
            let id = list(&mut d, "color", "RCOMBOBOX", fl::COLOR_LIST, colors.into_iter().map(|(n, _)| n).collect());
            // (csDropDownList: a pick, no typing)
            d.set(&id, "style", Value::Integer(2));
        }
        let (group, text) = fl::sample(effects);
        put_caption(&mut d, "group", "RGROUPBOX", "Sample", group);
        let sample = d.put("sample", "RDLGPART", text);
        d.set(&sample, "part", Value::String("sample".into()));
        let id = put_caption(&mut d, "ok", "RBUTTON", "OK", fl::OK);
        d.set(&id, "default", Value::Integer(-1));
        let id = put_caption(&mut d, "cancel", "RBUTTON", "Cancel", fl::CANCEL);
        d.set(&id, "cancel", Value::Integer(-1));
        if apply {
            put_caption(&mut d, "apply", "RBUTTON", "&Apply", fl::APPLY);
        }
        if help {
            let id = put_caption(&mut d, "help", "RBUTTON", "&Help", if apply { fl::HELP } else { fl::APPLY });
            d.set(&id, "enabled", Value::Integer(0));
        }
        let (w, h) = fl::size(effects);
        d.finish(w, h);
        d.show_font();
        d
    }

    /// The font dialog's lists, check boxes and colour, and the sample, from
    /// the choice.
    fn show_font(&mut self) {
        let Kind::Font(s) = &self.kind else { return };
        let font = s.req.font.clone();
        let font_index = s.names.iter().position(|n| n.eq_ignore_ascii_case(&font.name)).map_or(-1, |i| i as i64);
        // (the style list: italic 1 + bold 2; the font's bits: fsBold 0,
        // fsItalic 1)
        let style = i64::from(font.styles & 2 != 0) + 2 * i64::from(font.styles & 1 != 0);
        let size_index = s.sizes.iter().position(|v| *v == font.size).map_or(-1, |i| i as i64);
        let color_index = s.colors.iter().position(|(_, c)| *c == font.color).map_or(-1, |i| i as i64);
        let unselected = s.unselected;
        for (k, (part, index)) in [("font", font_index), ("style", style), ("size", size_index)].into_iter().enumerate() {
            let id = self.child(part);
            self.set(&id, "itemindex", Value::Integer(if unselected[k] { -1 } else { index }));
        }
        for (part, bit) in [("strike", 8), ("under", 4)] {
            let id = self.child(part);
            self.set(&id, "checked", Value::Integer(-i64::from(font.styles & bit != 0)));
        }
        let id = self.child("color");
        self.set(&id, "itemindex", Value::Integer(color_index));
        // (the sample in the font: its Font properties)
        let sample = self.child("sample");
        for (p, v) in fd::properties(&font) {
            self.set(&sample, p, v);
        }
    }

    /// The font dialog's choice from its lists, check boxes and colour (a
    /// list picked in shows its selection from now on: `picked`).
    fn read_font(&mut self, picked: Option<usize>) {
        let index = |d: &Dialog, part: &str| store::int(&d.store, &d.child(part), "itemindex", -1);
        let checked = |d: &Dialog, part: &str| store::flag(&d.store, &d.child(part), "checked", false);
        let (face, style, size, color) = (index(self, "font"), index(self, "style"), index(self, "size"), index(self, "color"));
        let (strike, under) = (checked(self, "strike"), checked(self, "under"));
        if let Kind::Font(s) = &mut self.kind {
            if let Some(k) = picked {
                s.unselected[k] = false;
            }
            let effects = s.req.has(fd::FD_EFFECTS);
            let f = &mut s.req.font;
            if let Some(n) = usize::try_from(face).ok().and_then(|i| s.names.get(i)) {
                f.name = n.clone();
            }
            // (Regular, Italic, Bold, Bold Italic: italic 1 + bold 2)
            if (0..4).contains(&style) {
                f.styles = (f.styles & !3) | u8::from(style & 2 != 0) | u8::from(style & 1 != 0) << 1;
            }
            if let Some(v) = usize::try_from(size).ok().and_then(|i| s.sizes.get(i)) {
                f.size = *v;
            }
            if effects {
                f.styles = (f.styles & 3) | u8::from(under) << 2 | u8::from(strike) << 3;
                if let Some((_, c)) = usize::try_from(color).ok().and_then(|i| s.colors.get(i)) {
                    f.color = *c;
                }
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
        match &self.kind {
            Kind::Message => Answer::Button(None),
            Kind::Color { state, .. } => Answer::Color(None, state.custom),
            Kind::Font(_) => Answer::Font(None),
        }
    }

    fn accepted(&self) -> Answer {
        match &self.kind {
            Kind::Message => Answer::Button(Some(0)),
            Kind::Color { state, .. } => Answer::Color(Some(state.color), state.custom),
            Kind::Font(s) => Answer::Font(Some(s.req.font.clone())),
        }
    }

    /// A font dialog's event (a list, a check box or the colour picked,
    /// Apply): `true` when it was one.
    fn font_event(&mut self, ev: &KernelEvent) -> bool {
        if !matches!(self.kind, Kind::Font(_)) {
            return false;
        }
        let (KernelEvent::Click(id) | KernelEvent::Change(id)) = ev else { return false };
        let Some(part) = self.part(id) else { return false };
        match part {
            "font" | "style" | "size" => {
                let k = ["font", "style", "size"].iter().position(|p| *p == part);
                self.read_font(k);
            }
            "strike" | "under" | "color" => self.read_font(None),
            "apply" if matches!(ev, KernelEvent::Click(_)) => {
                self.read_font(None);
                if let Kind::Font(s) = &self.kind {
                    self.applied = Some(s.req.font.clone());
                }
            }
            _ => return false,
        }
        true
    }

    /// A colour dialog's event (a swatch, the field or bar dragged, a box
    /// typed in, Define, Add): `true` when it was one.
    fn color_event(&mut self, ev: &KernelEvent) -> bool {
        use rapidr_value::input::Mouse;
        if !matches!(self.kind, Kind::Color { .. }) {
            return false;
        }
        match ev {
            KernelEvent::Mouse { id, kind, x, y, .. } => {
                let Some(part) = self.part(id).map(str::to_string) else { return false };
                let index = |p: &str| part.strip_prefix(p).and_then(|i| i.parse::<usize>().ok());
                let (x, y) = (*x as f64, *y as f64);
                if let Kind::Color { drag, .. } = &mut self.kind {
                    match kind {
                        Mouse::Down if matches!(part.as_str(), "spectrum" | "lum") => *drag = Some(if part == "spectrum" { "spectrum" } else { "lum" }),
                        Mouse::Up => *drag = None,
                        _ => {}
                    }
                }
                let dragging = matches!(&self.kind, Kind::Color { drag: Some(d), .. } if *d == part);
                match (kind, part.as_str()) {
                    (Mouse::Down, _) if index("c").is_some() => self.with_color(None, |s| s.pick_basic(index("c").unwrap_or(0))),
                    (Mouse::Down, _) if index("u").is_some() => self.with_color(None, |s| s.pick_custom(index("u").unwrap_or(0))),
                    // (the field's and the bar's pictures lie inside their
                    // sunken edge, as Part paints them)
                    (Mouse::Down | Mouse::Move, "spectrum") if dragging => {
                        let (w, h) = ((cl::SPECTRUM.2 - 2) as f64, (cl::SPECTRUM.3 - 2) as f64);
                        self.with_color(None, |s| s.pick_spectrum((x - 1.0).clamp(0.0, w - 1.0), (y - 1.0).clamp(0.0, h - 1.0), w, h));
                    }
                    (Mouse::Down | Mouse::Move, "lum") if dragging => {
                        let h = (cl::SPECTRUM.3 - 2) as f64;
                        self.with_color(None, |s| s.pick_lum((y - 4.0).clamp(0.0, h - 1.0), h));
                    }
                    _ => return false,
                }
                true
            }
            KernelEvent::Click(id) => match self.part(id) {
                Some("add") => {
                    self.with_color(None, ColorState::add_custom);
                    true
                }
                Some("define") => {
                    let opened = match &mut self.kind {
                        Kind::Color { state, .. } => state.define(),
                        _ => false,
                    };
                    if opened {
                        self.color_editor();
                        self.finish(cl::FULL.0, cl::FULL.1);
                        self.show_color(None);
                    }
                    true
                }
                _ => false,
            },
            KernelEvent::Change(id) => {
                let Some(i) = self.part(id).and_then(|p| p.strip_prefix('f')).and_then(|i| i.parse::<usize>().ok()) else { return false };
                let text = store::string(&self.store, id, "text");
                if let Ok(v) = text.trim().parse::<i64>() {
                    self.with_color(Some(i), |s| s.set_field(i, v));
                }
                true
            }
            _ => false,
        }
    }

    /// One of its form's events: the answer once the dialog closes. (What
    /// it changed is in its store: the form needs syncing.)
    pub fn event(&mut self, ev: &KernelEvent) -> Option<Answer> {
        if self.color_event(ev) || self.font_event(ev) {
            return None;
        }
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
                _ => None,
            },
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
/// [`MsgIcon`]'s code); the colour dialog's `spectrum` (the hue /
/// saturation field with its cross at `hue`, `sat`), `lum` (the luminance
/// bar for them, its arrow at `lum`) and `preview` (`color`).
pub struct Part;

thread_local! {
    /// The colour editor's last pictures (they change only with the scale,
    /// and the bar with the hue and saturation).
    static PICTURES: std::cell::RefCell<HashMap<&'static str, (u64, Arc<Picture>)>> = std::cell::RefCell::new(HashMap::new());
}

/// Picture `which` made by `make` (RGBA, `w` × `h`) unless the last one was
/// made for `key`.
fn picture(which: &'static str, key: u64, w: usize, h: usize, make: impl FnOnce() -> Vec<u8>) -> Arc<Picture> {
    PICTURES.with(|p| {
        let mut p = p.borrow_mut();
        match p.get(which) {
            Some((k, pic)) if *k == key => pic.clone(),
            _ => {
                let pic = Arc::new(Picture { width: w, height: h, rgba: make() });
                p.insert(which, (key, pic.clone()));
                pic
            }
        }
    })
}

impl ComponentKind for Part {
    fn name(&self) -> &'static str {
        "RDLGPART"
    }

    fn focusable(&self, _store: &dyn Store, _id: &str) -> bool {
        false
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let int = |prop: &str| store::int(cx.store, cx.id, prop, 0);
        let (w, h) = (cx.width(), cx.height());
        // (pictures at the screen's resolution)
        let scale = p.scale().max(1.0);
        let dev = |v: i64| ((v as f64 * scale).round() as usize).max(1);
        let sunken = |p: &mut Painter, r: cd::Rect| p.edge((r.0 - 1, r.1 - 1, r.2 + 2, r.3 + 2), &[SHADOW], &[LIGHT]);
        match store::string(cx.store, cx.id, "part").as_str() {
            "icon" => {
                if let Some(icon) = MsgIcon::from_code(store::int(cx.store, cx.id, "icon", -1)) {
                    for shape in icon_shapes(icon) {
                        p.shape(shape);
                    }
                }
            }
            "spectrum" => {
                let inner = (1, 1, w - 2, h - 2);
                let (pw, ph) = (dev(inner.2), dev(inner.3));
                let key = (pw * 100_000 + ph) as u64;
                let pic = picture("spectrum", key, pw, ph, || cd::spectrum_rgba(pw, ph));
                p.picture(&format!("{}#spectrum", cx.id), key, pic, inner);
                sunken(p, inner);
                // (the cross: four short bars around the hue and saturation)
                let x = 1 + (int("hue") * (inner.2 - 1)) / 239;
                let y = 1 + ((240 - int("sat")) * (inner.3 - 1)) / 240;
                p.clipped(inner, |p| {
                    for r in [(x - 8, y - 1, 5, 3), (x + 4, y - 1, 5, 3), (x - 1, y - 8, 3, 5), (x - 1, y + 4, 3, 5)] {
                        p.fill(r, 0x000000);
                    }
                });
            }
            "lum" => {
                let bar = (1, 4, cl::LUM_BAR_W - 2, cl::SPECTRUM.3 - 2);
                let (hue, sat) = (int("hue"), int("sat"));
                let (pw, ph) = (dev(bar.2), dev(bar.3));
                let key = (((hue * 241 + sat) as u64) << 32) | (pw * 100_000 + ph) as u64;
                let pic = picture("lum", key, pw, ph, || cd::lum_rgba(pw, ph, hue, sat));
                p.picture(&format!("{}#lum", cx.id), key, pic, bar);
                sunken(p, bar);
                let y = bar.1 as f64 + ((240 - int("lum")) * (bar.3 - 1)) as f64 / 240.0 + 0.5;
                let x = (cl::LUM_BAR_W + 2) as f64;
                p.op(Op::Arrow { points: [(x, y), (x + 7.0, y - 6.0), (x + 7.0, y + 6.0)], color: 0x000000 });
            }
            "preview" => {
                let inner = (1, 1, w - 2, h - 2);
                p.fill(inner, crate::text::bgr_to_rgb(int("color")));
                sunken(p, inner);
            }
            // (the font dialog's sample: its text centred in the font and
            // colour, with the underline and strikeout lines Windows draws)
            "sample" => {
                let font = cx.font.clone();
                let color = crate::text::bgr_to_rgb(font.color);
                p.text((0, 0, w, h), fd::SAMPLE, &font, color, rapidr_value::objects::ops::Place::Center);
                let (tw, th) = text_size(fd::SAMPLE, &font);
                let (left, top) = ((w - tw) / 2, (h - th) / 2);
                let px = font.pixel_size();
                let thick = (px / 14).max(1);
                // (Liberation's ascent is 0.905 em; the strikeout across the
                // lower case, about 0.3 em over the baseline)
                let base = top + (px as f64 * 0.905).round() as i64;
                if font.styles & 4 != 0 {
                    p.fill((left, base + 1, tw, thick), color);
                }
                if font.styles & 8 != 0 {
                    p.fill((left, base - (px as f64 * 0.3).round() as i64, tw, thick), color);
                }
            }
            _ => {}
        }
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = AccessNode::new(node_id(cx.id), Role::Image);
        n.name = match store::string(cx.store, cx.id, "part").as_str() {
            "icon" => MsgIcon::from_code(store::int(cx.store, cx.id, "icon", -1)).map_or("", MsgIcon::name).to_string(),
            "spectrum" => "Color matrix".into(),
            "lum" => "Luminosity".into(),
            "preview" => "Color|Solid".into(),
            "sample" => fd::SAMPLE.into(),
            _ => String::new(),
        };
        n.bounds = cx.rect;
        n
    }
}

/// The faces RapidQ programs name (every runtime's font dialog lists them:
/// `rapidr_value::font_dialog`).
pub use rapidr_value::font_dialog::FONT_NAMES;

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
        let custom = cd::DEFAULT_CUSTOM;
        let mut d = Dialog::color(3, "Color", ColorState::new(0x0000FF, custom, cd::CD_NO_FULL_OPEN));
        assert_eq!(store::int(&d.store, "rapidr:dlg3:sel", "left", -1), 6 + 2 - 2);
        assert_eq!(store::int(&d.store, "rapidr:dlg3:sel", "top", -1), 22 + 22 + 2 - 2);
        // (cdNoFullOpen: compact, Define disabled; the custom colours shown)
        assert_eq!(d.size, cl::COMPACT);
        assert_eq!(store::int(&d.store, "rapidr:dlg3:define", "enabled", 9), 0);
        assert_eq!(store::int(&d.store, "rapidr:dlg3:u2", "color", -1), 0x000080);
        let mut f = ui(&d);
        assert_eq!(click(&mut d, &mut f, &mut ts, "c26"), None);
        assert_eq!(click(&mut d, &mut f, &mut ts, "ok"), Some(Answer::Color(Some(0x008000), custom)));
        let mut f = ui(&d);
        assert_eq!(key(&mut d, &mut f, &mut ts, 27), Some(Answer::Color(None, custom)));
        // (Enter: OK)
        assert_eq!(key(&mut d, &mut f, &mut ts, 13), Some(Answer::Color(Some(0x008000), custom)));
    }

    /// A press and release at (x, y) of `part`.
    fn press_at(d: &mut Dialog, f: &mut FormUi, ts: &mut TextSystem, part: &str, (x, y): (f64, f64)) -> Option<Answer> {
        f.sync(&d.store);
        let n = f.node(&d.child(part)).expect("the part");
        let (ax, ay) = (n.abs.0 as f64 + x, n.abs.1 as f64 + y);
        f.mouse_down(&d.store, ts, ax, ay, Button::Left, Mods::NONE);
        f.mouse_up(&d.store, ts, ax, ay, Button::Left, Mods::NONE);
        answer(d, f)
    }

    #[test]
    fn color_dialog_editor() {
        let mut ts = TextSystem::new();
        // cdFullOpen: the editor shows red's HLS and RGB
        let mut d = Dialog::color(7, "Color", ColorState::new(0x0000FF, cd::DEFAULT_CUSTOM, cd::CD_FULL_OPEN));
        assert_eq!(d.size, cl::FULL);
        let boxes = |d: &Dialog| (0..6).map(|i| store::string(&d.store, &d.child(&format!("f{i}")), "text")).collect::<Vec<_>>().join(",");
        assert_eq!(boxes(&d), "0,240,120,255,0,0");
        let mut f = ui(&d);
        // the field's top right corner: hue 239, saturation 240
        let (_, _, w, _) = cl::SPECTRUM;
        assert_eq!(press_at(&mut d, &mut f, &mut ts, "spectrum", ((w - 2) as f64, 1.5)), None);
        assert_eq!(store::int(&d.store, &d.child("spectrum"), "hue", -1), 239);
        // the luminance bar's top: white
        assert_eq!(press_at(&mut d, &mut f, &mut ts, "lum", (4.5, 4.5)), None);
        assert_eq!(boxes(&d), "239,240,240,255,255,255");
        // Blue typed 0: yellow
        let blue = d.child("f5");
        d.store.set(&blue, "text", Value::String("0".into()));
        assert_eq!(d.event(&KernelEvent::Change(blue)), None);
        assert_eq!(store::int(&d.store, &d.child("preview"), "color", -1), 0x00FFFF);
        // Add to Custom Colors: the first custom box; Cancel keeps it
        assert_eq!(click(&mut d, &mut f, &mut ts, "add"), None);
        assert_eq!(store::int(&d.store, &d.child("u0"), "color", -1), 0x00FFFF);
        match click(&mut d, &mut f, &mut ts, "cancel") {
            Some(Answer::Color(None, custom)) => assert_eq!(custom[0], 0x00FFFF),
            other => panic!("{other:?}"),
        }
        // neither flag (Style 3): compact, Define opens the editor
        let mut d = Dialog::color(8, "Color", ColorState::new(0, cd::DEFAULT_CUSTOM, 3));
        assert_eq!((d.size, store::int(&d.store, &d.child("define"), "enabled", 9)), (cl::COMPACT, -1));
        let mut f = ui(&d);
        assert_eq!(click(&mut d, &mut f, &mut ts, "define"), None);
        assert_eq!(d.size, cl::FULL);
        assert_eq!(store::int(&d.store, &d.child("define"), "enabled", 9), 0);
        assert_eq!(store::string(&d.store, &d.child("f0"), "text"), "160");
        // its parts drawn: the field's and bar's pictures, the arrow
        let mut f = ui(&d);
        let list = f.paint(&d.store, &mut ts, 2.0);
        let dump = list.dump();
        assert!(dump.contains("#spectrum") && dump.contains("#lum") && dump.contains("arrow #000000"), "{dump}");
        assert_eq!(list.images.values().map(|p| p.width).max(), Some(2 * (cl::SPECTRUM.2 - 2) as usize));
    }

    #[test]
    fn font_dialog_lists_and_sample() {
        let mut ts = TextSystem::new();
        let all: Vec<String> = FONT_NAMES.iter().map(|s| s.to_string()).collect();
        let req = fd::request(&|_| Value::Null);
        let chosen = req.font.clone();
        let mut d = Dialog::font(4, "Font", req.clone(), &all);
        assert_eq!(store::int(&d.store, "rapidr:dlg4:size", "itemindex", -1), 2);
        assert_eq!(store::int(&d.store, "rapidr:dlg4:font", "itemindex", -1), 0);
        // (fdEffects by default: the effects, the colour Black; no Apply)
        assert_eq!(d.size, fd::layout::size(true));
        assert_eq!(store::int(&d.store, "rapidr:dlg4:color", "itemindex", -1), 0);
        assert!(!d.store.ids().contains(&"rapidr:dlg4:apply".to_string()));
        // the style list's "Bold" row, picked by the mouse
        let mut f = ui(&d);
        f.sync(&d.store);
        let n = f.node("rapidr:dlg4:style").expect("the style list");
        let (x, y) = (n.abs.0 as f64 + 10.0, n.abs.1 as f64 + 2.0 + 2.5 * 16.0);
        f.mouse_down(&d.store, &mut ts, x, y, Button::Left, Mods::NONE);
        f.mouse_up(&d.store, &mut ts, x, y, Button::Left, Mods::NONE);
        assert_eq!(answer(&mut d, &mut f), None);
        assert_eq!(store::int(&d.store, "rapidr:dlg4:sample", "fontbold", 0), -1);
        // Underline checked
        assert_eq!(click(&mut d, &mut f, &mut ts, "under"), None);
        assert_eq!(store::int(&d.store, "rapidr:dlg4:sample", "fontunderline", 0), -1);
        let picked = click(&mut d, &mut f, &mut ts, "ok");
        assert_eq!(picked, Some(Answer::Font(Some(Font { styles: 0b101, ..chosen.clone() }))));
        // (a font the list hasn't: added)
        let mut other = req.clone();
        other.font.name = "Comic Sans MS".into();
        let d2 = Dialog::font(5, "Font", other, &all);
        assert_eq!(store::int(&d2.store, "rapidr:dlg5:font", "itemindex", -1), 1);
    }

    #[test]
    fn font_dialog_options() {
        let mut ts = TextSystem::new();
        let all: Vec<String> = FONT_NAMES.iter().map(|s| s.to_string()).collect();
        // no effects; Apply and Help; sizes 10 to 14; no size selected
        let mut req = fd::request(&|_| Value::Null);
        req.options = 1 << fd::FD_APPLY_BUTTON | 1 << fd::FD_SHOW_HELP | 1 << fd::FD_LIMIT_SIZE | 1 << fd::FD_NO_SIZE_SEL;
        req.min = 10;
        req.max = 14;
        req.font.color = 0x0000FF;
        let mut d = Dialog::font(9, "Font", req, &all);
        assert_eq!(d.size, fd::layout::size(false));
        assert!(!d.store.ids().contains(&d.child("under")));
        assert_eq!(store::int(&d.store, &d.child("size"), "itemindex", 0), -1);
        assert_eq!(store::int(&d.store, &d.child("help"), "enabled", 9), 0);
        assert_eq!(rapidr_value::objects::get(&d.child("size"), "itemcount").map(|v| v.to_i64()), Some(4));
        // Apply: the font so far for the program's OnApply, the dialog open
        let mut f = ui(&d);
        assert_eq!(click(&mut d, &mut f, &mut ts, "apply"), None);
        let applied = d.take_applied().expect("applied");
        assert_eq!((applied.size, applied.color), (10, 0x0000FF));
        assert_eq!(d.take_applied(), None);
        // the sample draws its text with the lines when underlined
        let mut req = fd::request(&|_| Value::Null);
        req.font.styles = 4 | 8;
        let d = Dialog::font(10, "Font", req, &all);
        let mut f = ui(&d);
        let list = f.paint(&d.store, &mut ts, 1.0);
        let n = f.node(&d.child("sample")).expect("the sample").abs;
        use rapidr_value::objects::ops::Op as O;
        let at_sample = list.items.iter().filter(|i| matches!(i, crate::display::Item::Op { origin, op: O::Text { .. } | O::Fill { .. } } if *origin == (n.0, n.1))).count();
        // (its text and two lines)
        assert_eq!(at_sample, 3, "{}", list.dump());
    }
}
