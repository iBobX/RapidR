//! QEDIT, and what every kernel-drawn text box shares (QMEMO and QRICHEDIT
//! in `memo.rs`, an editable QCOMBOBOX's box): an [`EditUi`] — the
//! kernel's [`TextEditor`] (one parley layout per paragraph) showing a
//! model, scrolled to keep the caret in view.
//!
//! - **The model is the program's.** QEDIT, QMEMO and QRICHEDIT keep their
//!   text and selection in the shared `TextEdit` (what the program reads as
//!   Text, SelStart, SelLength, Modified, Line, WhereX …); a combo its
//!   `ItemList`'s Text. When the program changed it (its `revision`
//!   moved) the editor shows it again; what the user types or selects goes
//!   into it at once (`TextEdit::user_edit`), then OnChange when the text
//!   changed. runtime-core's `text_push` / `text_pull` are no-ops.
//! - **Windows' edit control**: ReadOnly (the caret moves, nothing types),
//!   MaxLength, CharCase, PasswordChar (the layout shows its character, the
//!   model keeps the text; no copying it out), Alignment, HideSelection
//!   (True: the selection shows only while focused), a double click selects
//!   a word and a triple click everything (a memo: the paragraph), Ctrl+Z
//!   undoes the last edit (and undoes the undo), Ctrl / Shift + Insert /
//!   Delete, and the right-click menu — Undo, Cut, Copy, Paste, Delete,
//!   Select All — drawn by the kernel (`menubar.rs`'s panels), unless the
//!   program gave the control a PopupMenu. The clipboard is the runtime's
//!   (`globals::Platform`, so `RAPIDR_TEST_CLIPBOARD` keeps tests off the
//!   user's).
//! - **Input methods**: a composition shows, underlined, at the caret but
//!   isn't the program's text until committed; the host allows IME only
//!   while an edit has the focus ([`ComponentKind::wants_ime`]).

use std::cell::RefCell;

use rapidr_value::objects::a11y::{node_id, AccessNode, Action, Role};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::Rect;
use rapidr_value::objects::{with_list, with_list_mut, with_textedit, with_textedit_mut};
use rapidr_value::scrollbars::Scroller;

use super::{ComponentKind, Cx, Ime, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::a11y::AccessValue;
use crate::display::TextItem;
use crate::input::{Clipboard, Mods};
use crate::paint::{Painter, DARK, FACE, GRAY_TEXT, HIGHLIGHT, HIGHLIGHT_TEXT, LIGHT, SHADOW};
use crate::store::{self, Store};
use crate::text::{bgr_to_rgb, Align, Ink, Look, TextEditor, TextSystem};
use crate::tree::{FormUi, NodeUi};

/// Where a text box's text lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// The shared `TextEdit` (QEDIT, QMEMO, QRICHEDIT).
    Text,
    /// A QCOMBOBOX's `ItemList::text` (its editable box).
    Combo,
}

/// The model as an edit shows it.
struct Model {
    /// Changes when the program changed the text or the selection.
    revision: u64,
    text: String,
    sel: Option<(usize, usize)>,
    read_only: bool,
    max_length: i64,
    char_case: i64,
}

fn read_model(src: Source, id: &str) -> Option<Model> {
    match src {
        Source::Text => with_textedit(id, |t| Model { revision: t.revision, text: t.raw(), sel: Some((t.sel_start, t.sel_len)), read_only: t.read_only, max_length: t.max_length, char_case: t.char_case }),
        Source::Combo => with_list(id, |l| {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            l.text.hash(&mut h);
            Model { revision: h.finish(), text: l.text.clone(), sel: None, read_only: false, max_length: 0, char_case: 0 }
        }),
    }
}

/// An edit's last change, for Ctrl+Z (Windows' edits undo one step, and an
/// undo undoes itself).
#[derive(Clone, Debug, PartialEq)]
struct Undo {
    text: String,
    sel: (usize, usize),
}

/// A text box's editor and how it shows the model.
pub struct EditUi {
    pub ed: TextEditor,
    src: Source,
    /// The model's revision last shown (`None`: nothing yet).
    shown: Option<u64>,
    /// How far the text is scrolled (device pixels): across, down.
    scroll: (f64, f64),
    /// A memo's scroll bars (logical pixels).
    pub bars: Scroller,
    /// The mouse went down on the bars, and where it is now (in their
    /// area).
    bar_at: Option<(i64, i64)>,
    undo: Option<Undo>,
    /// The last change was typing (typing on adds to its undo step).
    typing: bool,
}

/// What an edit's context menu offers now (`ComponentKind::context_menu`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MenuState {
    pub undo: bool,
    pub cut: bool,
    pub copy: bool,
    pub paste: bool,
    pub delete: bool,
    pub select_all: bool,
}

