//! QEDIT, single line: its text and selection live in the shared model
//! (`rapidr_value::objects::textedit::TextEdit`: what the program reads as
//! Text, SelStart, SelLength, Modified), shown and edited with parley's
//! `PlainEditor` (shaping, caret, selection, word moves, input methods).
//!
//! - The program changed the model (its `revision` moved): the editor shows
//!   it again ([`EditUi::refresh`]; runtime-core's `text_push` is a no-op).
//! - The user typed or selected: the model gets it at once
//!   (`TextEdit::user_edit`; `text_pull` is a no-op), and OnChange fires
//!   when the text changed.
//! - An input method's composition (preedit) is shown, underlined, but
//!   isn't the program's text until committed.
//!
//! ReadOnly, MaxLength and CharCase apply to typing and pasting, as in
//! Windows' edit control. PasswordChar, Alignment, word selection by
//! double click and the context menu are the text lane's (Stage 7a).

use parley::{Affinity, Cursor, Layout, PlainEditor, Selection};
use rapidr_value::objects::a11y::{node_id, AccessNode, Action, Role};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::Rect;
use rapidr_value::objects::{with_textedit, with_textedit_mut};

use super::{ComponentKind, Cx, Ime, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::a11y::AccessValue;
use crate::display::{DevRect, TextItem};
use crate::input::Clipboard;
use crate::paint::{Painter, DARK, FACE, GRAY_TEXT, HIGHLIGHT, HIGHLIGHT_TEXT, LIGHT, SHADOW};
use crate::text::{bgr_to_rgb, byte_of, chars_to, styles, Ink, TextSystem};
use crate::tree::NodeUi;

/// A QEDIT's parley editor and how it shows the model.
pub struct EditUi {
    editor: PlainEditor<Ink>,
    /// The model's revision last shown (`u64::MAX`: none yet).
    shown: u64,
    /// How far the text is scrolled left (device pixels).
    scroll: f64,
    /// The scale the editor lays out at.
    scale: f32,
    /// The font and colour it's styled with.
    style: (Font, u32),
}

/// The text area inside the 2-pixel sunken edge and a 1-pixel margin.
fn inner(w: i64, h: i64) -> Rect {
    (3, 2, w - 6, h - 4)
}

impl EditUi {
    fn new(font: &Font, color: u32) -> EditUi {
        let mut editor = PlainEditor::new(crate::text::font_pixels(font));
        for prop in styles(font, color) {
            editor.edit_styles().insert(prop);
        }
        EditUi { editor, shown: u64::MAX, scroll: 0.0, scale: 1.0, style: (font.clone(), color) }
    }

    /// The layout the host draws (a [`TextItem`]'s).
    pub fn layout(&self) -> Option<&Layout<Ink>> {
        self.editor.try_layout()
    }

    /// The text as the editor holds it (a composition included).
    pub fn text(&self) -> &str {
        self.editor.raw_text()
    }

    /// An input method is composing.
    pub fn composing(&self) -> bool {
        self.editor.is_composing()
    }

    /// Shows the model again if the program changed it, at `scale`.
    fn refresh(&mut self, ts: &mut TextSystem, id: &str, scale: f32) {
        if self.scale != scale {
            self.scale = scale;
            self.scroll = 0.0;
            self.editor.set_scale(scale);
        }
        let model = with_textedit(id, |t| (t.revision, t.raw(), t.sel_start, t.sel_len));
        if let Some((revision, raw, start, len)) = model {
            if self.shown != revision {
                self.shown = revision;
                self.editor.set_text(&raw);
                let (a, b) = (byte_of(&raw, start), byte_of(&raw, start + len));
                self.editor.driver(&mut ts.font_cx, &mut ts.layout_cx).select_byte_range(a, b);
            }
        }
        self.editor.refresh_layout(&mut ts.font_cx, &mut ts.layout_cx);
    }

    /// What the user did, into the model (`TextEdit::user_edit`): whether
    /// the text changed (OnChange). Not while composing.
    fn sync(&mut self, id: &str) -> bool {
        if self.editor.is_composing() {
            return false;
        }
        let text = self.editor.raw_text().to_string();
        let range = self.editor.raw_selection().text_range();
        let start = chars_to(&text, range.start);
        let len = chars_to(&text, range.end) - start;
        with_textedit_mut(id, |t| {
            let before = t.raw();
            t.user_edit(&text, start, len);
            text != before
        })
        .unwrap_or(false)
    }

    /// Keeps the caret in view (scrolls the text sideways).
    fn scroll_to_caret(&mut self, w: i64) {
        let inner_w = (inner(w, 0).2 as f64 * f64::from(self.scale)).max(1.0);
        let Some(caret) = self.editor.cursor_geometry(1.0) else { return };
        if caret.x0 - self.scroll > inner_w - 1.0 {
            self.scroll = caret.x0 - inner_w + 1.0;
        } else if caret.x0 < self.scroll {
            self.scroll = caret.x0;
        }
        let full = self.editor.try_layout().map_or(0.0, |l| f64::from(l.full_width()));
        if full < inner_w {
            self.scroll = 0.0;
        }
        self.scroll = self.scroll.max(0.0);
    }

    /// Where the layout's (0, 0) is, relative to the control (device
    /// pixels): the text area's left less the scroll, centred down.
    fn layout_origin(&self, w: i64, h: i64) -> (f64, f64) {
        let (ix, iy, _, ih) = inner(w, h);
        let s = f64::from(self.scale);
        let lh = self.editor.try_layout().map_or(0.0, |l| f64::from(l.height()));
        ((ix as f64 * s).round() - self.scroll, (iy as f64 * s).round() + ((ih as f64 * s - lh) / 2.0).round())
    }

    /// A point of the control (logical) in the layout's (device) pixels.
    fn layout_point(&self, x: f64, y: f64, w: i64, h: i64) -> (f32, f32) {
        let s = f64::from(self.scale);
        let (ox, oy) = self.layout_origin(w, h);
        ((x * s - ox) as f32, (y * s - oy) as f32)
    }

    /// Inserts typed or pasted text (ReadOnly, MaxLength and CharCase
    /// apply, as Windows' edit control's): whether there was text to type.
    fn insert(&mut self, ts: &mut TextSystem, id: &str, s: &str) -> bool {
        let s: String = s.chars().filter(|c| !c.is_control()).collect();
        if s.is_empty() {
            return false;
        }
        let Some((read_only, max_length, char_case)) = with_textedit(id, |t| (t.read_only, t.max_length, t.char_case)) else { return true };
        if read_only {
            return true;
        }
        let s = match char_case {
            1 => s.to_uppercase(),
            2 => s.to_lowercase(),
            _ => s,
        };
        let room = if max_length > 0 {
            let selected = self.editor.selected_text().map_or(0, |t| t.chars().count());
            (max_length as usize).saturating_sub(self.editor.raw_text().chars().count() - selected)
        } else {
            usize::MAX
        };
        let s: String = s.chars().take(room).collect();
        if !s.is_empty() {
            self.editor.driver(&mut ts.font_cx, &mut ts.layout_cx).insert_or_replace_selection(&s);
        }
        true
    }

    /// A key: whether it was the edit's (Windows' virtual-key codes).
    fn key(&mut self, ts: &mut TextSystem, id: &str, k: &KeyIn, clip: &mut dyn Clipboard) -> bool {
        let read_only = with_textedit(id, |t| t.read_only).unwrap_or(false);
        let m = k.mods;
        if m.command {
            match k.vk {
                // Ctrl/Cmd + A, C, X, V
                65 => self.editor.driver(&mut ts.font_cx, &mut ts.layout_cx).select_all(),
                67 => {
                    if let Some(t) = self.editor.selected_text() {
                        clip.set_text(t);
                    }
                }
                88 => {
                    if let Some(t) = self.editor.selected_text() {
                        clip.set_text(t);
                        if !read_only {
                            self.editor.driver(&mut ts.font_cx, &mut ts.layout_cx).delete_selection();
                        }
                    }
                }
                86 => {
                    if let Some(t) = clip.get_text() {
                        // (a single-line edit pastes the first line)
                        let line = t.lines().next().unwrap_or("").to_string();
                        self.insert(ts, id, &line);
                    }
                }
                // (Cmd + arrows on macOS: to the line's ends)
                37 | 36 if !m.ctrl => self.editor.driver(&mut ts.font_cx, &mut ts.layout_cx).move_to_line_start(),
                39 | 35 if !m.ctrl => self.editor.driver(&mut ts.font_cx, &mut ts.layout_cx).move_to_line_end(),
                _ if !m.ctrl => return false,
                _ => return self.move_key(ts, k.vk, m.shift, m.word || m.ctrl, read_only),
            }
            return true;
        }
        if self.move_key(ts, k.vk, m.shift, m.word, read_only) {
            return true;
        }
        if m.alt || m.ctrl {
            return false;
        }
        self.insert(ts, id, k.text)
    }

    /// Caret moves, selection and deletion: whether `vk` was one.
    fn move_key(&mut self, ts: &mut TextSystem, vk: i64, shift: bool, word: bool, read_only: bool) -> bool {
        let mut d = self.editor.driver(&mut ts.font_cx, &mut ts.layout_cx);
        match (vk, shift, word) {
            // (a single-line edit's Up / Down move as Left / Right)
            (37 | 38, false, false) => d.move_left(),
            (37 | 38, true, false) => d.select_left(),
            (37 | 38, false, true) => d.move_word_left(),
            (37 | 38, true, true) => d.select_word_left(),
            (39 | 40, false, false) => d.move_right(),
            (39 | 40, true, false) => d.select_right(),
            (39 | 40, false, true) => d.move_word_right(),
            (39 | 40, true, true) => d.select_word_right(),
            (36, false, _) => d.move_to_line_start(),
            (36, true, _) => d.select_to_line_start(),
            (35, false, _) => d.move_to_line_end(),
            (35, true, _) => d.select_to_line_end(),
            (8, _, false) if !read_only => d.backdelete(),
            (8, _, true) if !read_only => d.backdelete_word(),
            (46, _, false) if !read_only => d.delete(),
            (46, _, true) if !read_only => d.delete_word(),
            (8 | 46, ..) => {}
            _ => return false,
        }
        true
    }

    /// The text item drawing it at `origin` (device pixels in the window).
    fn item(&self, id: &str, origin: (f64, f64), focused: bool, caret_on: bool) -> TextItem {
        let s = f64::from(self.scale);
        let cw = s.round().max(1.0);
        let selection: Vec<DevRect> = if focused { self.editor.selection_geometry().into_iter().map(|(b, _)| (b.x0, b.y0, b.x1, b.y1)).collect() } else { Vec::new() };
        let caret = if focused && caret_on && selection.is_empty() {
            self.editor.cursor_geometry(cw as f32).map(|c| (c.x0.round(), c.y0.round(), c.x0.round() + cw, c.y1.round()))
        } else {
            None
        };
        let mut underlines = Vec::new();
        if let (Some(range), Some(layout)) = (self.editor.raw_compose().clone(), self.editor.try_layout()) {
            let sel = Selection::new(Cursor::from_byte_index(layout, range.start, Affinity::Downstream), Cursor::from_byte_index(layout, range.end, Affinity::Upstream));
            for (b, _) in sel.geometry(layout) {
                underlines.push((b.x0, (b.y1 - cw).round(), b.x1, b.y1.round()));
            }
        }
        TextItem { node: id.to_string(), origin, selection, highlight: HIGHLIGHT, highlight_text: HIGHLIGHT_TEXT, underlines, caret, caret_color: 0x000000 }
    }
}

pub struct Edit;

/// Node `ui`'s editor, made (or made again for a new font or state) and
/// showing the model.
fn ensure<'u>(ui: &'u mut NodeUi, ts: &mut TextSystem, id: &str, font: &Font, enabled: bool, scale: f64) -> &'u mut EditUi {
    let color = if enabled { bgr_to_rgb(font.color) } else { GRAY_TEXT };
    if ui.edit.as_ref().is_none_or(|e| e.style != (font.clone(), color)) {
        let mut e = EditUi::new(font, color);
        if let Some(old) = &ui.edit {
            e.scale = old.scale;
            e.editor.set_scale(old.scale);
        }
        ui.edit = Some(Box::new(e));
    }
    let e = ui.edit.as_mut().expect("made above");
    e.refresh(ts, id, scale as f32);
    e
}

