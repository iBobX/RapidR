//! A QFORM and its components as RapidR's own UI kernel would hold them
//! (ROADMAP principle 7): each component is a shared model that lays itself
//! out, draws itself as ops, hit-tests and takes input, and the host only
//! renders the ops and passes input in. Nothing here knows about winit,
//! wgpu or the OS: positions are RapidQ's logical pixels (1/96 inch), keys
//! are the kernel's own [`Key`], events go out as [`Event`]s for the
//! program's handlers (OnClick, OnChange).
//!
//! QTRACKBAR and QTABCONTROL are the existing `rapidr_value` models, routed
//! as the FLTK and web runtimes route them (`mouse_down`, `drag`, `key`
//! with Windows' virtual key codes). QEDIT keeps its text and selection in
//! the shared `TextEdit` model (what the program reads: Text, SelStart,
//! SelLength, Modified) and edits with parley's `PlainEditor` (shaping,
//! caret, selection, word moves, IME). QBUTTON and QLABEL are drawn here, as
//! Windows-classic controls, until they move to shared models too.

use parley::PlainEditor;
use rapidr_value::objects::font::Font;
use rapidr_value::objects::tabcontrol::{Rect, TabControl};
use rapidr_value::objects::textedit::TextEdit;
use rapidr_value::objects::trackbar::TrackBar;
use vello::kurbo::Affine;

use crate::paint::{Painter, Place};
use crate::text::{self, Ink, TextSystem};

/// Windows' 3D colours, as the shared models draw them.
pub const FACE: u32 = 0xF0F0F0;
const LIGHT: u32 = 0xFFFFFF;
const SHADOW: u32 = 0x808080;
const DARK: u32 = 0x404040;
/// A button under the mouse (additive: classic Windows has no hover).
const HOT_FACE: u32 = 0xE5F1FB;
/// Selected text's background and colour (COLOR_HIGHLIGHT).
const HIGHLIGHT: u32 = 0x0078D7;
const HIGHLIGHT_TEXT: u32 = 0xFFFFFF;

/// A key, as the kernel sees it (the host maps the OS's keys to these).
#[derive(Clone, Debug, PartialEq)]
pub enum Key {
    Tab,
    Enter,
    Space,
    Escape,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Backspace,
    Delete,
    /// A character key (shortcuts: with `Mods::command`).
    Char(char),
    Other,
}

impl Key {
    /// Windows' virtual key code, what the shared models' `key` take.
    pub fn vk(&self) -> Option<i64> {
        Some(match self {
            Key::PageUp => 33,
            Key::PageDown => 34,
            Key::End => 35,
            Key::Home => 36,
            Key::Left => 37,
            Key::Up => 38,
            Key::Right => 39,
            Key::Down => 40,
            _ => return None,
        })
    }
}

/// Modifier keys. `command` is the shortcut key (Cmd on macOS, Ctrl
/// elsewhere); `word` moves by words (Option on macOS, Ctrl elsewhere).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Mods {
    pub shift: bool,
    pub command: bool,
    pub word: bool,
}

/// The OS clipboard, as the kernel uses it (Copy, Cut, Paste in edits).
pub trait Clipboard {
    fn get_text(&mut self) -> Option<String>;
    fn set_text(&mut self, text: &str);
}

/// What happened, for the program's handlers.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// OnClick (a button clicked, by mouse, Space / Enter, or a screen reader).
    Click(usize),
    /// OnChange (a track bar's position, a tab control's tab, an edit's text).
    Change(usize),
}

/// A QEDIT: the shared model and parley's editor showing it.
pub struct Edit {
    pub model: TextEdit,
    editor: PlainEditor<Ink>,
    /// The model's revision last shown (the program changed it since: show
    /// it again).
    shown: u64,
    /// How far the text is scrolled left (device pixels).
    scroll: f64,
    /// The scale the editor lays out at.
    scale: f32,
}

pub enum Kind {
    Label { caption: String },
    Button { caption: String },
    Edit(Box<Edit>),
    TrackBar(TrackBar),
    TabControl(TabControl),
}

pub struct Component {
    pub name: String,
    pub left: i64,
    pub top: i64,
    pub width: i64,
    pub height: i64,
    pub font: Font,
    pub enabled: bool,
    pub kind: Kind,
}