/// The text area of a QEDIT inside its 2-pixel sunken edge and a 1-pixel
/// margin.
fn inner(w: i64, h: i64) -> Rect {
    (3, 2, w - 6, h - 4)
}

impl EditUi {
    pub fn new(multi: bool, src: Source) -> EditUi {
        let mut bars = Scroller::default();
        bars.auto = false;
        bars.horz.visible = false;
        bars.vert.visible = false;
        EditUi { ed: TextEditor::new(multi), src, shown: None, scroll: (0.0, 0.0), bars, bar_at: None, undo: None, typing: false }
    }

    /// The text as the editor holds it (a composition included).
    pub fn text(&self) -> String {
        self.ed.text()
    }

    /// Paragraph 0's layout.
    pub fn layout(&self) -> Option<&parley::Layout<Ink>> {
        self.ed.para_layout(0)
    }

    /// Paragraph `p`'s layout (what a display list's text item draws).
    pub fn para_layout(&self, p: usize) -> Option<&parley::Layout<Ink>> {
        self.ed.para_layout(p)
    }

    /// An input method is composing.
    pub fn composing(&self) -> bool {
        self.ed.composing()
    }

    pub fn scroll(&self) -> (f64, f64) {
        self.scroll
    }

    /// Scrolls to `to`, kept within a text of `content` in a `view`
    /// (device pixels).
    pub(crate) fn set_scroll(&mut self, to: (f64, f64), view: (f64, f64), content: (f64, f64)) {
        self.scroll = to;
        self.clamp_scroll(view, content);
    }

    /// The bars held (where the mouse is in their area) or let go.
    pub(crate) fn hold_bars(&mut self, at: Option<(i64, i64)>) {
        self.bar_at = at;
    }

    pub(crate) fn bars_held(&self) -> bool {
        self.bar_at.is_some()
    }

    pub(crate) fn held_at(&self) -> Option<(i64, i64)> {
        self.bar_at
    }

    pub(crate) fn mouse_text(&mut self, m: &MouseIn, area: Rect, dy: f64, multi: bool) -> bool {
        self.mouse(m, area, dy, multi)
    }

    pub(crate) fn sync_model(&mut self, id: &str) -> bool {
        self.sync(id)
    }

    pub(crate) fn scroll_to_caret_in(&mut self, view: (f64, f64)) {
        self.scroll_to_caret(view);
    }

    pub(crate) fn caret_area(&self, at: (i64, i64), dy: f64) -> Rect {
        self.ime_area(at, dy)
    }

    pub(crate) fn menu_state_of(&self, id: &str) -> MenuState {
        self.menu_state(id)
    }

    /// Shows the model again if the program changed it; laid out for
    /// `look` at `scale` in a view `width` logical pixels wide.
    fn refresh(&mut self, ts: &mut TextSystem, id: &str, look: Look, scale: f32, width: f64) {
        self.ed.set_scale(scale);
        self.ed.set_look(look);
        self.ed.set_width(width);
        if let Some(m) = read_model(self.src, id) {
            if self.shown != Some(m.revision) {
                self.shown = Some(m.revision);
                if self.ed.composing() || self.ed.text() != m.text {
                    self.ed.set_text(&m.text);
                }
                match m.sel {
                    Some((start, len)) => self.ed.set_selection_chars(start, len),
                    None => {
                        let n = self.ed.text().chars().count();
                        let (s, l) = self.ed.selection_chars();
                        if s + l > n {
                            self.ed.set_selection_chars(n, 0);
                        }
                    }
                }
            }
        }
        self.ed.lay_out(ts);
    }

    /// What the user did, into the model: whether the text changed
    /// (OnChange). Not while composing.
    fn sync(&mut self, id: &str) -> bool {
        if self.ed.composing() {
            return false;
        }
        let text = self.ed.text();
        let (start, len) = self.ed.selection_chars();
        match self.src {
            Source::Text => with_textedit_mut(id, |t| {
                let before = t.raw();
                t.user_edit(&text, start, len);
                text != before
            })
            .unwrap_or(false),
            Source::Combo => {
                let changed = with_list_mut(id, |l| {
                    let changed = l.text != text;
                    l.text = text.clone();
                    changed
                })
                .unwrap_or(false);
                // (the model's text is the editor's: not the program's change)
                self.shown = read_model(self.src, id).map(|m| m.revision);
                changed
            }
        }
    }

    /// Before a change: what Ctrl+Z goes back to (`typing` adds to a run
    /// of typing).
    fn remember(&mut self, typing: bool) {
        if !(typing && self.typing) {
            self.undo = Some(Undo { text: self.ed.text(), sel: self.ed.selection_chars() });
        }
        self.typing = typing;
    }

