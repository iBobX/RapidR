//! Dialogs the kernel draws itself, as forms in a [`MemStore`], from the
//! shared models every runtime shows them from: the MESSAGEBOX /
//! MESSAGEDLG / SHOWMESSAGE box (`rapidr_value::dialogs`: its icon, text
//! and buttons, the first one the default), QCOLORDIALOG
//! (`rapidr_value::color_dialog`: the basic and custom colours, the custom
//! colour editor) and QFONTDIALOG (`rapidr_value::font_dialog`: the face,
//! style and size lists, the effects, a sample) — the system has no colour
//! or font dialog the host could show without blocking (rfd has neither).
//!
//! A dialog is an ordinary kernel form: the host shows its window, routes
//! input into it (Tab, Enter on the default button, Escape, mnemonics,
//! screen readers) and queues its [`KernelEvent`]s; runtime-core hands them
//! to [`Dialog::event`] instead of the program, and steps until it answers.
//! Its components' ids start with [`PREFIX`], which no BASIC name can.
//! What only a dialog draws (a message's icon, the colour editor's field
//! and bar, the font's sample) is a [`Part`] (`RDLGPART`, a type no program
//! can make).

use std::collections::HashMap;
use std::sync::Arc;

use rapidr_value::color_dialog::{self as cd, layout as cl, State as ColorState};
use rapidr_value::dialogs::{icon_shapes, message_layout, MsgIcon};
use rapidr_value::font_dialog as fd;
use rapidr_value::i18n::tr_ui;
use rapidr_value::objects::a11y::{node_id, AccessNode, Role};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::Op;
use rapidr_value::objects::text::text_size;
use rapidr_value::Value;

use crate::components::{ComponentKind, Cx};
use crate::display::Picture;
use crate::input::KernelEvent;
use crate::paint::Painter;
use crate::store::{self, MemStore, Store};

/// What every kernel dialog's ids start with (`:` can't be in a BASIC name).
pub const PREFIX: &str = "rapidr:";

/// Whether `id` is (a component of) a kernel-drawn dialog.
pub fn is_dialog(id: &str) -> bool {
    id.len() >= PREFIX.len() && id[..PREFIX.len()].eq_ignore_ascii_case(PREFIX)
}

/// A message box's form property holding what Ctrl+C copies.
const COPY_PROP: &str = "__copytext";

/// What a message box copies on Ctrl+C (⌘C on a Mac, Ctrl+Insert), as
/// Windows' own message boxes give it: the title, the text and the
/// buttons between dashed lines.
pub fn message_copy(title: &str, text: &str, labels: &[&str]) -> String {
    let nl = if cfg!(windows) { "\r\n" } else { "\n" };
    let rule = "---------------------------";
    let text = text.split('\0').next().unwrap_or("").replace("\r\n", "\n").replace('\r', "\n").replace('\n', nl);
    let buttons: String = labels.iter().map(|l| format!("{}   ", l.replace('&', ""))).collect();
    [rule, title, rule, &text, rule, &buttons, rule].join(nl) + nl
}