impl Component {
    /// A component of RapidR's type `type_name`, its size RapidQ's default
    /// for it (`rapidr_value::layout::default_size`).
    pub fn new(name: &str, type_name: &str, left: i64, top: i64, kind: Kind) -> Self {
        let (width, height) = rapidr_value::layout::default_size(type_name).unwrap_or((75, 25));
        Component { name: name.into(), left, top, width, height, font: Font::default(), enabled: true, kind }
    }

    pub fn rect(&self) -> Rect {
        (self.left, self.top, self.width, self.height)
    }

    fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.left as f64 && y >= self.top as f64 && x < (self.left + self.width) as f64 && y < (self.top + self.height) as f64
    }

    /// Takes the keyboard focus (Tab stops on it).
    pub fn focusable(&self) -> bool {
        self.enabled && !matches!(self.kind, Kind::Label { .. })
    }

    pub fn set_caption(&mut self, s: &str) {
        if let Kind::Label { caption } | Kind::Button { caption } = &mut self.kind {
            *caption = s.to_string();
        }
    }
}

pub struct Form {
    pub caption: String,
    /// The client area (ClientWidth × ClientHeight).
    pub width: i64,
    pub height: i64,
    /// Color (&HBBGGRR).
    pub color: i64,
    pub components: Vec<Component>,
    pub focus: Option<usize>,
    /// Under the mouse.
    hover: Option<usize>,
    /// A button held down (drawn pushed while the mouse is on it).
    pressed: Option<usize>,
    /// The component the mouse is captured by (a button held, a thumb
    /// dragged, a selection made).
    capture: Option<usize>,
    /// The caret shows (it blinks).
    pub caret_on: bool,
}

/// Byte offset of the `n`th character.
fn byte_of(s: &str, n: usize) -> usize {
    s.char_indices().nth(n).map_or(s.len(), |(b, _)| b)
}

/// Characters before byte offset `b`.
fn chars_to(s: &str, b: usize) -> usize {
    s[..b.min(s.len())].chars().count()
}

impl Edit {
    pub fn new(text: &str, font: &Font) -> Box<Edit> {
        let mut model = TextEdit::new(false);
        model.set_text(text);
        let mut editor = PlainEditor::new(text::font_pixels(font));
        for prop in text::styles(font, 0x000000) {
            editor.edit_styles().insert(prop);
        }
        Box::new(Edit { model, editor, shown: u64::MAX, scroll: 0.0, scale: 1.0 })
    }

    /// Shows the model again if the program changed it, at `scale`.
    fn refresh(&mut self, ts: &mut TextSystem, scale: f32) {
        if self.scale != scale {
            self.scale = scale;
            self.editor.set_scale(scale);
        }
        if self.shown != self.model.revision {
            self.shown = self.model.revision;
            let raw = self.model.raw();
            self.editor.set_text(&raw);
            let (a, b) = (byte_of(&raw, self.model.sel_start), byte_of(&raw, self.model.sel_start + self.model.sel_len));
            self.editor.driver(&mut ts.font_cx, &mut ts.layout_cx).select_byte_range(a, b);
        }
        self.editor.refresh_layout(&mut ts.font_cx, &mut ts.layout_cx);
    }

    /// What the user did, into the model (as the runtimes'
    /// `TextEdit::user_edit`): whether the text changed (OnChange).
    fn sync(&mut self) -> bool {
        let text = self.editor.raw_text().to_string();
        let range = self.editor.raw_selection().text_range();
        let before = self.model.raw();
        let start = chars_to(&text, range.start);
        self.model.user_edit(&text, start, chars_to(&text, range.end) - start);
        text != before
    }

    /// The text area inside the 2-pixel sunken edge and a 1-pixel margin.
    fn inner(w: i64, h: i64) -> Rect {
        (3, 2, w - 6, h - 4)
    }