    /// Ctrl+Z: the text before the last change (and that change kept for
    /// the next Ctrl+Z).
    fn undo(&mut self, ts: &mut TextSystem) -> bool {
        let Some(u) = self.undo.take() else { return false };
        let now = Undo { text: self.ed.text(), sel: self.ed.selection_chars() };
        self.ed.set_text(&u.text);
        self.ed.lay_out(ts);
        self.ed.set_selection_chars(u.sel.0, u.sel.1);
        self.undo = Some(now);
        self.typing = false;
        true
    }

    /// Typed, pasted or composed text in (ReadOnly, MaxLength and CharCase
    /// apply, as Windows' edit control's): whether there was text to type.
    fn insert(&mut self, ts: &mut TextSystem, id: &str, s: &str, typing: bool) -> bool {
        let multi = self.ed.multi();
        let s: String = s.chars().filter(|c| !c.is_control() || (multi && *c == '\n')).collect();
        if s.is_empty() {
            return false;
        }
        let Some(m) = read_model(self.src, id) else { return true };
        if m.read_only {
            return true;
        }
        let s = match m.char_case {
            1 => s.to_uppercase(),
            2 => s.to_lowercase(),
            _ => s,
        };
        let room = if m.max_length > 0 {
            let selected = self.ed.selection_chars().1;
            (m.max_length as usize).saturating_sub(self.ed.text().chars().count() - selected)
        } else {
            usize::MAX
        };
        let s: String = s.chars().take(room).collect();
        if !s.is_empty() {
            self.remember(typing);
            self.ed.replace_selection(ts, &s);
        }
        true
    }

    fn read_only(&self, id: &str) -> bool {
        read_model(self.src, id).is_some_and(|m| m.read_only)
    }

    /// Copying out of a PasswordChar edit isn't allowed (Windows').
    fn masked(&self) -> bool {
        self.ed.look().mask.is_some()
    }

    /// A key: whether it was the edit's (Windows' virtual-key codes).
    fn key(&mut self, ts: &mut TextSystem, id: &str, k: &KeyIn, clip: &mut dyn Clipboard, page: isize) -> bool {
        let read_only = self.read_only(id);
        let m = k.mods;
        let multi = self.ed.multi();
        // Ctrl+Insert copies, Shift+Insert pastes, Shift+Delete cuts.
        let (vk, command) = match (k.vk, m.shift, m.command || m.ctrl) {
            (45, false, true) => (67, true),
            (45, true, false) => (86, true),
            (46, true, false) => (88, true),
            (vk, _, c) => (vk, c && m.command),
        };
        if command {
            match vk {
                // Ctrl/Cmd + A, C, X, V, Z
                65 => self.ed.select_all(),
                67 => {
                    if self.ed.has_selection() && !self.masked() {
                        clip.set_text(&self.ed.selected_text());
                    }
                }
                88 => {
                    if self.ed.has_selection() && !self.masked() {
                        clip.set_text(&self.ed.selected_text());
                        if !read_only {
                            self.remember(false);
                            self.ed.replace_selection(ts, "");
                        }
                    }
                }
                86 => {
                    if let Some(t) = clip.get_text() {
                        let t = t.replace("\r\n", "\n").replace('\r', "\n");
                        // (a single-line edit pastes the first line)
                        let t = if multi { t } else { t.lines().next().unwrap_or("").to_string() };
                        self.insert(ts, id, &t, false);
                    }
                }
                90 => {
                    if !read_only {
                        self.undo(ts);
                    }
                }
                // (Cmd + arrows on macOS: to the line's / text's ends)
                37 | 36 if !m.ctrl => self.ed.move_edge(false, false, m.shift),
                39 | 35 if !m.ctrl => self.ed.move_edge(true, false, m.shift),
                38 if !m.ctrl => self.ed.move_edge(false, true, m.shift),
                40 if !m.ctrl => self.ed.move_edge(true, true, m.shift),
                _ if !m.ctrl => return false,
                _ => return self.move_key(ts, vk, m.shift, true, read_only, page),
            }
            self.typing = false;
            return true;
        }
        if self.move_key(ts, vk, m.shift, m.word, read_only, page) {
            return true;
        }
        if m.alt || m.ctrl {
            return false;
        }
        if vk == 13 {
            // (a memo's Enter breaks the line; a QEDIT's goes to the form)
            return multi && {
                self.insert(ts, id, "\n", true);
                true
            };
        }
        self.insert(ts, id, k.text, true)
    }

