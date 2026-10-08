//! RCODEEDITOR (docs/ide-plan.md I2): RapidR Studio's code editor, a view
//! over the shared model `rapidr_value::objects::codeedit` (the editor's
//! document from `rapidr-editor`). Drawn by the kernel on every host, so it
//! is the same on the desktop and in the browser.
//!
//! - **Virtualized** (`view.rs`): only the rows in view are laid out (an
//!   LRU cache of parley layouts keyed by their text and style); rows are
//!   one line high, so scrolling is arithmetic.
//! - **The gutter**: line numbers, markers (breakpoints, the debugger's
//!   line, bookmarks, the program's own), diagnostics' icons, changes since
//!   the last save, fold arrows.
//! - **Editing** (`input.rs`): multiple carets and selections (Alt+click,
//!   Ctrl+D, Alt+Shift+drag for a column), folding, brackets and pairs,
//!   auto-indent, line moves, comments, word wrap, zoom; undo grouped by
//!   word; IME (the kernel's preedit / commit).
//! - **Find / replace / go to line / rename** (`find.rs`): a box over the
//!   text's top right.
//! - **Language features** (`lang.rs`, `popup.rs`): squiggles, completion,
//!   hover and signature popups drawn in the popup layer (over the other
//!   components), fed by the runtime's language service
//!   (`rapidr_editor::service`, RapidR's own for BASIC) or by the program
//!   (OnCompletionRequest → ShowCompletion …); automatic keyword case;
//!   go to definition, rename, formatting.
//! - **Colours** from the theme's scheme (`rapidr_value::code_scheme`), the
//!   code in JetBrains Mono.
//! - **Accessibility** (`access.rs`): a multi-line text field with the
//!   lines around the caret as its value, line / column and the caret
//!   line's problems announced, the completion list a list box.

pub mod access;
pub mod find;
pub mod input;
pub mod lang;
pub mod paint;
pub mod popup;
pub mod view;

use rapidr_editor::Buffer;
use rapidr_value::code_scheme::{self, Scheme};
use rapidr_value::objects::a11y::{AccessNode, Action};
use rapidr_value::objects::codeedit::CodeEditor as Model;
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::Rect;
use rapidr_value::objects::{with_code, with_code_mut};
use rapidr_value::scrollbars::{Scroller, BAR};

use super::edit::MenuState;
use super::{ComponentKind, Cx, Ime, KeyIn, MouseIn, MouseOut};
use crate::a11y::AccessValue;
use crate::input::{Clipboard, KernelEvent, Mods};
use crate::paint::Painter;
use crate::store::Store;
use crate::text::TextSystem;
use crate::tick::Instant;
use view::{LayoutCache, Metrics, RowMap};

/// What the mouse is dragging.
#[derive(Clone, Debug, PartialEq)]
pub enum Drag {
    /// Selecting text from `anchor` (bytes) by `unit`; `add`: a new caret
    /// (Alt+click); `column`: a column of carets (Alt+Shift).
    Text { anchor: (usize, usize), unit: Unit, add: bool, column: bool, at: (f64, f64) },
    /// Selecting whole lines from the gutter.
    Lines { anchor: usize, at: (f64, f64) },
    /// The minimap's slider.
    Minimap { grab: f64 },
    /// The scroll bars.
    Bars,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Char,
    Word,
    Line,
}

/// Where the view's parts are (the component's logical pixels).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Geo {
    /// Inside the border.
    pub inner: Rect,
    pub gutter: Rect,
    /// The gutter's columns: markers / diagnostics, line numbers (right
    /// edge), changes, fold arrows.
    pub glyph_x: i64,
    pub numbers_right: i64,
    pub diff_x: i64,
    pub fold_x: i64,
    /// The text's view.
    pub text: Rect,
    pub minimap: Option<Rect>,
    /// What the scroll bars scroll (the text and the minimap) and where.
    pub bars: Rect,
}

/// The language service's work for this editor (lang.rs).
#[derive(Clone, Debug, Default)]
pub struct Service {
    /// The document (generation, version) it was last given.
    pub synced: Option<(u64, u64)>,
    /// When to ask for diagnostics (after typing stops).
    pub diagnose_at: Option<Instant>,
    /// When to ask for a hover (the mouse resting) and where.
    pub hover_at: Option<(Instant, usize)>,
    /// Signature help follows the caret while it's open.
    pub signature_open: bool,
}