    /// Keeps the caret in view (scrolls the text sideways).
    fn scroll_to_caret(&mut self, w: i64) {
        let inner_w = (Self::inner(w, 0).2 as f64 * f64::from(self.scale)).max(1.0);
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

    /// A point of the control (logical) in the layout's (device) pixels.
    fn layout_point(&self, x: f64, y: f64, w: i64, h: i64) -> (f32, f32) {
        let (ix, iy, _, ih) = Self::inner(w, h);
        let s = f64::from(self.scale);
        let lh = self.editor.try_layout().map_or(0.0, |l| f64::from(l.height()));
        let top = (iy as f64 * s + ((ih as f64 * s - lh) / 2.0).round()).round();
        ((x * s - ix as f64 * s + self.scroll) as f32, (y * s - top) as f32)
    }

    /// Inserts typed or pasted text (ReadOnly, MaxLength and CharCase
    /// apply, as Windows' edit control's).
    fn insert(&mut self, ts: &mut TextSystem, s: &str) {
        if self.model.read_only {
            return;
        }
        let s: String = s.chars().filter(|c| !c.is_control()).collect();
        let s = match self.model.char_case {
            1 => s.to_uppercase(),
            2 => s.to_lowercase(),
            _ => s,
        };
        let room = if self.model.max_length > 0 {
            let selected = self.editor.selected_text().map_or(0, |t| t.chars().count());
            (self.model.max_length as usize).saturating_sub(self.editor.raw_text().chars().count() - selected)
        } else {
            usize::MAX
        };
        let s: String = s.chars().take(room).collect();
        if !s.is_empty() {
            self.editor.driver(&mut ts.font_cx, &mut ts.layout_cx).insert_or_replace_selection(&s);
        }
    }

    /// A key on the focused edit: whether it was the edit's.
    fn key(&mut self, ts: &mut TextSystem, key: &Key, m: Mods, clip: &mut dyn Clipboard) -> bool {
        let read_only = self.model.read_only;
        if m.command {
            match key {
                Key::Char('a') => self.editor.driver(&mut ts.font_cx, &mut ts.layout_cx).select_all(),
                Key::Char('c') => {
                    if let Some(t) = self.editor.selected_text() {
                        clip.set_text(t);
                    }
                }
                Key::Char('x') => {
                    if let Some(t) = self.editor.selected_text() {
                        clip.set_text(t);
                        if !read_only {
                            self.editor.driver(&mut ts.font_cx, &mut ts.layout_cx).delete_selection();
                        }
                    }
                }
                Key::Char('v') => {
                    if let Some(t) = clip.get_text() {
                        // (a single-line edit pastes the first line)
                        let line = t.lines().next().unwrap_or("").to_string();
                        self.insert(ts, &line);
                    }
                }
                Key::Left | Key::Home => self.editor.driver(&mut ts.font_cx, &mut ts.layout_cx).move_to_line_start(),
                Key::Right | Key::End => self.editor.driver(&mut ts.font_cx, &mut ts.layout_cx).move_to_line_end(),
                _ => return false,
            }
            return true;
        }
        let mut d = self.editor.driver(&mut ts.font_cx, &mut ts.layout_cx);
        match (key, m.shift, m.word) {
            // (a single-line edit's Up / Down move as Left / Right)
            (Key::Left | Key::Up, false, false) => d.move_left(),
            (Key::Left | Key::Up, true, false) => d.select_left(),
            (Key::Left | Key::Up, false, true) => d.move_word_left(),
            (Key::Left | Key::Up, true, true) => d.select_word_left(),
            (Key::Right | Key::Down, false, false) => d.move_right(),
            (Key::Right | Key::Down, true, false) => d.select_right(),
            (Key::Right | Key::Down, false, true) => d.move_word_right(),
            (Key::Right | Key::Down, true, true) => d.select_word_right(),
            (Key::Home, false, _) => d.move_to_line_start(),
            (Key::Home, true, _) => d.select_to_line_start(),
            (Key::End, false, _) => d.move_to_line_end(),
            (Key::End, true, _) => d.select_to_line_end(),
            (Key::Backspace, _, false) if !read_only => d.backdelete(),
            (Key::Backspace, _, true) if !read_only => d.backdelete_word(),
            (Key::Delete, _, false) if !read_only => d.delete(),
            (Key::Delete, _, true) if !read_only => d.delete_word(),
            (Key::Backspace | Key::Delete, ..) => {}
            _ => return false,
        }
        true
    }

    fn paint(&mut self, p: &mut Painter, w: i64, h: i64, focused: bool, caret_on: bool) {
        p.fill((0, 0, w, h), 0xFFFFFF);
        // Windows' sunken client edge.
        p.edge((0, 0, w, h), &[SHADOW, DARK], &[LIGHT, FACE]);
        self.refresh(p.text, p.scale as f32);
        let (ix, iy, iw, ih) = Self::inner(w, h);
        let s = p.scale;
        let lh = self.editor.try_layout().map_or(0.0, |l| f64::from(l.height()));
        let (x0, y0) = p.device_point(ix, iy);
        let origin = (x0 - self.scroll, y0 + ((ih as f64 * s - lh) / 2.0).round());
        let to_device = Affine::translate(origin);
        let selection = if focused { self.editor.selection_geometry() } else { Vec::new() };
        let caret = self.editor.cursor_geometry((s.round().max(1.0)) as f32);
        let Some(layout) = self.editor.try_layout() else { return };
        p.clipped((ix, iy, iw, ih), |p| {
            for (b, _) in &selection {
                let r = vello::kurbo::Rect::new(b.x0, b.y0, b.x1, b.y1);
                p.scene.fill(vello::peniko::Fill::NonZero, to_device, crate::paint::color(HIGHLIGHT), None, &r);
            }
            p.layout(layout, to_device);
            // The selected text again, white, inside the highlight.
            for (b, _) in &selection {
                let r = vello::kurbo::Rect::new(b.x0, b.y0, b.x1, b.y1);
                p.scene.push_clip_layer(vello::peniko::Fill::NonZero, to_device, &r);
                p.layout_in(layout, to_device, HIGHLIGHT_TEXT);
                p.scene.pop_layer();
            }
            if focused && caret_on && selection.is_empty() {
                if let Some(c) = caret {
                    let r = vello::kurbo::Rect::new(c.x0.round(), c.y0.round(), c.x0.round() + (s.round().max(1.0)), c.y1.round());
                    p.scene.fill(vello::peniko::Fill::NonZero, to_device, crate::paint::color(0x000000), None, &r);
                }
            }
        });
    }
}

impl Form {
    /// A form with RapidQ's default size: QFORM's Width × Height is the
    /// whole window (`default_size("RFORM")`, 320 × 240); its client area is
    /// that less the frame, as the desktop runtime accounts for it.
    pub fn new(caption: &str) -> Self {
        let (w, h) = rapidr_value::layout::default_size("RFORM").unwrap_or((320, 240));
        let (width, height) = rapidr_value::layout::form_client_size(w, h, 2, 0);
        Form {
            caption: caption.into(),
            width,
            height,
            color: rapidr_value::objects::form_color(&rapidr_value::Value::Null),
            components: Vec::new(),
            focus: None,
            hover: None,
            pressed: None,
            capture: None,
            caret_on: true,
        }
    }