    /// Caret moves, selection and deletion: whether `vk` was one.
    fn move_key(&mut self, ts: &mut TextSystem, vk: i64, shift: bool, word: bool, read_only: bool, page: isize) -> bool {
        let multi = self.ed.multi();
        let e = &mut self.ed;
        match vk {
            37 => e.move_h(false, word, shift),
            39 => e.move_h(true, word, shift),
            // (a single-line edit's Up / Down move as Left / Right)
            38 if multi => e.move_v(-1, shift),
            40 if multi => e.move_v(1, shift),
            38 => e.move_h(false, word, shift),
            40 => e.move_h(true, word, shift),
            33 if multi => e.move_v(-page, shift),
            34 if multi => e.move_v(page, shift),
            36 => e.move_edge(false, word && multi, shift),
            35 => e.move_edge(true, word && multi, shift),
            8 | 46 if read_only => {}
            8 | 46 => {
                let before = (e.text().len(), e.selection_chars());
                let word = word && !self.ed.look().mask.is_some();
                let undo = Undo { text: self.ed.text(), sel: self.ed.selection_chars() };
                let changed = if vk == 8 { self.ed.delete_back(ts, word) } else { self.ed.delete_forward(ts, word) };
                if changed && (self.ed.text().len(), self.ed.selection_chars()) != before {
                    if !self.typing {
                        self.undo = Some(undo);
                    }
                    self.typing = true;
                }
            }
            _ => return false,
        }
        if !matches!(vk, 8 | 46) {
            self.typing = false;
        }
        true
    }

    /// Keeps the caret in a `view` (device pixels) of the text: scrolls
    /// across (and, `multi`, down); then within the text's size.
    fn scroll_to_caret(&mut self, view: (f64, f64)) {
        let (cw, ch) = view;
        let (x0, y0, _, y1) = self.ed.caret_rect(1.0);
        let (tw, th) = self.ed.content_size();
        let (mut sx, mut sy) = self.scroll;
        if x0 - sx > cw - 1.0 {
            sx = x0 - cw + 1.0;
        } else if x0 < sx {
            sx = x0;
        }
        if self.ed.multi() {
            if y1 - sy > ch {
                sy = y1 - ch;
            } else if y0 < sy {
                sy = y0;
            }
        } else {
            sy = 0.0;
        }
        self.scroll = (sx, sy);
        self.clamp_scroll(view, (tw + 1.0, th));
    }

    fn clamp_scroll(&mut self, (cw, ch): (f64, f64), (tw, th): (f64, f64)) {
        let (sx, sy) = self.scroll;
        self.scroll = (sx.min(tw - cw).max(0.0).round(), sy.min(th - ch).max(0.0).round());
    }

    /// The text item drawing paragraph `p` with the text's top left at
    /// `origin` (device pixels in the window, scrolled).
    fn item(&self, id: &str, p: usize, origin: (f64, f64), show_selection: bool, caret: bool) -> TextItem {
        let s = f64::from(self.ed.scale());
        let cw = s.round().max(1.0);
        let (dx, top) = self.ed.para_origin(p);
        let selection = if show_selection { self.ed.selection_rects(p) } else { Vec::new() };
        let caret = (caret && self.ed.focus().para == p && !self.ed.has_selection()).then(|| {
            let (x0, y0, _, y1) = self.ed.caret_rect(cw as f32);
            let (x, y0, y1) = ((x0 - dx).round(), (y0 - top).round(), (y1 - top).round());
            (x, y0, x + cw, y1)
        });
        TextItem {
            node: id.to_string(),
            para: p,
            origin: (origin.0 + dx, origin.1 + top),
            selection,
            highlight: HIGHLIGHT,
            highlight_text: HIGHLIGHT_TEXT,
            underlines: self.ed.compose_rects(p, cw),
            caret,
            caret_color: 0x000000,
        }
    }

    /// Draws the text in `area` (logical, the component's), its first line
    /// `dy` device pixels down: the paragraphs that show.
    pub(crate) fn paint_text(&self, id: &str, p: &mut Painter, area: Rect, dy: f64, show_selection: bool, caret: bool) {
        let s = p.scale();
        let (ax, ay) = p.device_point(area.0, area.1);
        let origin = (ax - self.scroll.0, ay - self.scroll.1 + dy);
        let range = self.ed.visible(self.scroll.1 - dy, self.scroll.1 - dy + area.3 as f64 * s);
        p.clipped(area, |p| {
            for i in range {
                p.editor(self.item(id, i, origin, show_selection, caret));
            }
        });
    }

    /// A point of the component (logical) in the text (device pixels).
    fn text_point(&self, x: f64, y: f64, area: Rect, dy: f64) -> (f64, f64) {
        let s = f64::from(self.ed.scale());
        ((x - area.0 as f64) * s + self.scroll.0, (y - area.1 as f64) * s - dy + self.scroll.1)
    }

    /// The mouse in the text (`area` its view, the first line `dy` down).
    fn mouse(&mut self, m: &MouseIn, area: Rect, dy: f64, multi: bool) -> bool {
        let (px, py) = self.text_point(m.x, m.y, area, dy);
        match m.kind {
            MouseKind::Down => match m.clicks {
                0 | 1 => self.ed.click(px, py, m.mods.shift),
                2 => self.ed.select_word_at(px, py),
                _ if multi => self.ed.select_para_at(px, py),
                _ => self.ed.select_all(),
            },
            MouseKind::Move if m.captured => self.ed.click(px, py, true),
            _ => return false,
        }
        self.typing = false;
        true
    }

