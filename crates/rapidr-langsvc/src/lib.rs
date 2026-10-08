//! RapidR's language service (docs/ide-plan.md, I3): IntelliSense that
//! knows the program — completion, hover, signature help, go to
//! definition, references, rename, diagnostics in the compiler's own words,
//! outline, semantic tokens, formatting and code actions. One engine for
//! RapidR Studio, `rapidr lsp` (VS Code and other editors) and the web IDE.
//!
//! ```text
//! Analysis::new(options) → update(file, text) → completions(file, offset) / hover / signature /
//!   definition / references / rename / diagnostics(file) / outline(file) / semantic_tokens(file) /
//!   format(file, indent) / code_actions(file, range)
//! ```
//!
//! Offsets are bytes of the file's text ([`LineIndex`] converts to the
//! UTF-16 positions editors speak).
//!
//! It stands on the I0 foundations: [`front`] is the parser for tools
//! (`rapidr_parser::tools`), [`model`] the compiler's own semantic model
//! (`rapidr_bcgen::semantic`), and everything it says about the language
//! — components, members, builtins, statements, directives, constants,
//! their docs, origins (RapidQ's or RapidR's) and gaps — comes from the
//! language registry, `rapidr_lang` (the IDE's one source).

pub mod case;
pub mod diagnostics;
pub mod editor;
pub mod front;
pub mod model;
pub mod text;

mod compat;
mod complete;
mod context;
mod format;
mod hover;
mod navigate;
mod signature;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use case::{CaseOptions, CaseScope, IdentifierCase, KeywordCase};
pub use diagnostics::FileDiagnostic;
pub use front::{Location, Parsed};
pub use model::SemanticModel;
pub use rapidr_diagnostics::Severity;
pub use text::LineIndex;

/// How a project is analysed.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// A RapidQ-compatible project: RapidR's extensions are reported
    /// (docs/q-and-r-components.md §5).
    pub rapidq_compatible: bool,
    /// More folders to look for `$INCLUDE` files in (RapidQ's `include\`).
    pub include_dirs: Vec<PathBuf>,
    /// Automatic case of the language's words and the program's names
    /// (on typing and in formatting).
    pub case: CaseOptions,
    /// Confined to these folders (the language server's workspace and the
    /// folders of the files the editor opened; docs/security-audit.md
    /// SEC-18): a file elsewhere is never read from the disk — not as a
    /// document asked about, not as an `$INCLUDE`. `None`: no confinement
    /// (RapidR Studio, which opens a project's own files).
    pub roots: Option<Vec<PathBuf>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompletionKind {
    Keyword,
    Builtin,
    Sub,
    Function,
    Method,
    Property,
    Event,
    Variable,
    Parameter,
    Field,
    Constant,
    Component,
    Type,
    Directive,
    Label,
}

/// One completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    pub label: String,
    pub kind: CompletionKind,
    /// One line beside the label (a type, a signature).
    pub detail: Option<String>,
    /// Markdown.
    pub doc: Option<String>,
    /// What to insert, when not the label (snippet syntax when `snippet`).
    pub insert: Option<String>,
    pub snippet: bool,
    /// The order: smaller first.
    pub sort: String,
}