/// A snippet's tab stops being filled (bytes, in order; the current one).
#[derive(Clone, Debug, Default)]
pub struct SnippetSession {
    pub stops: Vec<(usize, usize)>,
    pub current: usize,
}

/// The view's own state (the model holds the rest).
pub struct CodeUi {
    pub rows: RowMap,
    pub cache: LayoutCache,
    pub metrics: Metrics,
    pub font: Font,
    /// Scrolled (logical pixels): across, down.
    pub scroll: (f64, f64),
    pub bars: Scroller,
    pub geo: Geo,
    /// What of the model's the view showed last.
    pub seen_revision: u64,
    pub seen_reveal: u64,
    pub seen_generation: u64,
    pub drag: Option<Drag>,
    pub find: Option<find::FindBox>,
    /// Ctrl+K pressed: the chord's second key comes.
    pub chord: bool,
    /// The primary caret and the selections as last reported (OnCaretMove,
    /// OnSelectionChange).
    pub last_caret: (usize, usize),
    pub last_selections: Vec<(usize, usize)>,
    /// An input method's composition at the primary caret.
    pub preedit: Option<(String, Option<(usize, usize)>)>,
    pub service: Service,
    pub snippet: Option<SnippetSession>,
    /// What a screen reader is told next (a live region).
    pub announce: String,
    /// The mouse over the gutter (fold arrows show).
    pub gutter_hover: bool,
    /// The completion list's first shown item.
    pub completion_top: usize,
    /// The view's own deadlines: idle colouring and folds.
    pub idle_at: Option<Instant>,
    /// The scale and theme the cache was made for.
    pub cache_key: (u64, &'static str, u64),
    /// The changes since the last save, per line (gutter marks): computed
    /// at idle.
    pub changes: lang::LineChanges,
    /// The frame's rows: (row index, row, cache slot, device origin).
    pub shown: Vec<paint::Shown>,
}

impl CodeUi {
    fn new() -> CodeUi {
        let mut bars = Scroller::default();
        bars.auto = false;
        bars.horz.visible = true;
        bars.vert.visible = true;
        CodeUi {
            rows: RowMap::default(),
            cache: LayoutCache::default(),
            metrics: Metrics { px: 13.0, line: 20, ch: 7.8 },
            font: Font::default(),
            scroll: (0.0, 0.0),
            bars,
            geo: Geo::default(),
            seen_revision: u64::MAX,
            seen_reveal: 0,
            seen_generation: u64::MAX,
            drag: None,
            find: None,
            chord: false,
            last_caret: (0, 0),
            last_selections: Vec::new(),
            preedit: None,
            service: Service::default(),
            snippet: None,
            announce: String::new(),
            gutter_hover: false,
            completion_top: 0,
            idle_at: None,
            cache_key: (0, "", 0),
            changes: lang::LineChanges::default(),
            shown: Vec::new(),
        }
    }

    /// A row's height (logical pixels).
    pub fn lh(&self) -> f64 {
        self.metrics.line as f64
    }

    /// Rows in view (whole and part).
    pub fn page_rows(&self) -> usize {
        ((self.geo.text.3 as f64 / self.lh()).ceil() as usize).max(1)
    }

    /// Whole rows in view.
    pub fn full_rows(&self) -> usize {
        ((self.geo.text.3 as f64 / self.lh()).floor() as usize).max(1)
    }

    /// The deadline the view needs next (the soonest of its own).
    pub fn next_wake(&self) -> Option<Instant> {
        let s = &self.service;
        [self.idle_at, s.diagnose_at, s.hover_at.map(|h| h.0)].into_iter().flatten().min()
    }
}

/// Everything a step of the view works with.
pub struct Ctx<'a> {
    pub c: &'a mut Model,
    pub ui: &'a mut CodeUi,
    pub ts: &'a mut TextSystem,
    pub events: &'a mut Vec<KernelEvent>,
    pub store: &'a dyn Store,
    pub id: &'a str,
    pub size: (i64, i64),
    pub scale: f64,
    pub focused: bool,
    pub enabled: bool,
    pub scheme: &'static Scheme,
}