    /// What the context menu offers now.
    fn menu_state(&self, id: &str) -> MenuState {
        let read_only = self.read_only(id);
        let sel = self.ed.has_selection();
        let masked = self.masked();
        MenuState {
            undo: self.undo.is_some() && !read_only,
            cut: sel && !read_only && !masked,
            copy: sel && !masked,
            paste: !read_only,
            delete: sel && !read_only,
            select_all: !self.ed.text().is_empty(),
        }
    }

    /// Where the input method's window goes: the caret (logical, absolute),
    /// the text's top left at `at` (logical, absolute) less the scroll.
    fn ime_area(&self, at: (i64, i64), dy: f64) -> Rect {
        let s = f64::from(self.ed.scale()).max(0.01);
        let (x0, y0, x1, y1) = self.ed.caret_rect(1.0);
        let (l, t) = ((x0 - self.scroll.0) / s, (y0 - self.scroll.1 + dy) / s);
        let (r, b) = ((x1 - self.scroll.0) / s, (y1 - self.scroll.1 + dy) / s);
        (at.0 + l.floor() as i64, at.1 + t.floor() as i64, (r - l).ceil().max(1.0) as i64, (b - t).ceil().max(1.0) as i64)
    }
}

/// How a text box's text looks now (its store's Font, PasswordChar,
/// Alignment, WordWrap).
pub fn look_of(store: &dyn Store, id: &str, font: &Font, enabled: bool, multi: bool) -> Look {
    let color = if enabled { bgr_to_rgb(font.color) } else { GRAY_TEXT };
    let mask = if multi { None } else { store::string(store, id, "passwordchar").chars().next() };
    let align = Align::from_prop(store::int(store, id, "alignment", 0));
    let wrap = multi && store::flag(store, id, "wordwrap", true);
    Look { font: font.clone(), color, mask, align, wrap }
}

/// Node `ui`'s editor (made the first time), showing the model, laid out
/// for a view `width` logical pixels wide.
fn ensure<'u>(ui: &'u mut NodeUi, ts: &mut TextSystem, id: &str, spec: &Spec, scale: f64) -> &'u mut EditUi {
    if ui.edit.as_ref().is_none_or(|e| e.ed.multi() != spec.multi || e.src != spec.src) {
        ui.edit = Some(Box::new(EditUi::new(spec.multi, spec.src)));
    }
    let e = ui.edit.as_mut().expect("made above");
    e.refresh(ts, id, spec.look.clone(), scale as f32, spec.width);
    e
}

/// A text box's background: its Color, white unless set.
pub fn background(store: &dyn Store, id: &str) -> u32 {
    match store.get(id, "color") {
        rapidr_value::Value::Null => 0xFFFFFF,
        v => bgr_to_rgb(v.to_i64()),
    }
}

/// Whether the selection shows (HideSelection, True by default: only
/// while focused).
pub fn shows_selection(store: &dyn Store, id: &str, focused: bool) -> bool {
    focused || !store::flag(store, id, "hideselection", true)
}

pub struct Edit;

impl Edit {
    fn spec(cx: &Cx) -> Spec {
        Spec::line(cx, inner(cx.width(), cx.height()), Source::Text)
    }

    /// QEDIT's editor for `cx`, ready.
    fn editor<'c>(cx: &'c mut Cx) -> &'c mut EditUi {
        Self::spec(cx).editor(&mut *cx.ui, &mut *cx.text, cx.id, cx.scale)
    }

    /// Where the line sits in the text area: centred down (device pixels).
    fn line_dy(e: &EditUi, h: i64) -> f64 {
        let s = f64::from(e.ed.scale());
        let lh = e.ed.content_size().1;
        ((inner(0, h).3 as f64 * s - lh) / 2.0).round()
    }

    fn view(e: &EditUi, w: i64, h: i64) -> (f64, f64) {
        let s = f64::from(e.ed.scale());
        let (_, _, iw, ih) = inner(w, h);
        ((iw as f64 * s).max(1.0), (ih as f64 * s).max(1.0))
    }
}

/// Paints, routes and describes a single-line text box over `src` in the
/// component-relative text area `area`: what QEDIT and an editable combo's
/// box share.
pub(crate) fn paint_line(cx: &mut Cx, p: &mut Painter, area: Rect, src: Source) {
    let (focused, caret_on) = (cx.state.focused, cx.state.caret_on);
    let show = shows_selection(cx.store, cx.id, focused);
    let spec = Spec::line(cx, area, src);
    let id = cx.id;
    let e = spec.editor(&mut *cx.ui, &mut *cx.text, id, cx.scale);
    let s = f64::from(e.ed.scale());
    let dy = ((area.3 as f64 * s - e.ed.content_size().1) / 2.0).round();
    e.paint_text(id, p, area, dy, show, focused && caret_on);
}

