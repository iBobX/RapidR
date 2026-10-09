//! What a code editor asks a language service (docs/ide-plan.md I2 / I3):
//! completion, hover, signature help, diagnostics, navigation, rename,
//! outline, semantic tokens, formatting and automatic keyword case — as
//! plain data, so the editor's view (rapidr-ui-kernel) needs no compiler.
//!
//! RapidR's own service (`rapidr-langsvc`, the one `rapidr lsp` serves VS
//! Code from) implements [`LanguageService`] for RapidQ / RapidR BASIC; a
//! runtime that carries it ([`install`]s it at start: `rapidr run`, RapidR
//! Studio on the desktop and in the browser) gives every RCODEEDITOR its
//! IntelliSense. Offsets are bytes of the file's text as the editor holds
//! it; files are named by the editor (a path, or a name of its own).

use std::cell::RefCell;

/// What a completion is (its icon in the list).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    Snippet,
    Text,
    /// A quick fix (Ctrl+.: the service's code actions).
    Fix,
}

impl CompletionKind {
    /// Its name as a program reads and writes it (`ShowCompletion`'s
    /// items, the registry's docs): `keyword`, `function` …
    pub fn name(self) -> &'static str {
        match self {
            CompletionKind::Keyword => "keyword",
            CompletionKind::Builtin => "builtin",
            CompletionKind::Sub => "sub",
            CompletionKind::Function => "function",
            CompletionKind::Method => "method",
            CompletionKind::Property => "property",
            CompletionKind::Event => "event",
            CompletionKind::Variable => "variable",
            CompletionKind::Parameter => "parameter",
            CompletionKind::Field => "field",
            CompletionKind::Constant => "constant",
            CompletionKind::Component => "component",
            CompletionKind::Type => "type",
            CompletionKind::Directive => "directive",
            CompletionKind::Label => "label",
            CompletionKind::Snippet => "snippet",
            CompletionKind::Text => "text",
            CompletionKind::Fix => "fix",
        }
    }

    /// The kind called `name` (any case); anything else is text.
    pub fn named(name: &str) -> CompletionKind {
        const ALL: [CompletionKind; 18] = [
            CompletionKind::Keyword,
            CompletionKind::Builtin,
            CompletionKind::Sub,
            CompletionKind::Function,
            CompletionKind::Method,
            CompletionKind::Property,
            CompletionKind::Event,
            CompletionKind::Variable,
            CompletionKind::Parameter,
            CompletionKind::Field,
            CompletionKind::Constant,
            CompletionKind::Component,
            CompletionKind::Type,
            CompletionKind::Directive,
            CompletionKind::Label,
            CompletionKind::Snippet,
            CompletionKind::Text,
            CompletionKind::Fix,
        ];
        ALL.into_iter().find(|k| k.name().eq_ignore_ascii_case(name.trim())).unwrap_or(CompletionKind::Text)
    }
}

/// One completion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Completion {
    pub label: String,
    pub kind: CompletionKind,
    /// One line beside the label (a type, a signature).
    pub detail: String,
    /// Its documentation (plain text, or light Markdown: `code`, **bold**).
    pub doc: String,
    /// What to insert, when not the label (snippet syntax when `snippet`).
    pub insert: Option<String>,
    pub snippet: bool,
    /// The order: smaller first (the label when empty).
    pub sort: String,
    /// Other edits of the file made with it, as one step (an import the
    /// name needs: `$INCLUDE "RAPIDQ.INC"`); bytes of the text before.
    pub edits: Vec<Edit>,
}

impl Completion {
    pub fn new(label: impl Into<String>, kind: CompletionKind) -> Completion {
        Completion { label: label.into(), kind, detail: String::new(), doc: String::new(), insert: None, snippet: false, sort: String::new(), edits: Vec::new() }
    }
}

/// The completions at a place, and the bytes they replace (the word typed
/// so far).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Completions {
    pub items: Vec<Completion>,
    pub start: usize,
    pub end: usize,
}

/// What a hover shows, over bytes `start..end`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hover {
    /// Light Markdown (a fenced code block first, then prose).
    pub text: String,
    pub start: usize,
    pub end: usize,
}

/// A signature, each parameter's byte range in `label`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signature {
    pub label: String,
    pub params: Vec<(usize, usize)>,
    pub doc: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SignatureHelp {
    pub signatures: Vec<Signature>,
    pub active_signature: usize,
    pub active_param: usize,
}

/// How bad a diagnostic is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    Error = 1,
    Warning = 2,
    Info = 3,
    Hint = 4,
}

impl Severity {
    pub fn name(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
            Severity::Hint => "hint",
        }
    }

    /// From a name (`error`, `warning`, `info` / `information`, `hint`) or a
    /// number (LSP's 1–4); errors otherwise.
    pub fn named(s: &str) -> Severity {
        match s.trim().to_ascii_lowercase().as_str() {
            "warning" | "warn" | "2" => Severity::Warning,
            "info" | "information" | "3" => Severity::Info,
            "hint" | "4" => Severity::Hint,
            _ => Severity::Error,
        }
    }
}

/// A problem in bytes `start..end` of a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub file: String,
    pub start: usize,
    pub end: usize,
    pub severity: Severity,
    pub message: String,
    /// The service's code for it (`rapidq-compat` …).
    pub code: String,
}

/// A place in a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Location {
    pub file: String,
    pub start: usize,
    pub end: usize,
}