    pub fn add(&mut self, c: Component) -> usize {
        self.components.push(c);
        let i = self.components.len() - 1;
        if self.focus.is_none() && self.components[i].focusable() {
            self.focus = Some(i);
        }
        i
    }

    pub fn find(&self, name: &str) -> Option<usize> {
        self.components.iter().position(|c| c.name.eq_ignore_ascii_case(name))
    }

    /// The topmost component at a point (the last created is on top).
    fn hit(&self, x: f64, y: f64) -> Option<usize> {
        self.components.iter().rposition(|c| c.contains(x, y))
    }

    /// Tab / Shift-Tab: the next (previous) component that takes the
    /// focus, in creation order (TabOrder).
    pub fn move_focus(&mut self, back: bool) {
        let n = self.components.len();
        if n == 0 {
            return;
        }
        let start = self.focus.unwrap_or(if back { 0 } else { n - 1 });
        for k in 1..=n {
            let i = if back { (start + n * 2 - k) % n } else { (start + k) % n };
            if self.components[i].focusable() {
                self.set_focus(Some(i));
                return;
            }
        }
    }

    pub fn set_focus(&mut self, i: Option<usize>) {
        self.focus = i;
        self.caret_on = true;
    }

    pub fn mouse_move(&mut self, x: f64, y: f64, ts: &mut TextSystem) -> (bool, Vec<Event>) {
        let mut events = Vec::new();
        let hover = self.hit(x, y);
        let mut redraw = hover != self.hover;
        self.hover = hover;
        if let Some(i) = self.capture {
            let c = &mut self.components[i];
            let (lx, ly, w, h) = (x - c.left as f64, y - c.top as f64, c.width, c.height);
            match &mut c.kind {
                Kind::TrackBar(t) if self.pressed.is_none() => {
                    if t.drag(lx, ly, w as f64, h as f64) {
                        events.push(Event::Change(i));
                    }
                    redraw = true;
                }
                Kind::Edit(e) => {
                    let (px, py) = e.layout_point(lx, ly, w, h);
                    e.editor.driver(&mut ts.font_cx, &mut ts.layout_cx).extend_selection_to_point(px, py);
                    e.sync();
                    e.scroll_to_caret(w);
                    redraw = true;
                }
                _ => {}
            }
        }
        // HotTrack: each tab control follows the mouse.
        for (i, c) in self.components.iter_mut().enumerate() {
            if let Kind::TabControl(t) = &mut c.kind {
                let at = (hover == Some(i)).then(|| ((x as i64) - c.left, (y as i64) - c.top));
                redraw |= t.mouse_move(at, c.width, c.height, &c.font);
            }
        }
        (redraw, events)
    }