/// Requests the kernel carries out after the input that asked (an edit's
/// menu key; a context menu's pick).
enum Request {
    /// Open component `id`'s context menu at its caret.
    Menu(String),
    /// The context menu's item `cmd` picked.
    Pick(String),
}

thread_local! {
    static REQUESTS: RefCell<Vec<Request>> = const { RefCell::new(Vec::new()) };
    /// The form and component whose context menu is open.
    static MENU_FOR: RefCell<Option<(String, String)>> = const { RefCell::new(None) };
}

/// The kernel's edit menu (a pop-up in the shared menu model, under an id
/// no program can name).
pub const EDIT_MENU: &str = "rr$editmenu";
const ITEMS: [(&str, &str); 8] = [("undo", "&Undo"), ("sep1", "-"), ("cut", "Cu&t"), ("copy", "&Copy"), ("paste", "&Paste"), ("delete", "&Delete"), ("sep2", "-"), ("selectall", "Select &All")];

/// Whether `id` is an item of the kernel's edit menu.
pub fn is_menu_item(id: &str) -> bool {
    id.starts_with(EDIT_MENU)
}

/// The edit menu's item `id` picked (menubar.rs's pick): done after the
/// input, on the edit that opened it.
pub fn queue_pick(id: &str) {
    REQUESTS.with(|r| r.borrow_mut().push(Request::Pick(id.to_string())));
}

/// The edit menu in the menu model, with what applies now enabled.
fn edit_menu(state: MenuState) {
    use rapidr_value::objects::menu;
    use rapidr_value::{v_int, v_str};
    if !menu::is_menu(EDIT_MENU) {
        menu::create(EDIT_MENU, "RPOPUPMENU");
        for (name, caption) in ITEMS {
            let item = format!("{EDIT_MENU}.{name}");
            menu::create(&item, "RMENUITEM");
            menu::set(&item, "caption", &v_str(caption));
            menu::set(&item, "parent", &v_str(EDIT_MENU));
        }
    }
    for (name, on) in [("undo", state.undo), ("cut", state.cut), ("copy", state.copy), ("paste", state.paste), ("delete", state.delete), ("selectall", state.select_all)] {
        let item = format!("{EDIT_MENU}.{name}");
        if menu::with(&item, |n| n.enabled) != Some(on) {
            menu::set(&item, "enabled", &v_int(i64::from(on)));
        }
    }
}

impl FormUi {
    /// Node `i`'s context menu at (x, y) of the client area: the kernel's
    /// edit menu, when it's an edit without a PopupMenu of the program's.
    pub(crate) fn open_edit_menu(&mut self, store: &dyn Store, ts: &mut TextSystem, i: usize, x: f64, y: f64) {
        let id = self.nodes[i].id.clone();
        if !store::string(store, &id, "popupmenu").is_empty() {
            return;
        }
        if self.can_focus(store, i) && self.focus != Some(i) {
            self.set_focus(Some(i));
        }
        let Some(state) = self.with_cx(store, ts, i, |k, cx| k.context_menu(cx)).flatten() else { return };
        edit_menu(state);
        if self.open_popup(EDIT_MENU, x.floor() as i64, y.floor() as i64) {
            MENU_FOR.with(|m| *m.borrow_mut() = Some((self.form.clone(), id)));
        }
    }

    /// What edits asked for during the input just routed: the menu key's
    /// context menu, the menu's pick (Undo, Cut, Copy, Paste, Delete,
    /// Select All — done as their keys would be, without OnKeyDown).
    pub fn edit_commands(&mut self, store: &dyn Store, ts: &mut TextSystem, clip: &mut dyn Clipboard) {
        for r in REQUESTS.with(|r| std::mem::take(&mut *r.borrow_mut())) {
            match r {
                Request::Menu(id) => {
                    let Some(i) = self.index_of(&id) else { continue };
                    let at = self.with_cx(store, ts, i, |k, cx| k.ime_area(cx)).flatten();
                    let (x, y) = at.map_or_else(|| (self.nodes[i].abs.0 as f64, self.nodes[i].abs.1 as f64), |(x, y, _, h)| (x as f64, (y + h) as f64));
                    self.open_edit_menu(store, ts, i, x, y);
                }
                Request::Pick(item) => {
                    let Some((form, id)) = MENU_FOR.with(|m| m.borrow_mut().take()) else { continue };
                    if form != self.form {
                        continue;
                    }
                    let Some(i) = self.index_of(&id) else { continue };
                    let cmd = item.strip_prefix(EDIT_MENU).unwrap_or("").trim_start_matches('.');
                    let command = Mods { command: true, ..Mods::NONE };
                    let (vk, mods) = match cmd {
                        "undo" => (90, command),
                        "cut" => (88, command),
                        "copy" => (67, command),
                        "paste" => (86, command),
                        "delete" => (46, Mods::NONE),
                        "selectall" => (65, command),
                        _ => continue,
                    };
                    self.dirty = true;
                    self.with_cx(store, ts, i, |k, cx| k.key(cx, &KeyIn { vk, text: "", mods }, clip));
                }
            }
        }
    }
}