/// The completions at a place, and the text they replace (the word typed
/// so far).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Completions {
    pub items: Vec<Completion>,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hover {
    pub markdown: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    pub label: String,
    /// Each parameter's range in `label` (bytes).
    pub params: Vec<(usize, usize)>,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelp {
    pub signatures: Vec<Signature>,
    pub active_signature: usize,
    pub active_param: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutlineKind {
    Sub,
    Function,
    Type,
    Field,
    Method,
    Event,
    Component,
    Constant,
    Variable,
    Label,
}

/// One entry of a file's outline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineItem {
    pub name: String,
    pub detail: Option<String>,
    pub kind: OutlineKind,
    /// The whole statement.
    pub start: usize,
    pub end: usize,
    /// Its name.
    pub name_start: usize,
    pub name_end: usize,
    pub children: Vec<OutlineItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// Edits to several files.
pub type WorkspaceEdit = Vec<(PathBuf, Vec<TextEdit>)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Function,
    Variable,
    Parameter,
    Property,
    Type,
    Constant,
    Component,
    Label,
}

/// Token modifiers (bits).
pub mod modifiers {
    pub const DECLARATION: u32 = 1;
    pub const READONLY: u32 = 2;
    pub const STATIC: u32 = 4;
    pub const GLOBAL: u32 = 8;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemanticToken {
    pub start: usize,
    pub end: usize,
    pub kind: TokenKind,
    pub modifiers: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeAction {
    pub title: String,
    pub edit: WorkspaceEdit,
    /// Fixes this diagnostic.
    pub fixes: Option<FileDiagnostic>,
    pub preferred: bool,
}

/// A file's analysis: one parse of the program it belongs to and the
/// program's names.
#[derive(Debug)]
pub struct Snapshot {
    pub parsed: Arc<Parsed>,
    pub model: Arc<SemanticModel>,
    /// (a view of an older analysis while the user types: the edits made
    /// since, in order, which offsets are mapped across)
    edits: Vec<Shift>,
}

/// One edit of a file since an analysis was made of it: its bytes
/// `prefix..old_end` then are `prefix..new_end` now (the rest the same).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Shift {
    file: PathBuf,
    prefix: usize,
    old_end: usize,
    new_end: usize,
}

impl Shift {
    /// The change from `old` to `new`: the common start and end kept.
    fn between(file: &Path, old: &str, new: &str) -> Shift {
        let (a, b) = (old.as_bytes(), new.as_bytes());
        let mut prefix = a.iter().zip(b).take_while(|(x, y)| x == y).count();
        while prefix > 0 && !(old.is_char_boundary(prefix) && new.is_char_boundary(prefix)) {
            prefix -= 1;
        }
        let max = a.len().min(b.len()) - prefix;
        let mut suffix = a.iter().rev().zip(b.iter().rev()).take(max).take_while(|(x, y)| x == y).count();
        while suffix > 0 && !(old.is_char_boundary(a.len() - suffix) && new.is_char_boundary(b.len() - suffix)) {
            suffix -= 1;
        }
        Shift { file: file.to_path_buf(), prefix, old_end: a.len() - suffix, new_end: b.len() - suffix }
    }

    /// Today's byte `offset` in the analysed text (inside the edit: its
    /// start).
    fn to_old(&self, offset: usize) -> usize {
        if offset <= self.prefix {
            offset
        } else if offset >= self.new_end {
            offset - self.new_end + self.old_end
        } else {
            self.prefix
        }
    }

    /// The analysed text's byte `offset` today.
    fn to_new(&self, offset: usize) -> usize {
        if offset <= self.prefix {
            offset
        } else if offset >= self.old_end {
            offset - self.old_end + self.new_end
        } else {
            self.prefix
        }
    }
}

impl Snapshot {
    fn new(parsed: Parsed, model: SemanticModel) -> Snapshot {
        Snapshot { parsed: Arc::new(parsed), model: Arc::new(model), edits: Vec::new() }
    }

    /// The preprocessed offset of byte `offset` of `file`.
    pub fn pre_offset(&self, file: &Path, offset: usize) -> Option<usize> {
        let offset = self.edits.iter().rev().filter(|e| e.file == file).fold(offset, |o, e| e.to_old(o));
        self.parsed.to_preprocessed(file, offset)
    }

    /// Where the model's span is in the files.
    pub fn locate(&self, span: rapidr_diagnostics::TextSpan) -> Option<Location> {
        let mut loc = self.parsed.locate(span)?;
        for e in self.edits.iter().filter(|e| e.file == loc.file) {
            loc.start = e.to_new(loc.start);
            loc.end = e.to_new(loc.end).max(loc.start);
        }
        Some(loc)
    }

    /// Whether this is an older analysis seen through the edits since.
    pub fn is_stale(&self) -> bool {
        !self.edits.is_empty()
    }

    /// A multi-line span of the AST in the files (its start and end located
    /// separately).
    pub fn locate_range(&self, span: rapidr_diagnostics::TextSpan) -> Option<Location> {
        let start = self.locate(span)?;
        let last = rapidr_diagnostics::TextSpan::new(span.end.saturating_sub(1).max(span.start), span.end);
        let end = self.locate(last).filter(|l| l.file == start.file).map_or(start.end, |l| l.end);
        Some(Location { end: end.max(start.start), ..start })
    }
}

/// The language service over a set of open files.
#[derive(Debug, Default)]
pub struct Analysis {
    options: Options,
    /// The editor's text of each open file.
    docs: HashMap<PathBuf, String>,
    snapshots: HashMap<PathBuf, Arc<Snapshot>>,
    /// Each main file's diagnostics, until a file changes.
    diagnostics: HashMap<PathBuf, Vec<FileDiagnostic>>,
    /// A main file's last analysis and the text it was made from, kept
    /// after the file changed: what's asked as the user types (completion,
    /// signatures, hovers, case) is answered from it — "the last good
    /// model", mapped across the edit — and the analysis is made again
    /// only when asked for what must be exact (diagnostics, definitions,
    /// references, rename, the outline) or with [`Analysis::refresh`].
    stale: HashMap<PathBuf, (Arc<Snapshot>, Vec<Shift>)>,
}

impl Analysis {
    pub fn new(options: Options) -> Self {
        Analysis { options, ..Default::default() }
    }

    pub fn options(&self) -> &Options {
        &self.options
    }

    pub fn set_options(&mut self, options: Options) {
        self.options = options;
        self.snapshots.clear();
        self.diagnostics.clear();
        self.stale.clear();
    }

    /// A file's new text (the editor's, whole).
    pub fn update(&mut self, file: impl Into<PathBuf>, text: impl Into<String>) {
        let file = file.into();
        let text = text.into();
        if self.docs.get(&file) == Some(&text) {
            return;
        }
        // (the file's own analysis is kept for typing's quick answers, with
        // each edit since: a typed key's is one small change)
        if !is_inc(&file) {
            if let Some(old) = self.docs.get(&file) {
                let edit = Shift::between(&file, old, &text);
                if let Some(s) = self.snapshots.get(&file).filter(|s| !s.is_stale()) {
                    self.stale.insert(file.clone(), (s.clone(), vec![edit]));
                } else if let Some((_, edits)) = self.stale.get_mut(&file) {
                    edits.push(edit);
                    if edits.len() > MAX_STALE_EDITS {
                        self.stale.remove(&file);
                    }
                }
            }
        }
        self.docs.insert(file, text);
        // (files include each other: every analysis is redone on demand)
        self.snapshots.clear();
        self.diagnostics.clear();
    }

    /// The analysis typing's questions are answered from: the current one
    /// when there is one, else the file's last, seen across the edits
    /// since (never older than its text; made afresh when there's none).
    fn quick_snapshot(&mut self, file: &Path) -> Option<Arc<Snapshot>> {
        if let Some(s) = self.snapshots.get(file) {
            return Some(s.clone());
        }
        if let Some((s, edits)) = self.stale.get(file) {
            return Some(Arc::new(Snapshot { parsed: s.parsed.clone(), model: s.model.clone(), edits: edits.clone() }));
        }
        self.snapshot(file)
    }

    /// The analysis made again now if the text changed since (an editor
    /// calls it when typing pauses, before the diagnostics).
    pub fn refresh(&mut self, file: &Path) {
        let _ = self.snapshot(file);
    }

    /// The editor closed a file: it's read from the disk again.
    pub fn close(&mut self, file: &Path) {
        self.docs.remove(file);
        self.snapshots.clear();
        self.diagnostics.clear();
        self.stale.remove(file);
    }

    /// The text of a file: the editor's, else the disk's (inside the
    /// roots, when confined).
    pub fn text(&self, file: &Path) -> Option<String> {
        self.docs.get(file).cloned().or_else(|| self.readable(file).then(|| rapidr_preprocessor::read_source(file).ok()).flatten())
    }

    /// Whether `file` may be read from the disk ([`Options::roots`]).
    pub fn readable(&self, file: &Path) -> bool {
        self.options.roots.as_ref().is_none_or(|roots| rapidr_preprocessor::is_within(file, roots))
    }

    /// The folders the analysis may read from now on (confined: see
    /// [`Options::roots`]).
    pub fn set_roots(&mut self, roots: Vec<PathBuf>) {
        if self.options.roots.as_ref() != Some(&roots) {
            self.options.roots = Some(roots);
            self.snapshots.clear();
            self.diagnostics.clear();
        }
    }

    fn preprocess_options(&self) -> rapidr_preprocessor::PreprocessOptions {
        // (confined: the roots, and the include folders the editor and the
        // environment name — RapidQ's include\ — are readable)
        let confine_to = self.options.roots.as_ref().map(|roots| {
            let mut all = roots.clone();
            all.extend(self.options.include_dirs.iter().cloned());
            if let Some(paths) = std::env::var_os("RAPIDR_INCLUDE_PATH") {
                all.extend(std::env::split_paths(&paths));
            }
            all
        });
        rapidr_preprocessor::PreprocessOptions { include_dirs: self.options.include_dirs.clone(), confine_to, ..Default::default() }
    }

    /// The analysis `file` is answered from: the program it is the main
    /// file of, or — for an `$INCLUDE`d file — an open program that
    /// includes it (with the editor's text of every open file).
    pub fn snapshot(&mut self, file: &Path) -> Option<Arc<Snapshot>> {
        if let Some(s) = self.snapshots.get(file) {
            return Some(s.clone());
        }
        if is_inc(file) {
            let mut roots: Vec<PathBuf> = self.docs.keys().filter(|p| p.as_path() != file && !is_inc(p)).cloned().collect();
            roots.sort();
            for root in roots {
                if let Some(s) = self.root_snapshot(&root) {
                    if s.parsed.file_key(file).is_some() {
                        self.snapshots.insert(file.to_path_buf(), s.clone());
                        return Some(s);
                    }
                }
            }
        }
        self.root_snapshot(file)
    }

    fn root_snapshot(&mut self, root: &Path) -> Option<Arc<Snapshot>> {
        if let Some(s) = self.snapshots.get(root) {
            return Some(s.clone());
        }
        let text = self.text(root)?;
        let open: Vec<(PathBuf, String)> = self.docs.iter().map(|(p, t)| (p.clone(), t.clone())).collect();
        let parsed = front::parse(root, &text, &self.preprocess_options(), &open);
        let model = model::analyze(&parsed.program, Some(&parsed.source));
        let s = Arc::new(Snapshot::new(parsed, model));
        self.stale.remove(root);
        self.snapshots.insert(root.to_path_buf(), s.clone());
        Some(s)
    }

    /// The compiler's diagnostics for the program `file` is the main file
    /// of (some may be in its `$INCLUDE` files), plus the registry's word
    /// on what the file uses ([`compat`]): what RapidR doesn't have yet,
    /// what one runtime only answers, and — in a RapidQ-compatible project
    /// — RapidR's extensions, which RapidQ's compiler refuses.
    pub fn diagnostics(&mut self, file: &Path) -> Vec<FileDiagnostic> {
        if let Some(d) = self.diagnostics.get(file) {
            return d.clone();
        }
        let Some(s) = self.root_snapshot(file) else { return Vec::new() };
        let mut out = diagnostics::compile(&s.parsed);
        let compiler = out.clone();
        // (where the compiler speaks, it's said)
        out.extend(compat::check(&s, file, self.options.rapidq_compatible).into_iter().filter(|c| {
            !compiler.iter().any(|d| d.file == c.file && d.start < c.end.max(c.start + 1) && c.start < d.end.max(d.start + 1))
        }));
        self.diagnostics.insert(file.to_path_buf(), out.clone());
        out
    }

    pub fn completions(&mut self, file: &Path, offset: usize) -> Completions {
        let Some(text) = self.text(file) else { return Completions::default() };
        let Some(s) = self.quick_snapshot(file) else { return Completions::default() };
        complete::completions(&s, file, &text, offset)
    }

    pub fn hover(&mut self, file: &Path, offset: usize) -> Option<Hover> {
        let text = self.text(file)?;
        let s = self.quick_snapshot(file)?;
        hover::hover(&s, file, &text, offset)
    }

    pub fn signature(&mut self, file: &Path, offset: usize) -> Option<SignatureHelp> {
        let text = self.text(file)?;
        let s = self.quick_snapshot(file)?;
        signature::signature(&s, file, &text, offset)
    }

    pub fn definition(&mut self, file: &Path, offset: usize) -> Vec<Location> {
        let Some(text) = self.text(file) else { return Vec::new() };
        let Some(s) = self.snapshot(file) else { return Vec::new() };
        navigate::definition(&s, file, &text, offset, &self.options)
    }

    pub fn references(&mut self, file: &Path, offset: usize, include_declaration: bool) -> Vec<Location> {
        let Some(s) = self.snapshot(file) else { return Vec::new() };
        navigate::references(&s, file, offset, include_declaration)
    }

    /// The name a rename at `offset` would change, or why it can't.
    pub fn prepare_rename(&mut self, file: &Path, offset: usize) -> Result<Location, String> {
        let s = self.snapshot(file).ok_or("no analysis for this file")?;
        navigate::prepare_rename(&s, file, offset)
    }

    /// Renames the symbol at `offset` everywhere it is used; refuses a
    /// name that isn't valid or that would change what another name means.
    pub fn rename(&mut self, file: &Path, offset: usize, new_name: &str) -> Result<WorkspaceEdit, String> {
        let s = self.snapshot(file).ok_or("no analysis for this file")?;
        navigate::rename(&s, file, offset, new_name)
    }

    pub fn outline(&mut self, file: &Path) -> Vec<OutlineItem> {
        let Some(s) = self.snapshot(file) else { return Vec::new() };
        navigate::outline(&s, file)
    }

    pub fn semantic_tokens(&mut self, file: &Path) -> Vec<SemanticToken> {
        let Some(s) = self.snapshot(file) else { return Vec::new() };
        navigate::semantic_tokens(&s, file)
    }

    /// The file formatted (`indent`: one level, e.g. four spaces):
    /// re-indented, and its words in the case the options ask
    /// ([`Analysis::case_edits`]).
    pub fn format(&mut self, file: &Path, indent: &str) -> Vec<TextEdit> {
        let Some(text) = self.text(file) else { return Vec::new() };
        self.format_range(file, 0, text.len(), indent)
    }

    /// The lines of bytes `start..end` of the file formatted (the whole
    /// file is read for its blocks; only edits on those lines are kept).
    pub fn format_range(&mut self, file: &Path, start: usize, end: usize, indent: &str) -> Vec<TextEdit> {
        let Some(text) = self.text(file) else { return Vec::new() };
        let index = LineIndex::new(&text);
        let first = index.line_start(index.line_col(start.min(text.len())).0).unwrap_or(0);
        // (a selection of whole lines ends at the next line's start)
        let end = end.min(text.len());
        let end = if end > start && index.line_col(end).1 == 0 { end - 1 } else { end };
        let last = index.line_end(&text, index.line_col(end).0);
        let mut edits: Vec<TextEdit> = format::format(&text, indent).into_iter().filter(|e| e.start >= first && e.end <= last).collect();
        edits.extend(self.case_edits(file, CaseScope::Range { start: first, end: last }));
        edits.sort_by_key(|e| e.start);
        edits
    }

    /// The edits that put `scope` of a file in the options' case: the
    /// language's words (keywords, statements, types, directives, builtins:
    /// the registry's) and, when asked, the program's names as declared.
    /// Strings, comments, directives' arguments and the program's own names
    /// are never the language's words. Editors call it as the user types
    /// ([`CaseScope::Typed`], after one of [`case::TRIGGERS`]) and the
    /// formatter on a range.
    pub fn case_edits(&mut self, file: &Path, scope: CaseScope) -> Vec<TextEdit> {
        let Some(text) = self.text(file) else { return Vec::new() };
        let Some(s) = self.quick_snapshot(file) else { return Vec::new() };
        case::case_edits(&s, file, &text, scope, self.options.case)
    }

    /// Fixes for the diagnostics in a range of a file.
    pub fn code_actions(&mut self, file: &Path, start: usize, end: usize) -> Vec<CodeAction> {
        // (the diagnostics are a main file's: an included one's are asked
        // through the program that includes it)
        let root = self.snapshot(file).map_or_else(|| file.to_path_buf(), |s| s.parsed.root.clone());
        let diags: Vec<FileDiagnostic> = self.diagnostics(&root).into_iter().filter(|d| d.file == file && d.start <= end && start <= d.end).collect();
        compat::actions(&diags, |f| self.text(f))
    }
}

/// Edits answered from an older analysis before it is made again anyway.
const MAX_STALE_EDITS: usize = 4096;

fn is_inc(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("inc"))
}