    pub fn mouse_leave(&mut self) {
        self.hover = None;
    }

    pub fn mouse_down(&mut self, x: f64, y: f64, m: Mods, ts: &mut TextSystem) -> Vec<Event> {
        let mut events = Vec::new();
        let Some(i) = self.hit(x, y) else { return events };
        let focusable = self.components[i].focusable();
        let c = &mut self.components[i];
        let (lx, ly, w, h) = (x - c.left as f64, y - c.top as f64, c.width, c.height);
        let mut take_focus = focusable;
        match &mut c.kind {
            Kind::Button { .. } => {
                self.pressed = Some(i);
                self.capture = Some(i);
            }
            Kind::Edit(e) => {
                e.refresh(ts, e.scale);
                let (px, py) = e.layout_point(lx, ly, w, h);
                let mut d = e.editor.driver(&mut ts.font_cx, &mut ts.layout_cx);
                if m.shift { d.shift_click_extension(px, py) } else { d.move_to_point(px, py) }
                e.sync();
                e.scroll_to_caret(w);
                self.capture = Some(i);
            }
            Kind::TrackBar(t) => {
                let (drag, changed) = t.mouse_down(lx, ly, w as f64, h as f64);
                if drag {
                    self.capture = Some(i);
                }
                if changed {
                    events.push(Event::Change(i));
                }
            }
            Kind::TabControl(t) => match t.mouse_down(lx as i64, ly as i64, w, h, &c.font) {
                Some((changed, focus)) => {
                    take_focus = focus;
                    if changed {
                        events.push(Event::Change(i));
                    }
                }
                None => take_focus = false,
            },
            Kind::Label { .. } => {}
        }
        if take_focus {
            self.set_focus(Some(i));
        }
        events
    }

    pub fn mouse_up(&mut self, x: f64, y: f64) -> Vec<Event> {
        let mut events = Vec::new();
        if let Some(i) = self.pressed.take() {
            // (a click: let go on the button it was pressed on)
            if self.hit(x, y) == Some(i) {
                events.push(Event::Click(i));
            }
        }
        self.capture = None;
        events
    }

    /// A key down (with what it types, if anything).
    pub fn key(&mut self, key: Key, m: Mods, typed: Option<&str>, ts: &mut TextSystem, clip: &mut dyn Clipboard) -> Vec<Event> {
        let mut events = Vec::new();
        if key == Key::Tab && !m.command {
            self.move_focus(m.shift);
            return events;
        }
        let Some(i) = self.focus else { return events };
        self.caret_on = true;
        let c = &mut self.components[i];
        match &mut c.kind {
            Kind::Button { .. } => {
                if matches!(key, Key::Space | Key::Enter) {
                    events.push(Event::Click(i));
                }
            }
            Kind::Edit(e) => {
                e.refresh(ts, e.scale);
                if !e.key(ts, &key, m, clip) && !m.command {
                    if let Some(t) = typed {
                        e.insert(ts, t);
                    }
                }
                if e.sync() {
                    events.push(Event::Change(i));
                }
                e.editor.refresh_layout(&mut ts.font_cx, &mut ts.layout_cx);
                e.scroll_to_caret(c.width);
            }
            Kind::TrackBar(t) => {
                if key.vk().is_some_and(|vk| t.key(vk)) {
                    events.push(Event::Change(i));
                }
            }
            Kind::TabControl(t) => {
                if key.vk().is_some_and(|vk| t.key(vk, c.width, c.height, &c.font)) {
                    events.push(Event::Change(i));
                }
            }
            Kind::Label { .. } => {}
        }
        events
    }