/// The menu key (or Shift+F10) on an edit: its context menu.
fn menu_key(id: &str, k: &KeyIn) -> bool {
    if k.vk == 93 || (k.vk == 121 && k.mods.shift && !k.mods.ctrl && !k.mods.alt) {
        REQUESTS.with(|r| r.borrow_mut().push(Request::Menu(id.to_string())));
        return true;
    }
    false
}

/// What a text box's editor is made for: how it looks, its view's width
/// (logical), one line or paragraphs, its model.
#[derive(Clone, Debug)]
pub(crate) struct Spec {
    pub look: Look,
    pub width: f64,
    pub multi: bool,
    pub src: Source,
}

impl Spec {
    /// A single-line box's (QEDIT, a combo's) with text area `area`.
    pub fn line(cx: &Cx, area: Rect, src: Source) -> Spec {
        Spec { look: look_of(cx.store, cx.id, &cx.font, cx.state.enabled, false), width: area.2 as f64, multi: false, src }
    }

    /// Node `ui`'s editor made for it, showing the model.
    pub fn editor<'u>(&self, ui: &'u mut NodeUi, ts: &mut TextSystem, id: &str, scale: f64) -> &'u mut EditUi {
        ensure(ui, ts, id, self, scale)
    }
}

/// A key on a text box (its editor made for `spec`; `view`: what of the
/// text shows, device pixels): whether it was its. OnChange when the text
/// changed.
pub(crate) fn key_in(cx: &mut Cx, spec: &Spec, k: &KeyIn, clip: &mut dyn Clipboard, view: impl Fn(&EditUi) -> (f64, f64)) -> bool {
    if menu_key(cx.id, k) {
        return true;
    }
    let id = cx.id;
    let e = spec.editor(&mut *cx.ui, &mut *cx.text, id, cx.scale);
    let page = {
        let (_, vh) = view(e);
        ((vh / e.ed.line_height()).floor() as isize).max(1)
    };
    let handled = e.key(&mut *cx.text, id, k, clip, page);
    let changed = e.sync(id);
    e.ed.lay_out(&mut *cx.text);
    let v = view(e);
    e.scroll_to_caret(v);
    if changed {
        cx.change();
    }
    handled
}

/// An input method's composition into a text box: OnChange for a commit
/// that changed the text.
pub(crate) fn ime_box(cx: &mut Cx, spec: &Spec, ime: &Ime, view: impl Fn(&EditUi) -> (f64, f64)) {
    let id = cx.id;
    let e = spec.editor(&mut *cx.ui, &mut *cx.text, id, cx.scale);
    let changed = ime_in(e, &mut *cx.text, id, ime);
    let v = view(e);
    e.scroll_to_caret(v);
    if changed {
        cx.change();
    }
}

impl ComponentKind for Edit {
    fn name(&self) -> &'static str {
        "REDIT"
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        p.fill((0, 0, w, h), background(cx.store, cx.id));
        // Windows' sunken client edge.
        p.edge((0, 0, w, h), &[SHADOW, DARK], &[LIGHT, FACE]);
        paint_line(cx, p, inner(w, h), Source::Text);
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let (w, h) = (cx.width(), cx.height());
        let id = cx.id.to_string();
        let changed = {
            let e = Self::editor(cx);
            let dy = Self::line_dy(e, h);
            if !e.mouse(m, inner(w, h), dy, false) {
                return MouseOut::default();
            }
            let changed = e.sync(&id);
            let v = Self::view(e, w, h);
            e.scroll_to_caret(v);
            changed
        };
        if changed {
            cx.change();
        }
        MouseOut::default()
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, clip: &mut dyn Clipboard) -> bool {
        let (w, h) = (cx.width(), cx.height());
        let spec = Self::spec(cx);
        key_in(cx, &spec, k, clip, |e| Self::view(e, w, h))
    }

    fn ime(&self, cx: &mut Cx, ime: &Ime) -> bool {
        let (w, h) = (cx.width(), cx.height());
        if !self.wants_ime(cx.store, cx.id) {
            return false;
        }
        let spec = Self::spec(cx);
        ime_box(cx, &spec, ime, |e| Self::view(e, w, h));
        true
    }

    fn ime_area(&self, cx: &mut Cx) -> Option<Rect> {
        let (x0, y0, w, h) = cx.rect;
        let e = Self::editor(cx);
        let dy = Self::line_dy(e, h);
        let (ix, iy, _, _) = inner(w, h);
        Some(e.ime_area((x0 + ix, y0 + iy), dy))
    }

    fn wants_ime(&self, store: &dyn Store, id: &str) -> bool {
        store::string(store, id, "passwordchar").is_empty() && !with_textedit(id, |t| t.read_only).unwrap_or(true)
    }

    fn context_menu(&self, cx: &mut Cx) -> Option<MenuState> {
        let id = cx.id.to_string();
        Some(Self::editor(cx).menu_state(&id))
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        describe_text(cx, false)
    }

    fn access(&self, cx: &mut Cx, action: Action, _part: Option<usize>, value: Option<&AccessValue>) -> bool {
        access_text(cx, action, value)
    }
}