/// Bytes `start..end` replaced with `text`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// A fix for a problem (Ctrl+., VS Code's quick fix): its title and its
/// edits of the file asked about (bytes of its text).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeAction {
    pub title: String,
    pub edits: Vec<Edit>,
    /// The one to pick first (offered first, selected).
    pub preferred: bool,
}

/// What an outline entry is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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

impl OutlineKind {
    pub fn name(self) -> &'static str {
        match self {
            OutlineKind::Sub => "sub",
            OutlineKind::Function => "function",
            OutlineKind::Type => "type",
            OutlineKind::Field => "field",
            OutlineKind::Method => "method",
            OutlineKind::Event => "event",
            OutlineKind::Component => "component",
            OutlineKind::Constant => "constant",
            OutlineKind::Variable => "variable",
            OutlineKind::Label => "label",
        }
    }
}

/// One entry of a file's outline: its whole statement (`start..end`) and
/// its name's bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineItem {
    pub name: String,
    pub detail: String,
    pub kind: OutlineKind,
    pub start: usize,
    pub end: usize,
    pub name_start: usize,
    pub name_end: usize,
    pub children: Vec<OutlineItem>,
}

/// A name the service knows what it is, for its colour: bytes
/// `start..end`, a token kind of the editor's vocabulary
/// (`function`, `variable.parameter`, `type.component` …).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticToken {
    pub start: usize,
    pub end: usize,
    pub kind: &'static str,
}

/// A language service, as an editor uses it. Every method answers for the
/// text last given with [`LanguageService::update`].
pub trait LanguageService {
    /// Whether it serves files of the language `language` (an id of
    /// `rapidr_editor::Languages`: `rapidr-basic` …).
    fn serves(&self, language: &str) -> bool;
    /// The editor's whole text of `file`, now.
    fn update(&mut self, file: &str, text: &str);
    /// The editor closed `file`.
    fn close(&mut self, file: &str);
    fn completions(&mut self, file: &str, offset: usize) -> Completions;
    fn hover(&mut self, file: &str, offset: usize) -> Option<Hover>;
    fn signature(&mut self, file: &str, offset: usize) -> Option<SignatureHelp>;
    /// The problems of the program `file` belongs to (some may be in its
    /// other files).
    fn diagnostics(&mut self, file: &str) -> Vec<Diagnostic>;
    fn definition(&mut self, file: &str, offset: usize) -> Vec<Location>;
    fn references(&mut self, file: &str, offset: usize) -> Vec<Location>;
    /// Renames what is at `offset` everywhere: each file's edits, or why it
    /// can't.
    fn rename(&mut self, file: &str, offset: usize, new_name: &str) -> Result<Vec<(String, Vec<Edit>)>, String>;
    fn outline(&mut self, file: &str) -> Vec<OutlineItem>;
    fn semantic_tokens(&mut self, file: &str) -> Vec<SemanticToken>;
    /// The file re-indented with `indent` per level (and its words cased).
    fn format(&mut self, file: &str, indent: &str) -> Vec<Edit>;
    /// After the user typed `ch` ending at `offset`: the edits that put the
    /// word (or, for a line break, the line) just finished in the keyword
    /// case `case` (`upper`, `lower`, `proper`; `preserve`: none), and with
    /// `+declaration` after it (`upper+declaration`) the program's own
    /// names as their declarations spell them.
    fn case_edits(&mut self, file: &str, offset: usize, ch: char, case: &str) -> Vec<Edit>;
    /// The characters after which [`LanguageService::case_edits`] is asked.
    fn case_triggers(&self) -> &'static [char];
    /// Byte `offset` of `file` (the editor's text, else the disk's) as a
    /// 1-based line and 1-based character column: where a definition or a
    /// reference in another file is, for the program to open it there.
    fn position(&mut self, file: &str, offset: usize) -> Option<(usize, usize)>;
    /// Fixes for the problems over bytes `start..end` of `file`
    /// (none: no quick fix there).
    fn code_actions(&mut self, file: &str, start: usize, end: usize) -> Vec<CodeAction> {
        let _ = (file, start, end);
        Vec::new()
    }
    /// The characters that open completion by themselves (`.`).
    fn completion_triggers(&self) -> &'static [char] {
        &['.']
    }
    /// The characters that open or move signature help (`(`, `,`).
    fn signature_triggers(&self) -> &'static [char] {
        &['(', ',']
    }
}

thread_local! {
    static SERVICE: RefCell<Option<Box<dyn LanguageService>>> = const { RefCell::new(None) };
}

/// Makes `service` the one editors on this thread ask (a runtime that
/// carries one, at start).
pub fn install(service: Box<dyn LanguageService>) {
    SERVICE.with(|s| *s.borrow_mut() = Some(service));
}

/// Whether a service is installed that serves `language`.
pub fn available(language: &str) -> bool {
    SERVICE.with(|s| s.borrow().as_ref().is_some_and(|s| s.serves(language)))
}

/// Runs `f` with the installed service (`None`: there is none, or it is
/// busy — a request made while another runs).
pub fn with<R>(f: impl FnOnce(&mut dyn LanguageService) -> R) -> Option<R> {
    SERVICE.with(|s| {
        let mut s = s.try_borrow_mut().ok()?;
        let svc = s.as_mut()?;
        Some(f(svc.as_mut()))
    })
}