/// [`ensure`] for a component's context.
macro_rules! editor {
    ($cx:expr) => {
        ensure(&mut *$cx.ui, &mut *$cx.text, $cx.id, &$cx.font, $cx.state.enabled, $cx.scale)
    };
}

impl ComponentKind for Edit {
    fn name(&self) -> &'static str {
        "REDIT"
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        p.fill((0, 0, w, h), 0xFFFFFF);
        // Windows' sunken client edge.
        p.edge((0, 0, w, h), &[SHADOW, DARK], &[LIGHT, FACE]);
        let (focused, caret_on) = (cx.state.focused, cx.state.caret_on);
        let id = cx.id.to_string();
        let e = editor!(cx);
        let (ox, oy) = e.layout_origin(w, h);
        let (dx, dy) = p.device_point(0, 0);
        let item = e.item(&id, (dx + ox, dy + oy), focused, caret_on);
        let (ix, iy, iw, ih) = inner(w, h);
        p.clipped((ix, iy, iw, ih), |p| p.editor(item));
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let (w, h) = (cx.width(), cx.height());
        let id = cx.id.to_string();
        let changed = {
            let e = editor!(cx);
            let ts = &mut *cx.text;
            let (px, py) = e.layout_point(m.x, m.y, w, h);
            match m.kind {
                MouseKind::Down => {
                    let mut d = e.editor.driver(&mut ts.font_cx, &mut ts.layout_cx);
                    if m.mods.shift {
                        d.shift_click_extension(px, py);
                    } else {
                        d.move_to_point(px, py);
                    }
                }
                MouseKind::Move if m.captured => e.editor.driver(&mut ts.font_cx, &mut ts.layout_cx).extend_selection_to_point(px, py),
                _ => return MouseOut::default(),
            }
            let changed = e.sync(&id);
            e.scroll_to_caret(w);
            changed
        };
        if changed {
            cx.change();
        }
        MouseOut::default()
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, clip: &mut dyn Clipboard) -> bool {
        let w = cx.width();
        let id = cx.id.to_string();
        let e = editor!(cx);
        let ts = &mut *cx.text;
        let handled = e.key(ts, &id, k, clip);
        let changed = e.sync(&id);
        e.editor.refresh_layout(&mut ts.font_cx, &mut ts.layout_cx);
        e.scroll_to_caret(w);
        if changed {
            cx.change();
        }
        handled
    }

    fn ime(&self, cx: &mut Cx, ime: &Ime) -> bool {
        let w = cx.width();
        let id = cx.id.to_string();
        if with_textedit(&id, |t| t.read_only).unwrap_or(true) {
            return false;
        }
        let e = editor!(cx);
        let ts = &mut *cx.text;
        let changed = match ime {
            Ime::Preedit(text, cursor) => {
                let mut d = e.editor.driver(&mut ts.font_cx, &mut ts.layout_cx);
                if text.is_empty() {
                    d.clear_compose();
                } else {
                    d.set_compose(text, *cursor);
                }
                false
            }
            Ime::Commit(text) => {
                e.editor.driver(&mut ts.font_cx, &mut ts.layout_cx).clear_compose();
                e.insert(ts, &id, text);
                e.sync(&id)
            }
        };
        e.editor.refresh_layout(&mut ts.font_cx, &mut ts.layout_cx);
        e.scroll_to_caret(w);
        if changed {
            cx.change();
        }
        true
    }

    fn ime_area(&self, cx: &mut Cx) -> Option<Rect> {
        let (x0, y0, w, h) = cx.rect;
        let e = editor!(cx);
        let s = f64::from(e.scale).max(0.01);
        let (ox, oy) = e.layout_origin(w, h);
        let b = e.editor.ime_cursor_area();
        let (l, t) = ((b.x0 + ox) / s, (b.y0 + oy) / s);
        let (r, bt) = ((b.x1 + ox) / s, (b.y1 + oy) / s);
        Some((x0 + l.floor() as i64, y0 + t.floor() as i64, (r - l).ceil().max(1.0) as i64, (bt - t).ceil().max(1.0) as i64))
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = AccessNode::new(node_id(cx.id), Role::TextInput);
        let (text, read_only) = with_textedit(cx.id, |t| (t.text(), t.read_only)).unwrap_or_default();
        n.value = Some(text);
        n.states.read_only = read_only;
        n.actions = vec![Action::SetValue, Action::Focus];
        n.bounds = cx.rect;
        n
    }

    fn access(&self, cx: &mut Cx, action: Action, _part: Option<usize>, value: Option<&AccessValue>) -> bool {
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
}