impl Ctx<'_> {
    /// An event for the program.
    pub fn fire(&mut self, event: &str, args: Vec<rapidr_value::Value>) {
        self.events.push(KernelEvent::Fire { id: self.id.to_string(), event: event.to_string(), args });
    }

    /// OnChange.
    pub fn change(&mut self) {
        self.events.push(KernelEvent::Change(self.id.to_string()));
    }

    /// The file name the language service knows this editor by.
    pub fn file(&self) -> String {
        if self.c.file_name.is_empty() {
            format!("untitled-{}.bas", self.id)
        } else {
            self.c.file_name.clone()
        }
    }
}

/// The editor's font (FontName, FontSize: JetBrains Mono at 10 points =
/// 13 pixels by default).
fn editor_font(c: &Model) -> Font {
    Font { name: c.opts.font_name.clone(), size: c.opts.font_size.max(4), ..Font::default() }
}

/// The view's geometry for a component `w` × `h`.
fn geometry(ui: &mut CodeUi, c: &Model, w: i64, h: i64) -> Geo {
    let fluent = rapidr_value::theme::current().fluent();
    let border = if fluent { 1 } else { 2 };
    let inner = (border, border, (w - 2 * border).max(0), (h - 2 * border).max(0));
    let ch = ui.metrics.ch;
    let digits = c.doc.line_count().max(1).to_string().len().max(3) as f64;
    let numbers = if c.opts.show_line_numbers { (digits * ch).ceil() as i64 + 12 } else { 0 };
    let glyph = 18;
    let diff = 4;
    let fold = if c.opts.show_folding { 16 } else { 0 };
    let gutter_w = glyph + numbers + diff + fold + 4;
    let gutter = (inner.0, inner.1, gutter_w.min(inner.2), inner.3);
    let glyph_x = gutter.0 + 1;
    let numbers_right = glyph_x + glyph + numbers - 6;
    let diff_x = glyph_x + glyph + numbers;
    let fold_x = diff_x + diff;
    let bars = (gutter.0 + gutter.2, inner.1, (inner.2 - gutter.2).max(0), inner.3);
    // (the bars decide what's left of the area for text and the minimap)
    ui.bars.vert.visible = true;
    ui.bars.horz.visible = !c.opts.word_wrap;
    let (cw, chh) = ui.bars.client(bars.2, bars.3);
    let mini_w = if c.opts.show_minimap && cw > 240 { 90 } else { 0 };
    let minimap = (mini_w > 0).then_some((bars.0 + cw - mini_w, bars.1, mini_w, chh));
    let text = (bars.0 + 4, bars.1, (cw - mini_w - 4).max(1), chh.max(1));
    Geo { inner, gutter, glyph_x, numbers_right, diff_x, fold_x, text, minimap, bars }
}

/// The scheme the editor draws with.
fn scheme_of(c: &Model) -> &'static Scheme {
    code_scheme::resolve(&c.opts.color_scheme)
}

/// Runs `f` with the editor's model and view brought up to date: its
/// font's metrics, rows, scroll bars, a program's reveal.
pub fn with_view<R>(cx: &mut Cx, f: impl FnOnce(&mut Ctx) -> R) -> Option<R> {
    let Cx { store, text, id, rect, state, ui, events, scale, .. } = cx;
    let ui = ui.code.get_or_insert_with(|| Box::new(CodeUi::new()));
    let (w, h) = (rect.2, rect.3);
    let (focused, enabled, scale) = (state.focused, state.enabled, *scale);
    with_code_mut(id, |c| {
        let scheme = scheme_of(c);
        let mut ctx = Ctx { c, ui, ts: text, events, store: *store, id, size: (w, h), scale, focused, enabled, scheme };
        sync(&mut ctx);
        f(&mut ctx)
    })
}

