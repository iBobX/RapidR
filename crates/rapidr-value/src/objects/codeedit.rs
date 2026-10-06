//! RCODEEDITOR's model (docs/ide-plan.md I2, docs/ide-components.md §3.1):
//! the editor's document (`rapidr_editor::Document` — a rope, selections,
//! the undo tree, search, the language's colours) and what the program and
//! the view share around it: options, folds, markers, diagnostics, the
//! completion / hover / signature popups, the file it was loaded from. The
//! same on every runtime; the UI kernel's `codeeditor` view draws it and
//! edits the document directly (the user's typing never copies the text).
//!
//! **What RCODEEDITOR always answered stays exactly as it was** (the
//! `code_editor` GUI case): Text with '\n' line breaks (CR LF and a lone CR
//! become '\n' when the program sets it), Lines / `Line(i)` / LineCount /
//! WhereX / WhereY counted from 0, SelStart / SelLength / SelText in
//! characters (a line break counts one), AddStrings, Clear, SelectAll,
//! ClearSelection, Modified, ReadOnly, MaxLength, CharCase, GetSubList,
//! GotoSub, GotoLine (0-based), the clipboard methods, LoadFromFile /
//! SaveToFile; the program's changes fire no OnChange.
//!
//! **What's new counts lines and columns from 1**, as the status bar shows
//! them (CaretLine, CaretColumn, AddCursor, ReplaceRange, Fold, AddMarker,
//! the events' Line / Column); a column is a character of the line.
//!
//! The document keeps '\n' line breaks; a file's own (CR LF for the `.bas`
//! files RapidQ's tools write) and its encoding (UTF-8, with or without a
//! BOM, or Windows' Latin-1) are remembered by LoadFromFile and written
//! back by SaveToFile.

use std::collections::BTreeSet;
use std::sync::Arc;

use rapidr_editor::service::{Completion, CompletionKind, Severity};
use rapidr_editor::transaction::{Assoc, Change, ChangeSet};
use rapidr_editor::{Buffer, Document, EditKind, FoldRange, Language, Languages, LineEnding, SearchQuery, Selection, Selections};

use crate::{v_int, v_str, Value};

/// Most characters a program's text may have (beyond it, text is cut).
const MAX_CHARS: usize = 256 << 20;

/// How a file's text was stored (SaveToFile writes it back the same).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Encoding {
    #[default]
    Utf8,
    Utf8Bom,
    /// Windows' code page 1252 read as Latin-1 (RapidQ's files).
    Latin1,
}

/// Where a problem came from: the program's SetDiagnostics or the
/// language service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagSource {
    Program,
    Service,
}

/// A problem in bytes `start..end` of the text.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    pub start: usize,
    pub end: usize,
    pub severity: Severity,
    pub message: String,
    pub code: String,
    pub source: DiagSource,
}

/// A gutter marker on the line starting at byte `at`: `breakpoint`,
/// `current` (the debugger's line), `error`, `warning`, `bookmark` or the
/// program's own kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Marker {
    pub at: usize,
    pub kind: String,
}

/// The completion list showing: its items, the bytes the chosen one
/// replaces from (`start` .. the caret), the one selected.
#[derive(Clone, Debug, PartialEq)]
pub struct CompletionList {
    pub items: Vec<Completion>,
    pub start: usize,
    pub selected: usize,
    /// The language service's (filtered again as the user types, asked
    /// again when nothing matches); else the program's.
    pub from_service: bool,
}

/// A hover showing over bytes `start..end`.
#[derive(Clone, Debug, PartialEq)]
pub struct HoverInfo {
    pub text: String,
    pub start: usize,
    pub end: usize,
}

/// Signature help showing, anchored at byte `at`.
#[derive(Clone, Debug, PartialEq)]
pub struct SignatureInfo {
    pub label: String,
    pub params: Vec<(usize, usize)>,
    pub active: usize,
    pub doc: String,
    pub at: usize,
}

/// A semantic colour from the language service: bytes `start..end` drawn
/// as token kind `kind` (`function`, `variable.parameter` …).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticSpan {
    pub start: usize,
    pub end: usize,
    pub kind: &'static str,
}

/// What the program asked the view to do (its language features and
/// boxes, done at the view's next tick).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    Completion,
    Signature,
    /// A hover at this byte.
    Hover(usize),
    Format,
    Definition,
    References,
    Rename(String),
    /// The find box: `find`, `replace`, `goto`, `rename`.
    Find(String),
}

/// The editor's options (the program's properties; the view reads them).
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// `ColorScheme`: `auto` (the theme's) or a scheme's name.
    pub color_scheme: String,
    pub word_wrap: bool,
    pub show_line_numbers: bool,
    pub show_folding: bool,
    pub show_minimap: bool,
    pub show_whitespace: bool,
    pub highlight_current_line: bool,
    /// Columns with a ruler line.
    pub rulers: Vec<u32>,
    /// Characters that ask for completion by themselves.
    pub completion_trigger: String,
    /// The built-in language service answers (where the runtime has one).
    pub language_service: bool,
    /// The language's words' case as the user types: `upper`, `lower`,
    /// `proper`, `preserve`.
    pub keyword_case: String,
    pub font_name: String,
    /// Points (10 = 13 pixels, as Font.Size).
    pub font_size: i64,
    pub auto_close: bool,
    pub auto_indent: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            color_scheme: "auto".into(),
            word_wrap: false,
            show_line_numbers: true,
            show_folding: true,
            show_minimap: false,
            show_whitespace: false,
            highlight_current_line: true,
            rulers: Vec::new(),
            completion_trigger: ".".into(),
            language_service: true,
            keyword_case: "upper".into(),
            font_name: crate::objects::text::CODE_FACE.into(),
            font_size: 10,
            auto_close: true,
            auto_indent: true,
        }
    }
}

/// RCODEEDITOR's state.
#[derive(Debug)]
pub struct CodeEditor {
    pub doc: Document,
    pub opts: Options,
    /// Goes up when the program changes the text, the selection or an
    /// option (the view shows the model again).
    pub revision: u64,
    /// Goes up when the program asks for the caret to be scrolled into
    /// view (GotoLine, GotoSub, Find …).
    pub reveal: u64,
    /// Modified, as RapidR always kept it: set by the user's edits and
    /// SelText, cleared by setting Text and by loading or saving.
    pub modified: bool,
    pub max_length: i64,
    pub char_case: i64,
    /// `Language` as the program set it (an id, or a definition file's
    /// path); empty: from the file's name, else BASIC.
    pub language_set: String,
    /// Fold ranges' first lines that are folded.
    pub folded: BTreeSet<usize>,
    pub markers: Vec<Marker>,
    pub diagnostics: Vec<Diagnostic>,
    /// The language service's colours of names (mapped through edits until
    /// asked again).
    pub semantic: Vec<SemanticSpan>,
    pub completion: Option<CompletionList>,
    pub hover: Option<HoverInfo>,
    pub signature: Option<SignatureInfo>,
    /// Where the view last asked the program for a hover (OnHoverRequest):
    /// ShowHover shows there.
    pub hover_request: Option<usize>,
    /// Find's query (FindNext's).
    pub last_find: Option<(String, String)>,
    /// The file it was loaded from or saved to; how it was stored.
    pub file_name: String,
    pub encoding: Encoding,
    pub file_ending: LineEnding,
    /// BeginUpdate's depth: the view doesn't draw while above 0.
    pub update_depth: u32,
    /// The lines edits replaced since the view last looked (first, before,
    /// after), for its per-line caches; `generation` goes up instead when
    /// the whole document is replaced (or too much happened unseen).
    pub line_edits: Vec<(usize, usize, usize)>,
    pub generation: u64,
    /// Places the view follows through edits (a snippet's tab stops): the
    /// byte, and whether text typed at it goes before it (`true`: the
    /// place stays at the typed text's start).
    pub anchors: Vec<(usize, bool)>,
    /// The program's requests for the view.
    pub requests: Vec<Request>,
    /// The fold ranges, as last computed (`fold_version`: the document's
    /// version then; mapped through edits meanwhile).
    folds: Vec<FoldRange>,
    /// Each fold's first and last lines' starts (bytes), mapped through
    /// edits.
    fold_anchors: Vec<(usize, usize)>,
    fold_version: Option<u64>,
}