    /// Text committed by an input method (dead keys, compositions) into
    /// the focused edit.
    pub fn commit_text(&mut self, s: &str, ts: &mut TextSystem) -> Vec<Event> {
        let Some(i) = self.focus else { return Vec::new() };
        let c = &mut self.components[i];
        let Kind::Edit(e) = &mut c.kind else { return Vec::new() };
        e.refresh(ts, e.scale);
        e.insert(ts, s);
        e.editor.refresh_layout(&mut ts.font_cx, &mut ts.layout_cx);
        e.scroll_to_caret(c.width);
        if e.sync() { vec![Event::Change(i)] } else { Vec::new() }
    }

    /// Draws the whole form (client area) at the painter's scale.
    pub fn paint(&mut self, p: &mut Painter) {
        let back = bgr_to_rgb(self.color);
        p.fill((0, 0, self.width, self.height), back);
        for (i, c) in self.components.iter_mut().enumerate() {
            let focused = self.focus == Some(i);
            let (w, h) = (c.width, c.height);
            let font = c.font.clone();
            let enabled = c.enabled;
            let pushed = self.pressed == Some(i) && self.hover == Some(i);
            let hot = self.hover == Some(i) && (self.pressed.is_none() || pushed);
            let color = self.color;
            let caret_on = self.caret_on;
            p.at((c.left, c.top), |p| match &mut c.kind {
                // (a label's caption is cut at its edges, as Windows' static)
                Kind::Label { caption } => p.clipped((0, 0, w, h), |p| p.text((0, 0, w, h), caption, &font, bgr_to_rgb(font.color), 0, Place::TopLeft)),
                Kind::Button { caption } => paint_button(p, w, h, caption, &font, focused, pushed, hot),
                Kind::Edit(e) => e.paint(p, w, h, focused, caret_on),
                Kind::TrackBar(t) => {
                    for shape in t.shapes(w as f64, h as f64, enabled) {
                        p.shape(&shape);
                    }
                    if focused {
                        p.focus((0, 0, w, h));
                    }
                }
                Kind::TabControl(t) => p.ops(&t.ops(w, h, &font, color, enabled, focused)),
            });
        }
    }
}

/// &HBBGGRR → 0xRRGGBB.
pub fn bgr_to_rgb(bgr: i64) -> u32 {
    let c = (bgr & 0xFFFFFF) as u32;
    (c & 0xFF) << 16 | (c & 0xFF00) | (c >> 16)
}

/// A Windows-classic push button (DrawFrameControl DFCS_BUTTONPUSH): raised;
/// with the focus it's the default button (a black frame) and shows a focus
/// rectangle; pushed, a black frame and a shadow line, its caption a pixel
/// down and right.
#[allow(clippy::too_many_arguments)]
fn paint_button(p: &mut Painter, w: i64, h: i64, caption: &str, font: &Font, focused: bool, pushed: bool, hot: bool) {
    p.fill((0, 0, w, h), if hot && !pushed { HOT_FACE } else { FACE });
    let mut r = (0, 0, w, h);
    if focused || pushed {
        p.edge(r, &[0x000000], &[0x000000]);
        r = (1, 1, w - 2, h - 2);
    }
    if pushed {
        p.edge(r, &[SHADOW], &[SHADOW]);
    } else {
        p.edge(r, &[LIGHT], &[DARK, SHADOW]);
    }
    let shift = i64::from(pushed);
    p.text((shift, shift, w, h), caption, font, bgr_to_rgb(font.color), 0, Place::Center);
    if focused {
        p.focus((4, 4, w - 8, h - 8));
    }
}