/// The view caught up with the model: font, cache, rows, bars, scroll.
pub fn sync(x: &mut Ctx) {
    let font = editor_font(x.c);
    if font != x.ui.font {
        x.ui.metrics = Metrics::of(x.ts, &font);
        x.ui.font = font;
        x.ui.cache.clear();
    }
    // (another scale, theme or font: every layout again)
    let key = ((x.scale * 1000.0) as u64, x.scheme.name, x.ts.generation);
    if key != x.ui.cache_key {
        x.ui.cache.clear();
        x.ui.cache_key = key;
    }
    x.ui.geo = geometry(x.ui, x.c, x.size.0, x.size.1);
    sync_rows(x);
    if x.c.generation != x.ui.seen_generation {
        x.ui.seen_generation = x.c.generation;
        x.ui.scroll = (0.0, 0.0);
        x.ui.snippet = None;
        x.ui.service.synced = None;
        x.ui.changes = lang::LineChanges::default();
    }
    update_bars(x);
    // (the program's requests: done at the next tick, whose events reach it)
    if !x.c.requests.is_empty() {
        x.ui.idle_at = Some(crate::tick::now());
    }
    if x.c.reveal != x.ui.seen_reveal {
        x.ui.seen_reveal = x.c.reveal;
        reveal_caret(x, true);
    }
    x.ui.seen_revision = x.c.revision;
}

/// The row table caught up with the text (an edit just made: the lines it
/// added or took away).
fn sync_rows(x: &mut Ctx) {
    let cols = if x.c.opts.word_wrap { ((x.ui.geo.text.2 as f64 - 8.0) / x.ui.metrics.ch).floor().max(8.0) as usize } else { 0 };
    x.ui.rows.sync(x.c, cols);
}

/// The bars' ranges and positions for the text now.
pub fn update_bars(x: &mut Ctx) {
    // (after an edit in this turn: the rows of the text as it is now — an
    // undo that took lines away once read past the text's end)
    sync_rows(x);
    let lh = x.ui.lh();
    let g = x.ui.geo;
    let rows = x.ui.rows.total() as f64;
    // (the last line can scroll up to the top, as editors do)
    let content_h = (rows - 1.0).max(0.0) * lh + g.text.3 as f64;
    let widest = widest_px(x);
    let b = &mut x.ui.bars;
    b.vert.increment = lh as i64;
    b.horz.increment = (x.ui.metrics.ch * 4.0) as i64;
    b.vert.range = content_h.ceil() as i64 - 1 + (g.bars.3 - g.text.3).max(0);
    b.horz.range = (widest + 40.0).ceil() as i64 + (g.bars.2 - g.text.2);
    b.update(g.bars.2, g.bars.3, &[]);
    clamp_scroll(x);
}

/// The widest line in view's width (pixels): enough for the horizontal
/// bar, without measuring every line of a large file.
fn widest_px(x: &mut Ctx) -> f64 {
    if x.c.opts.word_wrap {
        return 0.0;
    }
    let first = (x.ui.scroll.1 / x.ui.lh()).floor().max(0.0) as usize;
    let rows = x.ui.rows.rows(x.c, first, x.ui.page_rows() + 1);
    let tab = x.c.doc.tab_size as usize;
    let cols = rows.iter().map(|r| view::display_cols(&x.c.doc.line(r.line), tab)).max().unwrap_or(0);
    // (and the caret's line, so typing past the view scrolls)
    let head = x.c.doc.selections().primary().head;
    let line = x.c.doc.buffer().line_of(head);
    let cols = cols.max(view::display_cols(&x.c.doc.line(line), tab));
    cols as f64 * x.ui.metrics.ch
}

/// The scroll kept within the text; the bars' thumbs where it is.
pub fn clamp_scroll(x: &mut Ctx) {
    let g = x.ui.geo;
    let max_y = ((x.ui.rows.total() as f64 - 1.0) * x.ui.lh()).max(0.0);
    let max_x = if x.c.opts.word_wrap { 0.0 } else { (x.ui.bars.horz.range as f64 - (g.bars.2 - g.text.2) as f64 - g.text.2 as f64).max(0.0) };
    x.ui.scroll.1 = x.ui.scroll.1.clamp(0.0, max_y);
    x.ui.scroll.0 = x.ui.scroll.0.clamp(0.0, max_x);
    x.ui.bars.vert.position = x.ui.scroll.1.round() as i64;
    x.ui.bars.horz.position = x.ui.scroll.0.round() as i64;
}

/// Scrolls so the primary caret shows (`center`: in the middle when it
/// was far away, as a jump does).
pub fn reveal_caret(x: &mut Ctx, center: bool) {
    let head = x.c.doc.selections().primary().head;
    reveal_byte(x, head, center);
}