impl Default for CodeEditor {
    fn default() -> Self {
        CodeEditor::new()
    }
}

/// `s` with CR LF and lone CRs as '\n'.
fn normalize_breaks(s: &str) -> String {
    if s.contains('\r') {
        s.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        s.to_string()
    }
}

fn flag(b: bool) -> Value {
    v_int(if b { -1 } else { 0 })
}

thread_local! {
    /// The languages programs loaded from definition files (beside the
    /// built-in ones).
    static LOADED: std::cell::RefCell<Languages> = std::cell::RefCell::new(Languages::with_builtins());
}

/// The language `name` stands for: a language id, a definition file's path
/// (loaded once), or a file name whose extension says (`x.sql`).
pub fn language_named(name: &str) -> Arc<Language> {
    let n = name.trim();
    if n.is_empty() {
        return basic();
    }
    if let Some(l) = LOADED.with(|l| l.borrow().get(n)) {
        return l;
    }
    if n.to_ascii_lowercase().ends_with(".toml") {
        if let Ok(bytes) = super::read_file(n) {
            if let Ok(l) = LOADED.with(|l| l.borrow_mut().load(&String::from_utf8_lossy(&bytes))) {
                return l;
            }
        }
    }
    LOADED.with(|l| l.borrow().for_path(n))
}

/// RapidQ / RapidR BASIC.
pub fn basic() -> Arc<Language> {
    Languages::builtin().get("rapidq-basic").unwrap_or_else(|| Languages::builtin().plain_text())
}

/// Decodes a file: UTF-8 (a BOM dropped) when it is, else Latin-1.
pub fn decode(bytes: &[u8]) -> (String, Encoding) {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return (String::from_utf8_lossy(rest).into_owned(), Encoding::Utf8Bom);
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => (s.to_string(), Encoding::Utf8),
        Err(_) => (bytes.iter().map(|&b| char::from(b)).collect(), Encoding::Latin1),
    }
}

/// Encodes text as `enc` (Latin-1: characters beyond it as `?`).
pub fn encode(text: &str, enc: Encoding) -> Vec<u8> {
    match enc {
        Encoding::Utf8 => text.as_bytes().to_vec(),
        Encoding::Utf8Bom => [&[0xEF, 0xBB, 0xBF][..], text.as_bytes()].concat(),
        Encoding::Latin1 => text.chars().map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?')).collect(),
    }
}

impl CodeEditor {
    pub fn new() -> CodeEditor {
        let mut doc = Document::new("", basic());
        doc.track_changes(true);
        CodeEditor {
            doc,
            opts: Options::default(),
            revision: 0,
            reveal: 0,
            modified: false,
            max_length: 0,
            char_case: 0,
            language_set: String::new(),
            folded: BTreeSet::new(),
            markers: Vec::new(),
            diagnostics: Vec::new(),
            semantic: Vec::new(),
            completion: None,
            hover: None,
            signature: None,
            hover_request: None,
            last_find: None,
            file_name: String::new(),
            encoding: Encoding::Utf8,
            file_ending: LineEnding::Lf,
            update_depth: 0,
            line_edits: Vec::new(),
            generation: 0,
            anchors: Vec::new(),
            requests: Vec::new(),
            folds: Vec::new(),
            fold_anchors: Vec::new(),
            fold_version: None,
        }
    }

    fn changed(&mut self) {
        self.revision += 1;
    }

    /// The text as the program reads it ('\n' line breaks).
    pub fn text(&self) -> String {
        self.doc.text().to_string()
    }

    fn case_text(&self, s: &str) -> String {
        match self.char_case {
            1 => s.to_uppercase(),
            2 => s.to_lowercase(),
            _ => s.to_string(),
        }
    }

    /// What the program's text becomes ('\n' breaks, CharCase, the size
    /// limit).
    fn program_text(&self, s: &str) -> String {
        let s = self.case_text(&normalize_breaks(s));
        match s.char_indices().nth(MAX_CHARS) {
            Some((b, _)) => s[..b].to_string(),
            None => s,
        }
    }

    // ---- positions ----

    /// Characters before byte `b` (a line break counts one: the text has
    /// only '\n').
    pub fn char_of(&self, b: usize) -> usize {
        self.doc.buffer().byte_to_char(b.min(self.doc.len_bytes()))
    }

    /// The byte of character `c` (clamped).
    pub fn byte_of(&self, c: usize) -> usize {
        let buf = self.doc.buffer();
        let total = buf.byte_to_char(buf.len_bytes());
        buf.char_to_byte(c.min(total))
    }

    /// Byte `b` as a 1-based line and 1-based character column.
    pub fn line_col(&self, b: usize) -> (usize, usize) {
        let buf = self.doc.buffer();
        let b = b.min(buf.len_bytes());
        let line = buf.line_of(b);
        let start = buf.line_start(line);
        (line + 1, buf.byte_to_char(b) - buf.byte_to_char(start) + 1)
    }

    /// The byte at a 1-based line and character column (clamped to the
    /// text and the line).
    pub fn at_line_col(&self, line: i64, col: i64) -> usize {
        let buf = self.doc.buffer();
        let line = (line.max(1) as usize - 1).min(buf.len_lines() - 1);
        let start = buf.line_start(line);
        let end = buf.line_end(line);
        let c0 = buf.byte_to_char(start);
        let c1 = buf.byte_to_char(end);
        buf.char_to_byte((c0 + col.max(1) as usize - 1).min(c1))
    }

    /// The primary selection in characters: (start, length).
    fn sel_chars(&self) -> (usize, usize) {
        let p = self.doc.selections().primary();
        let s = self.char_of(p.start());
        (s, self.char_of(p.end()) - s)
    }

    /// One selection, in characters from `start`, `len` long (clamped).
    fn select_chars(&mut self, start: usize, len: usize) {
        let a = self.byte_of(start);
        let b = self.byte_of(start.saturating_add(len));
        self.doc.set_selections(Selections::single(Selection::new(a, b)));
    }

    /// The old model's lines: none for an empty text; a final '\n' ends
    /// the last line rather than starting another.
    fn old_lines(&self) -> Vec<String> {
        let t = self.doc.text();
        if t.is_empty() {
            return Vec::new();
        }
        let t = t.strip_suffix('\n').unwrap_or(&t);
        t.split('\n').map(str::to_string).collect()
    }

    /// The lines either side of the caret's a screen reader's window holds
    /// (`objects::text_window`'s).
    pub const WINDOW_RADIUS: usize = 100;

    /// The first and last lines of [`text_window`](Self::text_window)'s
    /// window (0-based, inclusive).
    pub fn window_lines(&self, radius: usize) -> (usize, usize) {
        let buf = self.doc.buffer();
        let line = buf.line_of(self.doc.selections().primary().head);
        (line.saturating_sub(radius), (line + radius).min(buf.len_lines() - 1))
    }