/// An input method's composition into editor `e`: whether the text
/// changed (a commit).
pub(crate) fn ime_in(e: &mut EditUi, ts: &mut TextSystem, id: &str, ime: &Ime) -> bool {
    match ime {
        Ime::Preedit(text, cursor) => {
            if text.is_empty() {
                e.ed.clear_compose(ts);
            } else {
                e.ed.set_compose(ts, text, *cursor);
            }
            false
        }
        Ime::Commit(text) => {
            e.ed.clear_compose(ts);
            e.insert(ts, id, text, true);
            e.sync(id)
        }
    }
}

/// A text box's accessibility node: its text (a PasswordChar's characters
/// for a password), ReadOnly, multi-line.
pub(crate) fn describe_text(cx: &mut Cx, multi: bool) -> AccessNode {
    let mut n = AccessNode::new(node_id(cx.id), Role::TextInput);
    let (text, read_only) = with_textedit(cx.id, |t| (t.text(), t.read_only)).unwrap_or_default();
    n.value = Some(match store::string(cx.store, cx.id, "passwordchar").chars().next() {
        Some(m) if !multi => std::iter::repeat_n(m, text.chars().count()).collect(),
        _ => text,
    });
    n.states.read_only = read_only;
    n.states.multiline = multi;
    n.actions = vec![Action::SetValue, Action::Focus];
    n.bounds = cx.rect;
    n
}

/// A screen reader setting a text box's value (as typing would: Modified,
/// OnChange).
pub(crate) fn access_text(cx: &mut Cx, action: Action, value: Option<&AccessValue>) -> bool {
    let (Action::SetValue, Some(AccessValue::Text(s))) = (action, value) else { return false };
    let changed = with_textedit_mut(cx.id, |t| {
        if t.read_only || t.text() == *s {
            return false;
        }
        t.set_text(s);
        t.modified = true;
        true
    });
    if changed.unwrap_or(false) {
        cx.change();
    }
    true
}

/// [`EditUi::mouse`] for a single-line box whose text area is `area` (a
/// combo's box): whether the text took it.
pub(crate) fn mouse_line(cx: &mut Cx, m: &MouseIn, area: Rect, src: Source) -> bool {
    let spec = Spec::line(cx, area, src);
    let id = cx.id;
    let e = spec.editor(&mut *cx.ui, &mut *cx.text, id, cx.scale);
    let s = f64::from(e.ed.scale());
    let dy = ((area.3 as f64 * s - e.ed.content_size().1) / 2.0).round();
    if !e.mouse(m, area, dy, false) {
        return false;
    }
    let changed = e.sync(id);
    e.scroll_to_caret(((area.2 as f64 * s).max(1.0), (area.3 as f64 * s).max(1.0)));
    if changed {
        cx.change();
    }
    true
}

/// [`key_in`] for a single-line box whose text area is `area` (a combo's).
pub(crate) fn key_line(cx: &mut Cx, k: &KeyIn, clip: &mut dyn Clipboard, area: Rect, src: Source) -> bool {
    let spec = Spec::line(cx, area, src);
    key_in(cx, &spec, k, clip, |e| {
        let s = f64::from(e.ed.scale());
        ((area.2 as f64 * s).max(1.0), (area.3 as f64 * s).max(1.0))
    })
}

/// The context menu of a single-line box over `src` with text area `area`.
pub(crate) fn menu_line(cx: &mut Cx, area: Rect, src: Source) -> MenuState {
    let spec = Spec::line(cx, area, src);
    spec.editor(&mut *cx.ui, &mut *cx.text, cx.id, cx.scale).menu_state(cx.id)
}

/// Where a single-line box's input method window goes (its text area
/// `area`).
pub(crate) fn ime_area_line(cx: &mut Cx, area: Rect, src: Source) -> Rect {
    let spec = Spec::line(cx, area, src);
    let (x0, y0) = (cx.rect.0, cx.rect.1);
    let e = spec.editor(&mut *cx.ui, &mut *cx.text, cx.id, cx.scale);
    let s = f64::from(e.ed.scale());
    let dy = ((area.3 as f64 * s - e.ed.content_size().1) / 2.0).round();
    e.ime_area((x0 + area.0, y0 + area.1), dy)
}

#[cfg(test)]
#[path = "edit_tests.rs"]
mod tests;