/// Scrolls so byte `at` shows.
pub fn reveal_byte(x: &mut Ctx, at: usize, center: bool) {
    let lh = x.ui.lh();
    let (row, piece) = x.ui.rows.row_at(x.c, at);
    let y = row as f64 * lh;
    let view_h = x.ui.geo.text.3 as f64;
    let top = x.ui.scroll.1;
    if y < top || y + lh > top + view_h {
        let far = y < top - view_h || y > top + 2.0 * view_h;
        x.ui.scroll.1 = if center && far {
            y - (view_h / 2.0 - lh / 2.0).max(0.0)
        } else if y < top {
            y
        } else {
            y + lh - view_h
        };
        // (whole rows at the top)
        x.ui.scroll.1 = (x.ui.scroll.1 / lh).round() * lh;
    }
    if !x.c.opts.word_wrap {
        let line_start = x.c.doc.buffer().line_start(piece.line);
        let tab = x.c.doc.tab_size as usize;
        let text = x.c.doc.line(piece.line);
        let col = view::display_cols(&text[..(at - line_start).min(text.len())], tab) as f64;
        let px = col * x.ui.metrics.ch;
        let vw = x.ui.geo.text.2 as f64;
        let margin = (x.ui.metrics.ch * 4.0).min(vw / 3.0);
        if px < x.ui.scroll.0 + margin {
            x.ui.scroll.0 = (px - margin).max(0.0);
        } else if px > x.ui.scroll.0 + vw - margin {
            x.ui.scroll.0 = px - vw + margin;
        }
    }
    update_bars(x);
}

pub struct CodeEditor;

impl ComponentKind for CodeEditor {
    fn name(&self) -> &'static str {
        "RCODEEDITOR"
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        paint::paint(cx, p);
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        input::mouse(cx, m)
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, clip: &mut dyn Clipboard) -> bool {
        input::key(cx, k, clip)
    }

    fn ime(&self, cx: &mut Cx, ime: &Ime) -> bool {
        input::ime(cx, ime)
    }

    fn ime_area(&self, cx: &mut Cx) -> Option<Rect> {
        input::ime_area(cx)
    }

    fn wants_ime(&self, _store: &dyn Store, id: &str) -> bool {
        with_code(id, |c| !c.doc.read_only).unwrap_or(false)
    }

    fn wheel(&self, cx: &mut Cx, dx: f64, dy: f64, mods: Mods) -> bool {
        input::wheel(cx, dx, dy, mods)
    }

    fn tick(&self, cx: &mut Cx) {
        input::tick(cx);
    }

    fn pending(&self, id: &str) -> bool {
        with_code(id, |c| !c.requests.is_empty()).unwrap_or(false)
    }

    fn context_menu(&self, cx: &mut Cx) -> Option<MenuState> {
        with_view(cx, |x| {
            let has = x.c.doc.selections().iter().any(|s| !s.is_empty());
            let ro = x.c.doc.read_only;
            MenuState { undo: x.c.doc.can_undo() && !ro, cut: !ro, copy: true, paste: !ro, delete: has && !ro, select_all: x.c.doc.len_bytes() > 0 }
        })
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        access::describe(cx)
    }

    fn access(&self, cx: &mut Cx, action: Action, part: Option<usize>, value: Option<&AccessValue>) -> bool {
        access::access(cx, action, part, value)
    }
}

/// Whether node `i`'s code editor takes a plain Tab (not read-only; a
/// program can give it to the focus with WantTabs = False).
pub fn takes_tab(store: &dyn Store, id: &str) -> bool {
    crate::store::flag(store, id, "wanttabs", true) && with_code(id, |c| !c.doc.read_only).unwrap_or(false)
}

/// Whether the focused code editor's find box takes Alt+`vk` (its
/// toggles: C case, W whole word, R regex, L in the selection) before the
/// form's mnemonics and menus — as VS Code's find widget does.
pub fn find_takes_alt(f: &crate::tree::FormUi, vk: i64) -> bool {
    matches!(vk, 67 | 76 | 82 | 87)
        && f.focus.is_some_and(|i| {
            let n = &f.nodes[i];
            n.type_name == "RCODEEDITOR" && n.ui.code.as_ref().is_some_and(|c| c.find.is_some())
        })
}

/// Scroll bars' area client size (for the tests).
pub fn bar_width() -> i64 {
    BAR
}

#[cfg(test)]
mod tests;