    /// The lines within `radius` of the primary caret's, and the primary
    /// selection in them (characters, clamped to the window): what a
    /// page's text field mirrors for screen readers and input methods.
    pub fn text_window(&self, radius: usize) -> super::TextWindow {
        let buf = self.doc.buffer();
        let p = self.doc.selections().primary();
        let (first, last) = self.window_lines(radius);
        let (a, b) = (buf.line_start(first), buf.line_end(last));
        let c0 = buf.byte_to_char(a);
        let total = buf.byte_to_char(b) - c0;
        let s = (buf.byte_to_char(p.start().clamp(a, b)) - c0).min(total);
        let e = (buf.byte_to_char(p.end().clamp(a, b)) - c0).min(total);
        super::TextWindow { text: self.doc.slice(a..b).into_owned(), sel_start: s, sel_len: e - s, first: c0, read_only: self.doc.read_only, code: true }
    }

    // ---- edits ----

    /// The program's edit (allowed while ReadOnly; one undo step): `f`
    /// edits the document.
    fn program_edit<R>(&mut self, f: impl FnOnce(&mut Document) -> R) -> R {
        let ro = self.doc.read_only;
        self.doc.read_only = false;
        let r = f(&mut self.doc);
        self.doc.read_only = ro;
        self.edited();
        self.changed();
        r
    }

    /// The text becomes `new` by its smallest change (the common start and
    /// end kept), one undo step; the selection stays at the same
    /// characters (clamped).
    fn replace_all_minimal(&mut self, new: &str) {
        let (s, l) = self.sel_chars();
        let old = self.doc.text();
        let (ob, nb) = (old.as_bytes(), new.as_bytes());
        let mut pre = ob.iter().zip(nb).take_while(|(a, b)| a == b).count();
        while pre > 0 && !(old.is_char_boundary(pre) && new.is_char_boundary(pre)) {
            pre -= 1;
        }
        let max_suf = ob.len().min(nb.len()) - pre;
        let mut suf = ob.iter().rev().zip(nb.iter().rev()).take(max_suf).take_while(|(a, b)| a == b).count();
        while suf > 0 && !(old.is_char_boundary(ob.len() - suf) && new.is_char_boundary(nb.len() - suf)) {
            suf -= 1;
        }
        let change = Change::new(pre..ob.len() - suf, &new[pre..nb.len() - suf]);
        self.program_edit(|d| {
            let set = ChangeSet::new(vec![change], d.len_bytes()).expect("one change in range");
            let after = d.selections().clone();
            let _ = d.apply(set, after, EditKind::Command, 0);
        });
        self.select_chars(s, l);
    }

    /// Lines set (AddStrings, `Line(i) = s`): joined with '\n', ending with
    /// one, as RapidR always wrote them.
    fn set_lines(&mut self, lines: &[String]) {
        let mut s = lines.join("\n");
        if !s.is_empty() {
            s.push('\n');
        }
        self.replace_all_minimal(&s);
    }

    /// The program sets the text: a new document (no undo back past it),
    /// the caret at its start, not Modified; folds and diagnostics go.
    pub fn set_text(&mut self, s: &str) {
        let text = self.program_text(s);
        self.replace_document(&text);
        self.modified = false;
        self.changed();
    }

    fn replace_document(&mut self, text: &str) {
        let lang = self.doc.language().clone();
        let (tab, spaces, ro) = (self.doc.tab_size, self.doc.insert_spaces, self.doc.read_only);
        let mut doc = Document::new(text, lang);
        doc.tab_size = tab;
        doc.insert_spaces = spaces;
        doc.read_only = ro;
        doc.auto_close = self.opts.auto_close;
        doc.auto_indent = self.opts.auto_indent;
        doc.track_changes(true);
        self.doc = doc;
        self.generation += 1;
        self.line_edits.clear();
        self.folded.clear();
        self.folds.clear();
        self.fold_anchors.clear();
        self.fold_version = None;
        self.diagnostics.clear();
        self.semantic.clear();
        self.markers.retain(|m| m.at <= text.len());
        self.hide_popups();
    }

    /// Replaces the primary selection (SelText = s): the caret after it,
    /// Modified.
    pub fn replace_selection(&mut self, s: &str) {
        let new = self.program_text(s);
        let p = self.doc.selections().primary();
        let at = p.start();
        self.program_edit(|d| {
            let _ = d.replace_range(p.range(), &new, 0);
        });
        self.doc.set_selections(Selections::caret(at + new.len()));
        self.modified = true;
    }

    /// What every edit is followed by: markers, diagnostics, semantic
    /// colours, folds and popups' anchors mapped through what changed.
    pub fn edited(&mut self) {
        let sets = self.doc.take_applied();
        if sets.is_empty() {
            return;
        }
        let lines = self.doc.take_line_edits();
        if self.line_edits.len() + lines.len() > 4096 {
            self.line_edits.clear();
            self.generation += 1;
        } else {
            self.line_edits.extend(lines);
        }
        for set in &sets {
            for m in &mut self.markers {
                m.at = set.map_pos(m.at, Assoc::Before);
            }
            for (a, stays) in &mut self.anchors {
                *a = set.map_pos(*a, if *stays { Assoc::Before } else { Assoc::After });
            }
            for d in &mut self.diagnostics {
                d.start = set.map_pos(d.start, Assoc::After);
                d.end = set.map_pos(d.end, Assoc::Before).max(d.start);
            }
            self.semantic.retain_mut(|s| {
                let (a, b) = (set.map_pos(s.start, Assoc::After), set.map_pos(s.end, Assoc::Before));
                s.start = a;
                s.end = b;
                b > a
            });
            if let Some(c) = &mut self.completion {
                c.start = set.map_pos(c.start, Assoc::Before);
            }
            if let Some(s) = &mut self.signature {
                s.at = set.map_pos(s.at, Assoc::Before);
            }
        }
        // (markers on lines: each at its line's start, one per kind)
        let buf = self.doc.buffer();
        for m in &mut self.markers {
            m.at = buf.line_start(buf.line_of(m.at.min(buf.len_bytes())));
        }
        let mut seen = std::collections::HashSet::new();
        self.markers.retain(|m| seen.insert((m.at, m.kind.clone())));
        // (folds: kept, with their lines' starts through the changes, until
        // computed again at idle — what's folded stays folded)
        if !self.folds.is_empty() {
            let was_folded: Vec<bool> = self.folds.iter().map(|f| self.folded.contains(&f.start_line)).collect();
            for (a, b) in &mut self.fold_anchors {
                for set in &sets {
                    *a = set.map_pos(*a, Assoc::Before);
                    *b = set.map_pos(*b, Assoc::Before);
                }
            }
            let buf = self.doc.buffer();
            self.folded.clear();
            let mut kept = Vec::new();
            let mut anchors = Vec::new();
            for ((f, (a, b)), folded) in self.folds.iter().zip(&self.fold_anchors).zip(was_folded) {
                let (s, e) = (buf.line_of((*a).min(buf.len_bytes())), buf.line_of((*b).min(buf.len_bytes())));
                if e > s {
                    if folded {
                        self.folded.insert(s);
                    }
                    kept.push(FoldRange { start_line: s, end_line: e, kind: f.kind });
                    anchors.push((*a, *b));
                }
            }
            self.folds = kept;
            self.fold_anchors = anchors;
        }
        self.hover = None;
    }

    // ---- folds ----