/// What a dialog form (`form`) copies on Ctrl+C: a message box's text
/// (`None`: another dialog, or not one of the kernel's).
pub fn copy_text(store: &dyn Store, form: &str) -> Option<String> {
    if !is_dialog(form) {
        return None;
    }
    let t = store::string(store, form, COPY_PROP);
    (!t.is_empty()).then_some(t)
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
    /// An input box's text (`None`: closed or Escape).
    Text(Option<String>),
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
    /// A prompt with a text field (the web's INPUT in a program with
    /// windows: no console to type in).
    Input,
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
        // (the theme's chrome font, its components following it: Windows'
        // in the classic look, Inter in RapidR's — a dialog is the
        // system's, not the program's)
        let font = rapidr_value::ide_theme::chrome_font(rapidr_value::theme::current());
        store.set(&id, "fontname", Value::String(font.name.into()));
        store.set(&id, "fontsize", Value::Integer(font.size));
        Dialog { id, title: title.to_string(), size: (0, 0), store, kind, applied: None }
    }

    /// A font dialog's Apply, since the last call: the font then.
    pub fn take_applied(&mut self) -> Option<Font> {
        self.applied.take()
    }

    /// The narrowest inside this dialog's window can have: what its title
    /// needs, so a short text with one button doesn't cut "Information"
    /// off (`frame::inside_for_title`, for the frame every dialog has: a
    /// close box only).
    fn title_width(&self) -> i64 {
        crate::frame::inside_for_title(&self.title, crate::frame::Frame { resizable: false, close: true, minimize: false, maximize: false })
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
        self.set(&id, "caption", Value::String(self.title.clone().into()));
    }

    /// MESSAGEBOX / MESSAGEDLG / SHOWMESSAGE: `icon` (if any) at the top
    /// left, `text` right of it, `labels`' buttons centred under both (the
    /// first the default: Enter), titled `title` — RapidQ's MessageDlg
    /// layout (`rapidr_value::dialogs::message_layout`), as every runtime
    /// lays it out.
    pub fn message(n: u64, title: &str, text: &str, labels: &[&str], icon: Option<MsgIcon>) -> Dialog {
        let mut d = Dialog::new(n, title, Kind::Message);
        let font = d.store.font(&d.id);
        // (the text ends at a NUL, as Windows' message boxes end it: RapidQ's
        // Database/mdb examples show a path made for a DLL, `… + CHR$(0)`)
        let text = text.split('\0').next().unwrap_or("");
        let lines = wrap(text, &font, WRAP);
        let line_h = text_size("Ag", &font).1.max(1);
        let text_w = lines.iter().map(|l| text_size(l, &font).0).max().unwrap_or(0);
        let layout = message_layout(text_w, lines.len() as i64 * line_h, labels.len(), icon.is_some(), d.title_width());
        if let (Some(icon), Some(rect)) = (icon, layout.icon) {
            let id = d.put("icon", "RDLGPART", rect);
            d.set(&id, "part", Value::String("icon".into()));
            d.set(&id, "icon", Value::Integer(icon.code()));
        }
        let (tx, ty, _, _) = layout.text;
        for (i, line) in lines.iter().enumerate() {
            let id = d.put(&format!("t{i}"), "RLABEL", (tx, ty + i as i64 * line_h, text_w.max(1) + 2, line_h));
            d.set(&id, "caption", Value::String(literal(line).into()));
        }
        for (i, (label, rect)) in labels.iter().zip(&layout.buttons).enumerate() {
            let id = d.put(&format!("b{i}"), "RBUTTON", *rect);
            d.set(&id, "caption", Value::String(button_caption(label).into()));
            if i == 0 {
                d.set(&id, "default", Value::Integer(-1));
            }
        }
        // (Ctrl+C — ⌘C — copies the box as Windows' message boxes do:
        // `copy_text`)
        let id = d.id.clone();
        d.set(&id, COPY_PROP, Value::String(message_copy(title, text, labels).into()));
        d.finish(layout.size.0, layout.size.1);
        d
    }

    /// A message box whose button `i` is the default (RapidR's MESSAGEDLG
    /// DefaultButton, MESSAGEBOX's MB_DEFBUTTONn): Enter presses it, and it
    /// has the focus as the box opens, as Windows' message boxes give it.
    /// Tab still walks the buttons left to right, round from it. Escape
    /// and the close box are as they were. A button that isn't there: the
    /// first stays the default.
    pub fn with_default(mut self, i: usize) -> Dialog {
        let buttons: Vec<String> = (0..).map(|j| self.child(&format!("b{j}"))).take_while(|id| self.store.ids().contains(id)).collect();
        let n = buttons.len();
        if i >= n || !matches!(self.kind, Kind::Message) {
            return self;
        }
        for (j, id) in buttons.iter().enumerate() {
            self.set(id, "default", Value::Integer(if j == i { -1 } else { 0 }));
            // (the focus starts at the default: the Tab order starts there
            // and goes round, the cycle Windows' left-to-right one)
            self.set(id, "taborder", Value::Integer(((j + n - i) % n) as i64));
        }
        self
    }

    /// An input box: `text` (the prompt) over a text field holding
    /// `initial`, an OK button under them (the default: Enter in the field
    /// answers), titled `title` — a message box's layout with the field
    /// under the text.
    pub fn input(n: u64, title: &str, text: &str, initial: &str) -> Dialog {
        Dialog::prompt(n, title, text, initial, &tr_ui("OK"), None)
    }

    /// An input box whose default button is captioned `ok` ("&Save"),
    /// with a Cancel button captioned `cancel` beside it when there's one
    /// (Escape and the close box cancel either way).
    pub fn prompt(n: u64, title: &str, text: &str, initial: &str, ok: &str, cancel: Option<&str>) -> Dialog {
        const FIELD_W: i64 = 240;
        const FIELD_H: i64 = 21;
        const GAP: i64 = 8;
        let mut d = Dialog::new(n, title, Kind::Input);
        let font = d.store.font(&d.id);
        let lines = wrap(text, &font, WRAP);
        let line_h = text_size("Ag", &font).1.max(1);
        let text_w = lines.iter().map(|l| text_size(l, &font).0).max().unwrap_or(0).max(FIELD_W);
        let text_h = lines.len() as i64 * line_h;
        let layout = message_layout(text_w, text_h + GAP + FIELD_H, 1 + usize::from(cancel.is_some()), false, d.title_width());
        let (tx, ty, _, _) = layout.text;
        for (i, line) in lines.iter().enumerate() {
            let id = d.put(&format!("t{i}"), "RLABEL", (tx, ty + i as i64 * line_h, text_w + 2, line_h));
            d.set(&id, "caption", Value::String(literal(line).into()));
        }
        let field = d.put("field", "REDIT", (tx, ty + text_h + GAP, text_w, FIELD_H));
        d.set(&field, "text", Value::String(initial.to_string().into()));
        // (the proposed text selected, as Windows' boxes have it: typing
        // replaces it)
        d.set(&field, "selstart", Value::Integer(0));
        d.set(&field, "sellength", Value::Integer(initial.chars().count() as i64));
        d.set(&field, "accessiblename", Value::String(lines.join(" ").into()));
        if let Some(rect) = layout.buttons.first() {
            let id = d.put("ok", "RBUTTON", *rect);
            d.set(&id, "caption", Value::String(ok.into()));
            d.set(&id, "default", Value::Integer(-1));
        }
        if let (Some(caption), Some(rect)) = (cancel, layout.buttons.get(1)) {
            let id = d.put("cancel", "RBUTTON", *rect);
            d.set(&id, "caption", Value::String(caption.into()));
            d.set(&id, "cancel", Value::Integer(-1));
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
        label(&mut d, "basic", &tr_ui("&Basic colors:"), cl::BASIC_LABEL);
        // (the selection's frame, under the swatches: drawn first)
        let sel = d.put("sel", "RPANEL", (0, 0, 0, 0));
        d.set(&sel, "bevelouter", Value::Integer(0));
        d.set(&sel, "color", Value::Integer(0));
        for (i, rgb) in cd::BASIC_COLORS.iter().enumerate() {
            d.swatch(&format!("c{i}"), cd::swap_rb(i64::from(*rgb)), cl::basic(i));
        }
        label(&mut d, "custom", &tr_ui("&Custom colors:"), cl::CUSTOM_LABEL);
        for i in 0..16 {
            d.swatch(&format!("u{i}"), 0, cl::custom(i));
        }
        let define = d.put("define", "RBUTTON", cl::DEFINE);
        d.set(&define, "caption", Value::String(tr_ui("&Define Custom Colors >>").into()));
        let id = d.put("ok", "RBUTTON", cl::OK);
        d.set(&id, "caption", Value::String(tr_ui("OK").into()));
        d.set(&id, "default", Value::Integer(-1));
        let id = d.put("cancel", "RBUTTON", cl::CANCEL);
        d.set(&id, "caption", Value::String(tr_ui("Cancel").into()));
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
        self.set(&id, "caption", Value::String(tr_ui("Color|S&olid").into()));
        self.set(&id, "alignment", Value::Integer(2));
        for (i, caption) in cd::FIELDS.iter().enumerate() {
            let (label, edit) = cl::field(i);
            let id = self.put(&format!("l{i}"), "RLABEL", label);
            self.set(&id, "caption", Value::String(caption.to_string().into()));
            let id = self.put(&format!("f{i}"), "REDIT", edit);
            self.set(&id, "maxlength", Value::Integer(3));
        }
        let id = self.put("add", "RBUTTON", cl::ADD);
        self.set(&id, "caption", Value::String(tr_ui("&Add to Custom Colors").into()));
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
                self.set(&id, "text", Value::String(v.to_string().into()));
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
        put_caption(&mut d, "lfont", "RLABEL", &tr_ui("&Font:"), fl::FONT_LABEL);
        put_caption(&mut d, "lstyle", "RLABEL", &tr_ui("Font st&yle:"), fl::STYLE_LABEL);
        put_caption(&mut d, "lsize", "RLABEL", &tr_ui("&Size:"), fl::SIZE_LABEL);
        let list = |d: &mut Dialog, part: &str, kind: &str, rect: fl::Rect, items: Vec<String>| {
            let id = d.put(part, kind, rect);
            let args: Vec<Value> = items.into_iter().map(Value::from).collect();
            d.store.call(&id, "additems", &args);
            id
        };
        list(&mut d, "font", "RLISTBOX", fl::FONT_LIST, names);
        list(&mut d, "style", "RLISTBOX", fl::STYLE_LIST, fd::STYLES.iter().map(|s| s.to_string()).collect());
        list(&mut d, "size", "RLISTBOX", fl::SIZE_LIST, sizes.iter().map(|s| s.to_string()).collect());
        if effects {
            put_caption(&mut d, "effects", "RGROUPBOX", &tr_ui("Effects"), fl::EFFECTS);
            put_caption(&mut d, "strike", "RCHECKBOX", &tr_ui("Stri&keout"), fl::STRIKEOUT);
            put_caption(&mut d, "under", "RCHECKBOX", &tr_ui("&Underline"), fl::UNDERLINE);
            put_caption(&mut d, "lcolor", "RLABEL", &tr_ui("&Color:"), fl::COLOR_LABEL);
            let id = list(&mut d, "color", "RCOMBOBOX", fl::COLOR_LIST, colors.into_iter().map(|(n, _)| n).collect());
            // (csDropDownList: a pick, no typing)
            d.set(&id, "style", Value::Integer(2));
        }
        let (group, text) = fl::sample(effects);
        put_caption(&mut d, "group", "RGROUPBOX", &tr_ui("Sample"), group);
        let sample = d.put("sample", "RDLGPART", text);
        d.set(&sample, "part", Value::String("sample".into()));
        let id = put_caption(&mut d, "ok", "RBUTTON", &tr_ui("OK"), fl::OK);
        d.set(&id, "default", Value::Integer(-1));
        let id = put_caption(&mut d, "cancel", "RBUTTON", &tr_ui("Cancel"), fl::CANCEL);
        d.set(&id, "cancel", Value::Integer(-1));
        if apply {
            put_caption(&mut d, "apply", "RBUTTON", &tr_ui("&Apply"), fl::APPLY);
        }
        if help {
            let id = put_caption(&mut d, "help", "RBUTTON", &tr_ui("&Help"), if apply { fl::HELP } else { fl::APPLY });
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
        // (a system colour kept: the sample in the theme's ink for it)
        let ink = s.req.system.filter(|sc| sc.shown == font.color).map(|sc| sc.ink);
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
        for (p, v) in fd::properties(&Font { color: ink.unwrap_or(font.color), ..font }) {
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
            Kind::Input => Answer::Text(None),
            Kind::Color { state, .. } => Answer::Color(None, state.custom),
            Kind::Font(_) => Answer::Font(None),
        }
    }

    fn accepted(&self) -> Answer {
        match &self.kind {
            Kind::Message => Answer::Button(Some(0)),
            Kind::Input => Answer::Text(Some(self.store.get(&self.child("field"), "text").to_string_val())),
            Kind::Color { state, .. } => Answer::Color(Some(state.color), state.custom),
            Kind::Font(s) => Answer::Font(Some(s.req.answer())),
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
                    self.applied = Some(s.req.answer());
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
        let t = p.theme();
        let sunken = |p: &mut Painter, r: cd::Rect| p.thin_sunken((r.0 - 1, r.1 - 1, r.2 + 2, r.3 + 2));
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
                        p.fill(r, t.text);
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
                p.op(Op::Arrow { points: [(x, y), (x + 7.0, y - 6.0), (x + 7.0, y + 6.0)], color: t.text });
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
            "spectrum" => tr_ui("Color matrix"),
            "lum" => tr_ui("Luminosity"),
            "preview" => tr_ui("Color|Solid"),
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

    /// (COPY-ANYWHERE) Ctrl+C (⌘C, Ctrl+Insert) on a message box copies
    /// it as Windows' message boxes do — the title, the whole text (not as
    /// it wraps), the buttons — and leaves the box open; an input box's
    /// field copies its own selection instead.
    #[test]
    fn ctrl_c_copies_a_message_box_as_windows_does() {
        let mut ts = TextSystem::new();
        let long = "Can't open data.csv: the file is in use by another program. Close it and try again.";
        let mut d = Dialog::message(41, "Error", long, &["OK", "Cancel"], None);
        let mut f = ui(&d);
        let nl = if cfg!(windows) { "\r\n" } else { "\n" };
        let want = ["---------------------------", "Error", "---------------------------", long, "---------------------------", "OK   Cancel   ", "---------------------------", ""].join(nl);
        for mods in [Mods { ctrl: true, ..Mods::NONE }, Mods { command: true, ..Mods::NONE }] {
            let mut clip = MemClipboard::default();
            f.key_down(&d.store, &mut ts, 67, "", mods, &mut clip);
            f.key_up(67, mods);
            assert_eq!(clip.0.as_deref(), Some(want.as_str()));
            assert_eq!(answer(&mut d, &mut f), None, "the box stays open");
        }
        let mut clip = MemClipboard::default();
        f.key_down(&d.store, &mut ts, 45, "", Mods { ctrl: true, ..Mods::NONE }, &mut clip);
        assert_eq!(clip.0.as_deref(), Some(want.as_str()));
        // (a plain C: nothing copied)
        let mut clip = MemClipboard::default();
        f.key_down(&d.store, &mut ts, 67, "c", Mods::NONE, &mut clip);
        assert_eq!(clip.0, None);
        // (a mnemonic's & isn't copied; Yes / No's are RapidR's)
        let d2 = Dialog::message(42, "Question", "Save?", &["Yes", "No"], None);
        assert!(copy_text(&d2.store, &d2.id).unwrap().contains("Yes   No   "));
        // (an input box isn't copied whole: its field copies its own text)
        let d3 = Dialog::input(43, "Name", "Your name:", "Robert");
        assert_eq!(copy_text(&d3.store, &d3.id), None);
    }

    #[test]
    fn a_default_button_takes_enter_and_the_focus_tab_goes_round() {
        let mut ts = TextSystem::new();
        // (MESSAGEDLG's DefaultButton mrNo: "No" of Yes / No / Cancel)
        let mut d = Dialog::message(31, "Warning", "This deletes 1,204 rows.", &["Yes", "No", "Cancel"], None).with_default(1);
        assert!(!store::flag(&d.store, "rapidr:dlg31:b0", "default", false));
        assert!(store::flag(&d.store, "rapidr:dlg31:b1", "default", false));
        // Enter as the box opens: No
        let mut f = ui(&d);
        assert_eq!(f.focus.map(|i| f.nodes[i].id.clone()).as_deref(), Some("rapidr:dlg31:b1"));
        assert_eq!(key(&mut d, &mut f, &mut ts, 13), Some(Answer::Button(Some(1))));
        // Tab: Cancel, then Yes (left to right, round), Enter presses the
        // focused one
        let mut f = ui(&d);
        assert_eq!(key(&mut d, &mut f, &mut ts, 9), None);
        assert_eq!(f.focus.map(|i| f.nodes[i].id.clone()).as_deref(), Some("rapidr:dlg31:b2"));
        assert_eq!(key(&mut d, &mut f, &mut ts, 9), None);
        assert_eq!(f.focus.map(|i| f.nodes[i].id.clone()).as_deref(), Some("rapidr:dlg31:b0"));
        assert_eq!(key(&mut d, &mut f, &mut ts, 13), Some(Answer::Button(Some(0))));
        // Escape: as always (the runtime maps None to Cancel / No)
        let mut f = ui(&d);
        assert_eq!(key(&mut d, &mut f, &mut ts, 27), Some(Answer::Button(None)));
        // (Yes / No: No; a button that isn't there leaves the first)
        let mut d2 = Dialog::message(32, "Confirm", "Let AI apps read the rows?", &["Yes", "No"], None).with_default(1);
        let mut f2 = ui(&d2);
        assert_eq!(key(&mut d2, &mut f2, &mut ts, 13), Some(Answer::Button(Some(1))));
        let mut d3 = Dialog::message(33, "Confirm", "?", &["Yes", "No"], None).with_default(5);
        let mut f3 = ui(&d3);
        assert_eq!(key(&mut d3, &mut f3, &mut ts, 13), Some(Answer::Button(Some(0))));
    }

    #[test]
    fn prompt_with_cancel_selects_the_proposed_text() {
        let mut ts = TextSystem::new();
        let mut d = Dialog::prompt(11, "Save As", "File name:", "notes.txt", "&Save", Some("Cancel"));
        let field = "rapidr:dlg11:field";
        assert_eq!(store::string(&d.store, field, "text"), "notes.txt");
        // (selected whole: typing replaces it)
        assert_eq!((store::int(&d.store, field, "selstart", -1), store::int(&d.store, field, "sellength", -1)), (0, 9));
        assert_eq!(store::string(&d.store, "rapidr:dlg11:ok", "caption"), "&Save");
        assert!(store::flag(&d.store, "rapidr:dlg11:cancel", "cancel", false));
        let mut f = ui(&d);
        assert_eq!(click(&mut d, &mut f, &mut ts, "cancel"), Some(Answer::Text(None)));
        let mut f = ui(&d);
        assert_eq!(click(&mut d, &mut f, &mut ts, "ok"), Some(Answer::Text(Some("notes.txt".into()))));
        // (INPUT's box: OK only)
        let d2 = Dialog::input(12, "", "Name?", "");
        assert!(!d2.store.ids().contains(&"rapidr:dlg12:cancel".to_string()));
    }

    #[test]
    fn message_box_icon_left_of_the_text() {
        let mut ts = TextSystem::new();
        let d = Dialog::message(6, "Warning", "Disk full", &["OK"], Some(MsgIcon::Warning));
        let int = |id: &str, p: &str| store::int(&d.store, id, p, -1);
        // the icon at RapidQ's margins, the text 15 right of it, the button
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

    /// A short text with one button is narrower than its title's room: the
    /// window takes the title's width, in every look (RapidR's light, dark
    /// and high contrast, the classic one), and the frame the kernel draws
    /// shows the whole title, never the clipped "Informa". Wider text or
    /// more buttons keep their own width.
    #[test]
    fn a_dialog_is_as_wide_as_its_title_needs() {
        use crate::display::{Item, Picture};
        use crate::frame::{self, Frame, Look};
        use rapidr_value::theme;
        let close_only = Frame { resizable: false, close: true, minimize: false, maximize: false };
        let was = theme::current();
        for t in [&theme::CLASSIC, &theme::RAPIDR, &theme::RAPIDR_DARK, &theme::RAPIDR_HIGH_CONTRAST] {
            theme::set(t);
            let needs = frame::inside_for_title("Information", close_only);
            // (the buttons and a short text alone make 99 pixels)
            let d = Dialog::message(21, "Information", "Saved.", &["OK"], None);
            assert!(needs > 99, "{}: {needs}", t.name);
            assert_eq!(d.size.0, needs, "{}", t.name);
            // (with an icon, MessageDlg's mtInformation: at least that)
            let with_icon = Dialog::message(25, "Information", "Saved.", &["OK"], Some(MsgIcon::Information));
            assert!(with_icon.size.0 >= needs, "{}: {:?}", t.name, with_icon.size);
            // (the frame the kernel draws, with the application's icon, gives the title the room it measures)
            let icon = Arc::new(Picture { width: 16, height: 16, rgba: vec![0; 16 * 16 * 4] });
            let look = Look { title: "Information".into(), active: true, border: true, caption: true, frame: close_only, maximized: false, icon: Some((icon, 1)) };
            let (w, h) = frame::outer(d.size, true, true);
            let list = frame::paint(&look, (w, h), 1.0);
            let title = list.items.iter().find_map(|i| match i {
                Item::Op { op: Op::Text { rect, text, font, .. }, .. } if text == "Information" => Some((rect.2, text_size(text, font).0)),
                _ => None,
            });
            let (room, wanted) = title.expect("the title is drawn");
            assert!(room >= wanted, "{}: the title's room {room} < its width {wanted}", t.name);
            // (a long text, or three buttons, make it wider than the title needs: as before)
            let wide = Dialog::message(22, "Information", "A message long enough to be wider than its title needs.", &["OK"], None);
            assert!(wide.size.0 > d.size.0);
            let three = Dialog::message(23, "Q", "?", &["Yes", "No", "Cancel"], None);
            assert_eq!(three.size.0, 3 * 75 + 2 * 6 + 24, "{}", t.name);
            // (INPUT's box is wide already; a title of its own length widens it)
            let long = "A title longer than the input box's field is wide, to be sure";
            let p = Dialog::prompt(24, long, "Name?", "", "OK", None);
            assert_eq!(p.size.0, frame::inside_for_title(long, close_only).max(240 + 24), "{}", t.name);
        }
        theme::set(was);
    }

    #[test]
    fn message_text_ends_at_a_nul() {
        // (as Windows' message boxes: RC.EXE shows "Error al crear Test.mdb"
        // of `"Error al crear Test.mdb" + CHR$(0) + "hidden"`)
        let mut ts = TextSystem::new();
        let d = Dialog::message(7, "x", "Error al crear Test.mdb\0hidden", &["OK"], None);
        assert_eq!(store::string(&d.store, "rapidr:dlg7:t0", "caption"), "Error al crear Test.mdb");
        let mut f = ui(&d);
        let list = f.paint(&d.store, &mut ts, 1.0);
        let texts: Vec<String> = list.items.iter().filter_map(|i| match i {
            crate::display::Item::Op { op: rapidr_value::objects::ops::Op::Text { text, .. }, .. } => Some(text.clone()),
            _ => None,
        }).collect();
        assert!(texts.iter().all(|t| !t.contains('\0') && !t.contains("hidden")), "{texts:?}");
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
        // (RapidQ's look, checked op for op: the classic theme, named)
        rapidr_value::theme::set(&rapidr_value::theme::CLASSIC);
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
        // (RapidQ's default font: MS Sans Serif 8)
        assert_eq!(store::int(&d.store, "rapidr:dlg4:size", "itemindex", -1), 0);
        assert_eq!(store::int(&d.store, "rapidr:dlg4:font", "itemindex", -1), 3);
        // (fdEffects by default: the effects, the colour Black; no Apply)
        assert_eq!(d.size, fd::layout::size(true));
        assert_eq!(store::int(&d.store, "rapidr:dlg4:color", "itemindex", -1), 0);
        assert!(!d.store.ids().contains(&"rapidr:dlg4:apply".to_string()));
        // the style list's "Bold" row, picked by the mouse
        let mut f = ui(&d);
        f.sync(&d.store);
        let n = f.node("rapidr:dlg4:style").expect("the style list");
        let (x, y) = (n.abs.0 as f64 + 10.0, n.abs.1 as f64 + 2.0 + 2.5 * 13.0);
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
        // (Color clWindowText, as a new dialog's: Black in the list, no
        // Custom; kept, OK answers clWindowText; another picked, that one)
        let sys = fd::request(&|p| if p == "color" { Value::Integer(rapidr_value::component_defaults::CL_WINDOW_TEXT) } else { Value::Null });
        let mut d3 = Dialog::font(6, "Font", sys, &all);
        assert_eq!(store::int(&d3.store, "rapidr:dlg6:color", "itemindex", -1), 0);
        assert_eq!(rapidr_value::objects::get("rapidr:dlg6:color", "itemcount").map(|v| v.to_i64()), Some(16));
        let mut f3 = ui(&d3);
        assert_eq!(click(&mut d3, &mut f3, &mut ts, "ok"), Some(Answer::Font(Some(Font { color: rapidr_value::component_defaults::CL_WINDOW_TEXT, ..chosen.clone() }))));
        let sys = fd::request(&|p| if p == "color" { Value::Integer(rapidr_value::component_defaults::CL_WINDOW_TEXT) } else { Value::Null });
        let mut d4 = Dialog::font(7, "Font", sys, &all);
        d4.set("rapidr:dlg7:color", "itemindex", Value::Integer(9));
        let mut f4 = ui(&d4);
        assert!(d4.font_event(&KernelEvent::Change("rapidr:dlg7:color".into())));
        assert_eq!(click(&mut d4, &mut f4, &mut ts, "ok"), Some(Answer::Font(Some(Font { color: 0x0000FF, ..chosen.clone() }))));
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
        assert_eq!((applied.size, applied.color), (8, 0x0000FF));
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
