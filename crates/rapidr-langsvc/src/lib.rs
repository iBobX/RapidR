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
//! Three seams wait for the I0 foundations, each one module, so each
//! switch is local: [`front`] (the parser for tools, L-PARSE), [`model`]
//! (the semantic model exported from the compiler, L-PARSE) and
//! [`registry`] (the language registry, L-REG).

pub mod diagnostics;
pub mod front;
pub mod model;
pub mod registry;
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
    pub parsed: Parsed,
    pub model: SemanticModel,
}

impl Snapshot {
    /// The preprocessed offset of byte `offset` of `file`.
    pub fn pre_offset(&self, file: &Path, offset: usize) -> Option<usize> {
        self.parsed.to_preprocessed(file, offset)
    }

    /// Where the model's span is in the files.
    pub fn locate(&self, span: rapidr_diagnostics::TextSpan) -> Option<Location> {
        self.parsed.locate(span)
    }

    /// A multi-line span of the AST in the files (its start and end located
    /// separately).
    pub fn locate_range(&self, span: rapidr_diagnostics::TextSpan) -> Option<Location> {
        let start = self.parsed.locate(span)?;
        let last = rapidr_diagnostics::TextSpan::new(span.end.saturating_sub(1).max(span.start), span.end);
        let end = self.parsed.locate(last).filter(|l| l.file == start.file).map_or(start.end, |l| l.end);
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
    }

    /// A file's new text (the editor's, whole).
    pub fn update(&mut self, file: impl Into<PathBuf>, text: impl Into<String>) {
        self.docs.insert(file.into(), text.into());
        // (files include each other: every analysis is redone on demand)
        self.snapshots.clear();
        self.diagnostics.clear();
    }

    /// The editor closed a file: it's read from the disk again.
    pub fn close(&mut self, file: &Path) {
        self.docs.remove(file);
        self.snapshots.clear();
        self.diagnostics.clear();
    }

    /// The text of a file: the editor's, else the disk's.
    pub fn text(&self, file: &Path) -> Option<String> {
        self.docs.get(file).cloned().or_else(|| rapidr_preprocessor::read_source(file).ok())
    }

    fn preprocess_options(&self) -> rapidr_preprocessor::PreprocessOptions {
        rapidr_preprocessor::PreprocessOptions { include_dirs: self.options.include_dirs.clone(), ..Default::default() }
    }

    /// The analysis `file` is answered from: the program it is the main
    /// file of, or — for an `$INCLUDE`d file not changed in the editor —
    /// the open program that includes it.
    pub fn snapshot(&mut self, file: &Path) -> Option<Arc<Snapshot>> {
        if let Some(s) = self.snapshots.get(file) {
            return Some(s.clone());
        }
        let text = self.text(file)?;
        let is_include = file.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("inc"));
        if is_include {
            let saved = rapidr_preprocessor::read_source(file).ok();
            if saved.as_deref() == Some(text.as_str()) {
                let roots: Vec<PathBuf> = self.docs.keys().filter(|p| p.as_path() != file && !is_inc(p)).cloned().collect();
                for root in roots {
                    if let Some(s) = self.root_snapshot(&root) {
                        if s.parsed.file_key(file).is_some() {
                            self.snapshots.insert(file.to_path_buf(), s.clone());
                            return Some(s);
                        }
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
        let docs = &self.docs;
        let overlay = |p: &Path| docs.get(p).cloned();
        let parsed = front::parse(root, &text, &self.preprocess_options(), &overlay);
        let model = model::analyze(&parsed.program, Some(&parsed.source));
        let s = Arc::new(Snapshot { parsed, model });
        self.snapshots.insert(root.to_path_buf(), s.clone());
        Some(s)
    }

    /// The compiler's diagnostics for the program `file` is the main file
    /// of (some may be in its `$INCLUDE` files), plus the RapidQ
    /// compatibility checks of a RapidQ-compatible project.
    pub fn diagnostics(&mut self, file: &Path) -> Vec<FileDiagnostic> {
        if let Some(d) = self.diagnostics.get(file) {
            return d.clone();
        }
        let Some(text) = self.text(file) else { return Vec::new() };
        let docs = &self.docs;
        let file_text = |p: &Path| docs.get(p).cloned();
        let mut out = diagnostics::compile(file, &text, &self.preprocess_options(), &file_text);
        if self.options.rapidq_compatible {
            if let Some(s) = self.snapshot(file) {
                out.extend(compat::check(&s, file));
            }
        }
        self.diagnostics.insert(file.to_path_buf(), out.clone());
        out
    }

    pub fn completions(&mut self, file: &Path, offset: usize) -> Completions {
        let Some(text) = self.text(file) else { return Completions::default() };
        let Some(s) = self.snapshot(file) else { return Completions::default() };
        complete::completions(&s, file, &text, offset)
    }

    pub fn hover(&mut self, file: &Path, offset: usize) -> Option<Hover> {
        let text = self.text(file)?;
        let s = self.snapshot(file)?;
        hover::hover(&s, file, &text, offset)
    }

    pub fn signature(&mut self, file: &Path, offset: usize) -> Option<SignatureHelp> {
        let text = self.text(file)?;
        let s = self.snapshot(file)?;
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

    /// The file re-indented (`indent`: one level, e.g. four spaces).
    pub fn format(&mut self, file: &Path, indent: &str) -> Vec<TextEdit> {
        let Some(text) = self.text(file) else { return Vec::new() };
        format::format(&text, indent)
    }

    /// Fixes for the diagnostics in a range of a file.
    pub fn code_actions(&mut self, file: &Path, start: usize, end: usize) -> Vec<CodeAction> {
        let diags: Vec<FileDiagnostic> = self.diagnostics(file).into_iter().filter(|d| d.file == file && d.start <= end && start <= d.end).collect();
        compat::actions(&diags)
    }
}

fn is_inc(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("inc"))
}