    /// The fold ranges (computed again when the text changed since).
    pub fn fold_ranges(&mut self) -> &[FoldRange] {
        let v = self.doc.version();
        if self.fold_version != Some(v) {
            self.folds = self.doc.fold_ranges();
            let buf = self.doc.buffer();
            self.fold_anchors = self.folds.iter().map(|f| (buf.line_start(f.start_line), buf.line_start(f.end_line))).collect();
            self.fold_version = Some(v);
            let starts: BTreeSet<usize> = self.folds.iter().map(|f| f.start_line).collect();
            self.folded.retain(|l| starts.contains(l));
        }
        &self.folds
    }

    /// The fold ranges as last computed, without computing them again (the
    /// view's paint while typing; `fold_ranges` at idle).
    pub fn cached_fold_ranges(&self) -> (&[FoldRange], bool) {
        (&self.folds, self.fold_version == Some(self.doc.version()))
    }

    /// The innermost fold range containing 0-based `line` (one starting
    /// there first).
    pub fn fold_at(&mut self, line: usize) -> Option<FoldRange> {
        let ranges = self.fold_ranges();
        if let Some(f) = ranges.iter().filter(|f| f.start_line == line).min_by_key(|f| f.end_line - f.start_line) {
            return Some(*f);
        }
        ranges.iter().filter(|f| f.start_line < line && line <= f.end_line).min_by_key(|f| f.end_line - f.start_line).copied()
    }

    /// Folds the range at 0-based `line`; whether there was one.
    pub fn fold(&mut self, line: usize) -> bool {
        match self.fold_at(line) {
            Some(f) => {
                self.folded.insert(f.start_line);
                self.unhide_carets();
                self.changed();
                true
            }
            None => false,
        }
    }

    pub fn unfold(&mut self, line: usize) -> bool {
        let hit = self.fold_at(line).is_some_and(|f| self.folded.remove(&f.start_line)) || self.folded.remove(&line);
        if hit {
            self.changed();
        }
        hit
    }

    pub fn fold_all(&mut self) {
        let starts: Vec<usize> = self.fold_ranges().iter().map(|f| f.start_line).collect();
        self.folded.extend(starts);
        self.unhide_carets();
        self.changed();
    }

    pub fn unfold_all(&mut self) {
        self.folded.clear();
        self.changed();
    }

    /// The 0-based lines folds hide now (sorted, merged ranges, end
    /// exclusive).
    pub fn hidden_ranges(&self) -> Vec<std::ops::Range<usize>> {
        let mut out: Vec<std::ops::Range<usize>> = Vec::new();
        for f in &self.folds {
            if !self.folded.contains(&f.start_line) {
                continue;
            }
            let r = rapidr_editor::structure::hidden_lines(f);
            if r.is_empty() {
                continue;
            }
            match out.last_mut() {
                Some(last) if r.start <= last.end => last.end = last.end.max(r.end),
                _ => out.push(r),
            }
        }
        out.sort_by_key(|r| r.start);
        let mut merged: Vec<std::ops::Range<usize>> = Vec::new();
        for r in out {
            match merged.last_mut() {
                Some(last) if r.start <= last.end => last.end = last.end.max(r.end),
                _ => merged.push(r),
            }
        }
        merged
    }

    /// Unfolds what hides a caret (a caret never sits on a hidden line).
    pub fn unhide_carets(&mut self) {
        loop {
            let hidden = self.hidden_ranges();
            let buf = self.doc.buffer();
            let lines: Vec<usize> = self.doc.selections().iter().map(|s| buf.line_of(s.head)).collect();
            let hit: Vec<usize> = self
                .folds
                .iter()
                .filter(|f| self.folded.contains(&f.start_line))
                .filter(|f| {
                    let r = rapidr_editor::structure::hidden_lines(f);
                    lines.iter().any(|l| r.contains(l))
                })
                .map(|f| f.start_line)
                .collect();
            if hit.is_empty() || hidden.is_empty() {
                return;
            }
            for l in hit {
                self.folded.remove(&l);
            }
        }
    }

    // ---- markers, diagnostics ----

    pub fn add_marker(&mut self, line: i64, kind: &str) {
        let at = self.at_line_col(line, 1);
        let kind = kind.trim().to_lowercase();
        let kind = if kind.is_empty() { "bookmark".to_string() } else { kind };
        if !self.markers.iter().any(|m| m.at == at && m.kind == kind) {
            self.markers.push(Marker { at, kind });
            self.markers.sort_by_key(|m| m.at);
            self.changed();
        }
    }

    pub fn remove_marker(&mut self, line: i64, kind: &str) -> bool {
        let at = self.at_line_col(line, 1);
        let kind = kind.trim().to_lowercase();
        let before = self.markers.len();
        self.markers.retain(|m| !(m.at == at && (kind.is_empty() || m.kind == kind)));
        self.changed();
        self.markers.len() != before
    }

    pub fn clear_markers(&mut self, kind: &str) {
        let kind = kind.trim().to_lowercase();
        self.markers.retain(|m| !kind.is_empty() && m.kind != kind);
        self.changed();
    }

    /// The 1-based lines with a marker of `kind` (every kind when empty).
    pub fn marker_lines(&self, kind: &str) -> Vec<usize> {
        let kind = kind.trim().to_lowercase();
        let buf = self.doc.buffer();
        let mut lines: Vec<usize> = self.markers.iter().filter(|m| kind.is_empty() || m.kind == kind).map(|m| buf.line_of(m.at.min(buf.len_bytes())) + 1).collect();
        lines.sort_unstable();
        lines.dedup();
        lines
    }

    /// The markers on 0-based `line`.
    pub fn markers_on(&self, line: usize) -> impl Iterator<Item = &Marker> {
        let start = self.doc.buffer().line_start(line.min(self.doc.line_count() - 1));
        self.markers.iter().filter(move |m| m.at == start)
    }

    /// The service's diagnostics replace its last ones.
    pub fn set_service_diagnostics(&mut self, diags: Vec<Diagnostic>) {
        self.diagnostics.retain(|d| d.source != DiagSource::Service);
        self.diagnostics.extend(diags);
        self.diagnostics.sort_by_key(|d| (d.start, d.end));
    }

    /// The problems' counts: (errors, warnings, the caret line's messages).
    pub fn line_severity_summary(&self) -> (usize, usize, Vec<String>) {
        let errors = self.diagnostics.iter().filter(|d| d.severity == Severity::Error).count();
        let warnings = self.diagnostics.iter().filter(|d| d.severity == Severity::Warning).count();
        let buf = self.doc.buffer();
        let line = buf.line_of(self.doc.selections().primary().head);
        let (s, e) = (buf.line_start(line), buf.line_end(line));
        let here = self.diagnostics.iter().filter(|d| d.start <= e && d.end >= s).map(|d| format!("{}: {}", d.severity.name(), d.message)).collect();
        (errors, warnings, here)
    }

    /// The worst diagnostic on 0-based `line`.
    pub fn line_severity(&self, line: usize) -> Option<Severity> {
        let buf = self.doc.buffer();
        let (s, e) = (buf.line_start(line), buf.line_end(line));
        self.diagnostics.iter().filter(|d| d.start <= e && d.end >= s).map(|d| d.severity).min()
    }

    // ---- popups ----

    pub fn hide_popups(&mut self) {
        self.completion = None;
        self.hover = None;
        self.signature = None;
    }

    /// The start of the word before byte `at` (where a completion's
    /// replacement starts).
    pub fn word_start_before(&self, at: usize) -> usize {
        let buf = self.doc.buffer();
        let lang = self.doc.language();
        let mut s = at;
        while let Some(c) = buf.char_before(s) {
            if !lang.is_word_char(c) && !lang.keyword_suffixes.contains(c) {
                break;
            }
            s -= c.len_utf8();
        }
        s
    }

    /// ShowCompletion's items: JSON (`[{"label", "kind", "detail",
    /// "insert", "doc"}]`) or lines of tab-separated `label`, `kind`,
    /// `detail`, `insert`.
    pub fn parse_items(items: &str) -> Vec<Completion> {
        let t = items.trim();
        if t.starts_with('[') {
            if let Ok(serde_json::Value::Array(a)) = serde_json::from_str::<serde_json::Value>(t) {
                return a
                    .iter()
                    .filter_map(|v| {
                        let s = |k: &str| v.get(k).and_then(|x| x.as_str()).map(str::to_string);
                        let label = s("label").or_else(|| v.as_str().map(str::to_string))?;
                        let mut c = Completion::new(label, CompletionKind::named(&s("kind").unwrap_or_default()));
                        c.detail = s("detail").unwrap_or_default();
                        c.doc = s("doc").or_else(|| s("documentation")).unwrap_or_default();
                        c.insert = s("insert").or_else(|| s("insertText"));
                        c.snippet = v.get("snippet").and_then(serde_json::Value::as_bool).unwrap_or(false);
                        c.sort = s("sort").unwrap_or_default();
                        Some(c)
                    })
                    .collect();
            }
        }
        normalize_breaks(items)
            .split('\n')
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                let mut f = l.split('\t');
                let label = f.next().unwrap_or("").to_string();
                let mut c = Completion::new(label, CompletionKind::named(f.next().unwrap_or("")));
                c.detail = f.next().unwrap_or("").to_string();
                c.insert = f.next().filter(|s| !s.is_empty()).map(str::to_string);
                c
            })
            .collect()
    }

    /// Shows completion items at the primary caret.
    pub fn show_completion(&mut self, items: Vec<Completion>, from_service: bool, start: Option<usize>) {
        if items.is_empty() {
            self.completion = None;
        } else {
            let head = self.doc.selections().primary().head;
            let start = start.unwrap_or_else(|| self.word_start_before(head));
            self.completion = Some(CompletionList { items, start, selected: 0, from_service });
        }
        self.changed();
    }

    /// ShowSignature(Text, ActiveParam): the parameters are what the text's
    /// parentheses hold, split at commas.
    pub fn show_signature(&mut self, label: &str, active: i64) {
        let mut params = Vec::new();
        if let (Some(open), Some(close)) = (label.find('('), label.rfind(')')) {
            let mut start = open + 1;
            let mut depth = 0;
            for (i, c) in label[open + 1..close].char_indices() {
                let i = i + open + 1;
                match c {
                    '(' | '[' => depth += 1,
                    ')' | ']' => depth -= 1,
                    ',' if depth == 0 => {
                        params.push(trimmed(label, start, i));
                        start = i + 1;
                    }
                    _ => {}
                }
            }
            if close > start || !params.is_empty() {
                params.push(trimmed(label, start, close));
            }
        }
        let at = self.doc.selections().primary().head;
        self.signature = Some(SignatureInfo { label: label.to_string(), params, active: active.max(0) as usize, doc: String::new(), at });
        self.changed();
    }

    // ---- the program's properties and methods ----

    pub fn get(&self, prop: &str) -> Option<Value> {
        let (s, l) = self.sel_chars();
        let p = self.doc.selections().primary();
        Some(match prop {
            "text" => v_str(&self.text()),
            "seltext" => v_str(&self.doc.slice(p.range())),
            "selstart" => v_int(s as i64),
            "sellength" => v_int(l as i64),
            "modified" => flag(self.modified),
            "readonly" => flag(self.doc.read_only),
            "maxlength" => v_int(self.max_length),
            "charcase" => v_int(self.char_case),
            "linecount" => v_int(self.old_lines().len() as i64),
            "wherex" => v_int(self.line_col(p.head).1 as i64 - 1),
            "wherey" => v_int(self.line_col(p.head).0 as i64 - 1),
            // (a function read without parentheses: `S$ = Ed.GetSubList`)
            "getsublist" => v_str(&super::code::sub_list(&self.doc.text()).join("\n")),
            // (I2)
            "language" => v_str(&self.doc.language().id),
            "colorscheme" => v_str(&self.opts.color_scheme),
            "tabsize" => v_int(i64::from(self.doc.tab_size)),
            "insertspaces" => flag(self.doc.insert_spaces),
            "wordwrap" => flag(self.opts.word_wrap),
            "showlinenumbers" => flag(self.opts.show_line_numbers),
            "showfolding" => flag(self.opts.show_folding),
            "showminimap" => flag(self.opts.show_minimap),
            "showwhitespace" => flag(self.opts.show_whitespace),
            "highlightcurrentline" => flag(self.opts.highlight_current_line),
            "rulers" => v_str(&self.opts.rulers.iter().map(u32::to_string).collect::<Vec<_>>().join(",")),
            "caretline" => v_int(self.line_col(p.head).0 as i64),
            "caretcolumn" => v_int(self.line_col(p.head).1 as i64),
            "cursorcount" => v_int(self.doc.selections().len() as i64),
            "canundo" => flag(self.doc.can_undo()),
            "canredo" => flag(self.doc.can_redo()),
            "completiontrigger" => v_str(&self.opts.completion_trigger),
            "languageservice" => flag(self.opts.language_service),
            "keywordcase" => v_str(&self.opts.keyword_case),
            "filename" => v_str(&self.file_name),
            "fontname" => v_str(&self.opts.font_name),
            "fontsize" => v_int(self.opts.font_size),
            "autoclose" => flag(self.opts.auto_close),
            "autoindent" => flag(self.opts.auto_indent),
            "lineending" => v_str(match self.file_ending {
                LineEnding::Lf => "LF",
                LineEnding::CrLf => "CRLF",
                LineEnding::Cr => "CR",
            }),
            "encoding" => v_str(match self.encoding {
                Encoding::Utf8 => "UTF-8",
                Encoding::Utf8Bom => "UTF-8 BOM",
                Encoding::Latin1 => "Latin-1",
            }),
            "diagnosticcount" => v_int(self.diagnostics.len() as i64),
            "foldcount" => v_int(self.folded.len() as i64),
            "outline" => v_str(&super::code::sub_list(&self.doc.text()).join("\n")),
            _ => return None,
        })
    }

    pub fn set(&mut self, prop: &str, val: &Value) -> bool {
        let b = val.to_bool();
        match prop {
            "text" => self.set_text(&val.to_string_val()),
            "seltext" => self.replace_selection(&val.to_string_val()),
            // (setting SelStart drops the selection and the other carets)
            "selstart" => {
                self.select_chars(val.to_i64().max(0) as usize, 0);
                self.changed();
            }
            "sellength" => {
                let (s, _) = self.sel_chars();
                self.select_chars(s, val.to_i64().max(0) as usize);
                self.changed();
            }
            "modified" => {
                self.modified = b;
                if !b {
                    self.doc.mark_saved();
                }
            }
            "readonly" => {
                self.doc.read_only = b;
                self.changed();
            }
            "maxlength" => self.max_length = val.to_i64().max(0),
            "charcase" => {
                self.char_case = val.to_i64().clamp(0, 2);
                let t = self.case_text(&self.doc.text());
                if *t != *self.doc.text() {
                    self.replace_all_minimal(&t);
                }
                self.changed();
            }
            "language" => {
                let name = val.to_string_val();
                self.language_set = name.clone();
                let lang = language_named(&name);
                self.doc.set_language(lang);
                self.fold_version = None;
                self.semantic.clear();
                self.changed();
            }
            "colorscheme" => self.opts.color_scheme = val.to_string_val(),
            "tabsize" => {
                self.doc.tab_size = val.to_i64().clamp(1, 16) as u32;
                self.fold_version = None;
            }
            "insertspaces" => self.doc.insert_spaces = b,
            "wordwrap" => self.opts.word_wrap = b,
            "showlinenumbers" => self.opts.show_line_numbers = b,
            "showfolding" => self.opts.show_folding = b,
            "showminimap" => self.opts.show_minimap = b,
            "showwhitespace" => self.opts.show_whitespace = b,
            "highlightcurrentline" => self.opts.highlight_current_line = b,
            "rulers" => {
                self.opts.rulers = val.to_string_val().split([',', ' ', ';']).filter_map(|s| s.trim().parse::<u32>().ok()).filter(|&c| c > 0).collect();
            }
            "caretline" | "caretcolumn" => {
                let (line, col) = self.line_col(self.doc.selections().primary().head);
                let (line, col) = if prop == "caretline" { (val.to_i64(), col as i64) } else { (line as i64, val.to_i64()) };
                let at = self.at_line_col(line, col);
                self.doc.set_selections(Selections::caret(at));
                self.unhide_carets();
                self.reveal += 1;
            }
            "completiontrigger" => self.opts.completion_trigger = val.to_string_val(),
            "languageservice" => self.opts.language_service = b,
            "keywordcase" => {
                let v = val.to_string_val().trim().to_lowercase();
                self.opts.keyword_case = if matches!(v.as_str(), "upper" | "lower" | "proper" | "preserve") { v } else { "preserve".into() };
            }
            "filename" => self.file_name = val.to_string_val(),
            "fontname" => self.opts.font_name = val.to_string_val(),
            "fontsize" => self.opts.font_size = val.to_i64().clamp(4, 96),
            "autoclose" => {
                self.opts.auto_close = b;
                self.doc.auto_close = b;
            }
            "autoindent" => {
                self.opts.auto_indent = b;
                self.doc.auto_indent = b;
            }
            "lineending" => {
                self.file_ending = match val.to_string_val().trim().to_ascii_uppercase().as_str() {
                    "CRLF" | "CR LF" | "WINDOWS" => LineEnding::CrLf,
                    "CR" => LineEnding::Cr,
                    _ => LineEnding::Lf,
                };
            }
            _ => return false,
        }
        self.changed();
        true
    }

    /// The program's methods; `None` for one it doesn't have.
    pub fn call(&mut self, method: &str, args: &[Value]) -> Option<Value> {
        let arg = |i: usize| args.get(i).map(Value::to_string_val).unwrap_or_default();
        let num = |i: usize, d: i64| args.get(i).map_or(d, Value::to_i64);
        match method {
            // Line(i) reads a line; Line(i) = s (here with s) changes it.
            "line" | "lines" | "line=" => {
                let i = usize::try_from(num(0, -1)).ok();
                let mut lines = self.old_lines();
                if args.len() >= 2 {
                    if let Some(l) = i.and_then(|i| lines.get_mut(i)) {
                        *l = self.program_text(&arg(1));
                        self.set_lines(&lines);
                    }
                    return Some(Value::Null);
                }
                return Some(v_str(i.and_then(|i| lines.get(i)).map_or("", |s| s.as_str())));
            }
            "addstrings" | "addstring" | "addlines" | "add" | "additems" => {
                let mut lines = self.old_lines();
                lines.extend(args.iter().map(|a| self.program_text(&a.to_string_val())));
                self.set_lines(&lines);
            }
            "clear" => {
                self.program_edit(|d| {
                    let n = d.len_bytes();
                    let set = ChangeSet::new(vec![Change::delete(0..n)], n).expect("whole text");
                    let _ = d.apply(set, Selections::caret(0), EditKind::Command, 0);
                });
            }
            "selectall" => {
                self.doc.select_all();
                self.changed();
            }
            "clearselection" => {
                if !self.doc.read_only {
                    self.replace_selection("");
                }
            }
            "getsublist" => return Some(v_str(&super::code::sub_list(&self.doc.text()).join("\n"))),
            "gotosub" => {
                if let Some(line) = super::code::sub_line(&self.doc.text(), &arg(0)) {
                    self.goto_line0(line);
                }
            }
            "gotoline" => self.goto_line0(num(0, 0).max(0) as usize),
            // ---- I2 ----
            "undo" => {
                if self.doc.undo() {
                    self.after_history();
                }
            }
            "redo" => {
                if self.doc.redo() {
                    self.after_history();
                }
            }
            "find" => {
                let found = self.find(&arg(0), &arg(1), true);
                return Some(flag(found));
            }
            "findnext" | "findprevious" => {
                let Some((text, opts)) = self.last_find.clone() else { return Some(flag(false)) };
                let found = self.find(&text, &opts, method == "findnext");
                return Some(flag(found));
            }
            "replace" => {
                let q = SearchQuery::with_options(arg(0), &arg(2));
                self.last_find = Some((arg(0), arg(2)));
                let done = self.program_edit(|d| d.replace_next(&q, &arg(1), 0).unwrap_or(false));
                self.reveal += 1;
                return Some(v_int(i64::from(done)));
            }
            "replaceall" => {
                let opts = arg(2);
                let q = SearchQuery::with_options(arg(0), &opts);
                let in_sel = opts.to_ascii_lowercase().contains("selection");
                self.last_find = Some((arg(0), opts));
                let n = self.program_edit(|d| d.replace_all(&q, &arg(1), in_sel, 0).unwrap_or(0));
                return Some(v_int(n as i64));
            }
            "addcursor" => {
                let at = self.at_line_col(num(0, 1), num(1, 1));
                self.doc.add_cursor(at);
                self.changed();
            }
            "selectnextoccurrence" => {
                let r = self.doc.select_next_occurrence();
                self.reveal += 1;
                self.changed();
                return Some(flag(r));
            }
            "clearcursors" => {
                self.doc.clear_cursors();
                self.changed();
            }
            "fold" => return Some(flag(self.fold(num(0, 1).max(1) as usize - 1))),
            "unfold" => return Some(flag(self.unfold(num(0, 1).max(1) as usize - 1))),
            "foldall" => self.fold_all(),
            "unfoldall" => self.unfold_all(),
            "inserttext" => {
                let t = self.program_text(&arg(0));
                self.program_edit(|d| {
                    let _ = d.paste(&t, 0);
                });
                self.reveal += 1;
            }
            "replacerange" => {
                let a = self.at_line_col(num(0, 1), num(1, 1));
                let b = self.at_line_col(num(2, 1), num(3, 1));
                let t = self.program_text(&arg(4));
                let (a, b) = (a.min(b), a.max(b));
                self.program_edit(|d| {
                    let _ = d.replace_range(a..b, &t, 0);
                });
            }
            "applyedits" => return Some(flag(self.apply_edits_json(&arg(0)))),
            "setdiagnostics" => {
                let diags = self.parse_diagnostics(&arg(0));
                self.diagnostics.retain(|d| d.source != DiagSource::Program);
                self.diagnostics.extend(diags);
                self.diagnostics.sort_by_key(|d| (d.start, d.end));
            }
            "cleardiagnostics" => self.diagnostics.clear(),
            "addmarker" => self.add_marker(num(0, 1), &arg(1)),
            "removemarker" => return Some(flag(self.remove_marker(num(0, 1), &arg(1)))),
            "clearmarkers" => self.clear_markers(&arg(0)),
            "getmarkers" => return Some(v_str(&self.marker_lines(&arg(0)).iter().map(usize::to_string).collect::<Vec<_>>().join(","))),
            "hasmarker" => {
                let line = num(0, 1).max(1) as usize - 1;
                let kind = arg(1).trim().to_lowercase();
                return Some(flag(self.markers_on(line).any(|m| kind.is_empty() || m.kind == kind)));
            }
            "showcompletion" => {
                let items = Self::parse_items(&arg(0));
                self.show_completion(items, false, None);
            }
            "showhover" => {
                let text = arg(0);
                let at = self.hover_request.unwrap_or(self.doc.selections().primary().head);
                let r = self.doc.word_at(at).unwrap_or(at..at);
                self.hover = (!text.trim().is_empty()).then(|| HoverInfo { text, start: r.start, end: r.end });
            }
            "showsignature" => self.show_signature(&arg(0), num(1, 0)),
            "hidepopups" => self.hide_popups(),
            "triggercompletion" => self.requests.push(Request::Completion),
            "triggersignature" => self.requests.push(Request::Signature),
            "triggerhover" => {
                let at = if args.len() >= 2 { self.at_line_col(num(0, 1), num(1, 1)) } else { self.doc.selections().primary().head };
                self.requests.push(Request::Hover(at));
            }
            "formatdocument" => self.requests.push(Request::Format),
            "gotodefinition" => self.requests.push(Request::Definition),
            "findreferences" => self.requests.push(Request::References),
            "rename" => self.requests.push(Request::Rename(arg(0))),
            "openfind" => self.requests.push(Request::Find(if arg(0).is_empty() { "find".into() } else { arg(0).to_lowercase() })),
            "beginupdate" => self.update_depth += 1,
            "endupdate" => self.update_depth = self.update_depth.saturating_sub(1),
            "gotolinecolumn" => {
                let at = self.at_line_col(num(0, 1), num(1, 1));
                self.doc.set_selections(Selections::caret(at));
                self.unhide_carets();
                self.reveal += 1;
            }
            "togglecomment" => {
                self.program_edit(|d| {
                    let _ = d.toggle_line_comment(0);
                });
            }
            "indent" => {
                self.program_edit(|d| {
                    let _ = d.indent_lines(0);
                });
            }
            "outdent" => {
                self.program_edit(|d| {
                    let _ = d.outdent_lines(0);
                });
            }
            "copytext" => return Some(v_str(&self.doc.copy_text())),
            _ => return None,
        }
        self.changed();
        Some(Value::Null)
    }

    fn after_history(&mut self) {
        self.edited();
        self.modified = self.doc.is_modified();
        self.unhide_carets();
        self.reveal += 1;
    }

    /// The caret at 0-based line `n`'s start (past the last, the text's
    /// end), scrolled into view: GotoLine's, as RapidR always did it.
    fn goto_line0(&mut self, n: usize) {
        let c = super::code::line_start(&self.doc.text(), n);
        self.select_chars(c, 0);
        self.unhide_carets();
        self.reveal += 1;
    }

    /// Find(Text, Options): selects the next match after the selection (or
    /// before it); whether there was one.
    pub fn find(&mut self, text: &str, opts: &str, forward: bool) -> bool {
        self.last_find = Some((text.to_string(), opts.to_string()));
        if text.is_empty() {
            return false;
        }
        let q = SearchQuery::with_options(text, opts);
        let found = self.doc.select_next_match(&q, forward).unwrap_or(false);
        if found {
            self.unhide_carets();
            self.reveal += 1;
        }
        self.changed();
        found
    }

    /// A position of a JSON edit or diagnostic: `line` / `column` (1-based)
    /// or `start` (a character offset from 0); `end…` for its end.
    fn json_pos(&self, v: &serde_json::Value, end: bool) -> Option<usize> {
        let n = |k: &str| v.get(k).and_then(serde_json::Value::as_i64);
        let (lk, ck, ok) = if end { ("endLine", "endColumn", "end") } else { ("line", "column", "start") };
        if let Some(line) = n(lk).or_else(|| n(&lk.to_lowercase())) {
            return Some(self.at_line_col(line, n(ck).or_else(|| n(&ck.to_lowercase())).unwrap_or(1)));
        }
        n(ok).map(|c| self.byte_of(c.max(0) as usize))
    }

    /// ApplyEdits(Json): `[{"line", "column", "endLine", "endColumn",
    /// "text"}]` (or `start` / `end` character offsets), one undo step;
    /// refused whole (False) when two overlap.
    pub fn apply_edits_json(&mut self, json: &str) -> bool {
        let Ok(serde_json::Value::Array(a)) = serde_json::from_str::<serde_json::Value>(json) else { return false };
        let mut changes = Vec::new();
        for e in &a {
            let Some(s) = self.json_pos(e, false) else { return false };
            let end = self.json_pos(e, true).unwrap_or(s);
            let t = e.get("text").and_then(serde_json::Value::as_str).unwrap_or("");
            changes.push(Change::new(s.min(end)..s.max(end), self.program_text(t)));
        }
        changes.sort_by_key(|c| c.range.start);
        self.program_edit(|d| d.apply_edits(changes, 0).is_ok())
    }

    fn parse_diagnostics(&self, json: &str) -> Vec<Diagnostic> {
        let Ok(serde_json::Value::Array(a)) = serde_json::from_str::<serde_json::Value>(json) else { return Vec::new() };
        a.iter()
            .filter_map(|e| {
                let s = self.json_pos(e, false)?;
                let end = self.json_pos(e, true).unwrap_or_else(|| self.doc.word_at(s).map_or(s, |r| r.end));
                let str_of = |k: &str| e.get(k).map(|v| v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string())).unwrap_or_default();
                Some(Diagnostic {
                    start: s.min(end),
                    end: s.max(end),
                    severity: Severity::named(&str_of("severity")),
                    message: str_of("message"),
                    code: str_of("code"),
                    source: DiagSource::Program,
                })
            })
            .collect()
    }

    /// LoadFromFile's text: its encoding and line breaks remembered, the
    /// language from its name unless the program chose one.
    pub fn load(&mut self, path: &str, bytes: &[u8]) {
        let (text, enc) = decode(bytes);
        self.encoding = enc;
        self.file_ending = LineEnding::detect(&text);
        let text = self.program_text(&text);
        if self.language_set.is_empty() {
            self.doc.set_language(Languages::builtin().for_path(path));
            if self.doc.language().id == "plaintext" && !path.contains('.') {
                self.doc.set_language(basic());
            }
        }
        self.replace_document(&text);
        self.file_name = path.to_string();
        self.modified = false;
        self.changed();
    }

    /// SaveToFile's bytes (the file's own line breaks and encoding).
    pub fn saved_bytes(&self) -> Vec<u8> {
        let text = self.doc.text();
        let text = match self.file_ending {
            LineEnding::Lf => text.to_string(),
            le => text.replace('\n', le.as_str()),
        };
        encode(&text, self.encoding)
    }

    /// The text was saved to `path`.
    pub fn saved(&mut self, path: &str) {
        self.file_name = path.to_string();
        self.doc.mark_saved();
        self.modified = false;
        self.changed();
    }
}

fn trimmed(s: &str, a: usize, b: usize) -> (usize, usize) {
    let piece = &s[a..b];
    let lead = piece.len() - piece.trim_start().len();
    let trail = piece.len() - piece.trim_end().len();
    (a + lead, (b - trail).max(a + lead))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: Option<Value>) -> String {
        v.unwrap().to_string_val()
    }
    fn n(v: Option<Value>) -> i64 {
        v.unwrap().to_i64()
    }

    /// The `code_editor` GUI case's answers, as the old model gave them.
    #[test]
    fn the_old_api_answers_as_before() {
        let mut c = CodeEditor::new();
        c.set("text", &v_str("DIM a\r\nSUB Foo\r\nEND SUB\r\nFUNCTION Bar(x)"));
        assert_eq!(s(c.get("text")), "DIM a\nSUB Foo\nEND SUB\nFUNCTION Bar(x)");
        assert_eq!(s(c.call("getsublist", &[])), "Foo\nBar");
        assert_eq!(c.get("getsublist"), c.call("getsublist", &[]));
        let rev = c.revision;
        c.call("gotosub", &[v_str("bar")]);
        assert_eq!((n(c.get("selstart")), n(c.get("sellength")), c.reveal), (22, 0, 1));
        assert!(c.revision > rev && !c.modified);
        c.call("gotoline", &[v_int(1)]);
        assert_eq!((n(c.get("wherey")), n(c.get("selstart"))), (1, 6));
        c.set("sellength", &v_int(9));
        assert_eq!(s(c.get("seltext")), "SUB Foo\nE");

        // the fixture's own steps
        let mut e = CodeEditor::new();
        let q = "\"";
        e.set("text", &v_str(&format!("DIM a AS INTEGER\nSUB Hello(x AS INTEGER)\n  PRINT {q}hi{q} ' greet\nEND SUB\nFUNCTION Twice(n)\n  Twice = n * 2.5\nEND FUNCTION")));
        assert_eq!(n(e.get("linecount")), 7);
        assert_eq!(s(e.call("line", &[v_int(1)])), "SUB Hello(x AS INTEGER)");
        assert_eq!(s(e.get("text")).len(), 118);
        e.call("gotosub", &[v_str("twice")]);
        assert_eq!((n(e.get("wherey")), n(e.get("selstart"))), (4, 70));
        e.call("gotoline", &[v_int(2)]);
        e.set("sellength", &v_int(7));
        assert_eq!(s(e.get("seltext")), "  PRINT");
        e.set("seltext", &v_str("  BEEP"));
        assert_eq!(s(e.call("line", &[v_int(2)])), "  BEEP \"hi\" ' greet");
        for i in 1..=30 {
            e.call("addstrings", &[v_str(&format!("' more {i}"))]);
        }
        assert_eq!(n(e.get("linecount")), 37);
        e.call("gotoline", &[v_int(33)]);
        assert_eq!(n(e.get("wherey")), 33);
        assert!(s(e.get("text")).ends_with("' more 30\n"));
        assert_eq!(n(e.get("modified")), -1, "SelText modifies");

        // Line(i) = s, Clear, SelectAll, an empty text's lines
        let mut m = CodeEditor::new();
        m.call("addstrings", &[v_str("one"), v_str("two")]);
        assert_eq!(s(m.get("text")), "one\ntwo\n");
        m.call("line", &[v_int(0), v_str("ONE")]);
        assert_eq!(s(m.get("text")), "ONE\ntwo\n");
        m.call("selectall", &[]);
        assert_eq!(n(m.get("sellength")), 8);
        m.call("clear", &[]);
        assert_eq!((s(m.get("text")), n(m.get("linecount"))), (String::new(), 0));
        m.set("charcase", &v_int(1));
        m.set("text", &v_str("hello"));
        assert_eq!(s(m.get("text")), "HELLO");
    }

    #[test]
    fn new_members_count_from_one() {
        let mut c = CodeEditor::new();
        c.set("text", &v_str("SUB A\n  PRINT 1\nEND SUB\nx = 2\n"));
        c.set("caretline", &v_int(2));
        c.set("caretcolumn", &v_int(3));
        assert_eq!((n(c.get("caretline")), n(c.get("caretcolumn")), n(c.get("wherey")), n(c.get("wherex"))), (2, 3, 1, 2));
        c.call("addcursor", &[v_int(4), v_int(1)]);
        assert_eq!(n(c.get("cursorcount")), 2);
        c.call("clearcursors", &[]);
        assert_eq!(n(c.get("cursorcount")), 1);
        c.call("replacerange", &[v_int(4), v_int(5), v_int(4), v_int(6), v_str("42")]);
        assert_eq!(s(c.call("line", &[v_int(3)])), "x = 42");
        assert_eq!(n(c.get("canundo")), -1);
        c.call("undo", &[]);
        assert_eq!(s(c.call("line", &[v_int(3)])), "x = 2");
        c.call("redo", &[]);
        assert_eq!(s(c.call("line", &[v_int(3)])), "x = 42");
        // folds: SUB … END SUB
        assert_eq!(n(c.call("fold", &[v_int(1)])), -1);
        assert_eq!(c.hidden_ranges(), vec![1..3]);
        c.call("unfoldall", &[]);
        assert!(c.hidden_ranges().is_empty());
        // markers follow their lines
        c.call("addmarker", &[v_int(4), v_str("breakpoint")]);
        c.call("gotolinecolumn", &[v_int(1), v_int(1)]);
        c.call("inserttext", &[v_str("' top\n")]);
        assert_eq!(s(c.call("getmarkers", &[v_str("breakpoint")])), "5");
        // find, replace all
        assert_eq!(n(c.call("find", &[v_str("print"), v_str("")])), -1);
        assert_eq!(n(c.get("caretline")), 3);
        assert_eq!(n(c.call("replaceall", &[v_str("PRINT"), v_str("BEEP"), v_str("case")])), 1);
        // ApplyEdits as one step; a JSON diagnostic
        assert_eq!(n(c.call("applyedits", &[v_str(r#"[{"line":1,"column":1,"endLine":1,"endColumn":6,"text":"REM"},{"start":6,"end":6,"text":"' "}]"#)])), -1);
        assert!(s(c.get("text")).starts_with("REM\n' SUB A"), "{}", s(c.get("text")));
        c.call("setdiagnostics", &[v_str(r#"[{"line":2,"column":5,"endColumn":8,"severity":"warning","message":"hm"}]"#)]);
        assert_eq!(c.diagnostics.len(), 1);
        assert_eq!(c.line_severity(1), Some(Severity::Warning));
        // completion items both ways
        let items = CodeEditor::parse_items("Caption\tproperty\tSTRING\nShow\tmethod");
        assert_eq!((items.len(), items[0].kind, items[1].kind), (2, CompletionKind::Property, CompletionKind::Method));
        let items = CodeEditor::parse_items(r#"[{"label":"Left","kind":"property","detail":"INTEGER"}]"#);
        assert_eq!(items[0].detail, "INTEGER");
        c.show_signature("MID$(S AS STRING, Start, [Count])", 1);
        let sig = c.signature.clone().unwrap();
        assert_eq!(sig.params.iter().map(|&(a, b)| &sig.label[a..b]).collect::<Vec<_>>(), ["S AS STRING", "Start", "[Count]"]);
    }

    #[test]
    fn files_keep_their_breaks_and_encoding() {
        let mut c = CodeEditor::new();
        c.load("x.bas", b"PRINT \"\xe9\"\r\nEND\r\n");
        assert_eq!(c.encoding, Encoding::Latin1);
        assert_eq!(c.file_ending, LineEnding::CrLf);
        assert_eq!(s(c.get("text")), "PRINT \"\u{e9}\"\nEND\n");
        assert_eq!(c.saved_bytes(), b"PRINT \"\xe9\"\r\nEND\r\n");
        c.load("y.sql", "SELECT 1\n".as_bytes());
        assert_eq!(s(c.get("language")), "sql");
        assert_eq!(c.encoding, Encoding::Utf8);
    }
}
