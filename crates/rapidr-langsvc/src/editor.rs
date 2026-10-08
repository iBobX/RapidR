//! RapidR's language service as RapidR Studio's code editor asks it
//! (`rapidr_editor::service::LanguageService`): [`Analysis`]'s answers in
//! the editor's plain data, the way `rapidr lsp` gives them to VS Code
//! (Markdown hovers and docs, the compiler's diagnostics with its words,
//! every reference with its declaration, rename checked first).
//!
//! A runtime that carries the compiler [`install`]s it once at start, on
//! its UI thread (the editor's service is per thread): `rapidr` on the
//! desktop, the web runtime built with its `langsvc` feature (RapidR
//! Studio in the browser). Every RCODEEDITOR of a program there then has
//! its IntelliSense for RapidQ / RapidR BASIC.

use std::path::{Path, PathBuf};

use rapidr_editor::service as ed;

use crate::case::{self, CaseScope, KeywordCase};
use crate::{Analysis, CompletionKind, Options, OutlineKind, Severity, TokenKind};

/// The language id of RapidQ / RapidR BASIC in `rapidr_editor::Languages`.
pub const LANGUAGE: &str = "rapidq-basic";

/// [`Analysis`] behind the editor's contract. Files are the editor's names
/// for them, taken as paths (a real path, or a name of its own such as
/// `untitled-1.bas`: then only the editor's text is read).
#[derive(Debug, Default)]
pub struct EditorService {
    analysis: Analysis,
}

impl EditorService {
    pub fn new() -> Self {
        EditorService { analysis: Analysis::new(Options::default()) }
    }

    /// Over an analysis with these options (a project's: RapidQ-compatible,
    /// its include folders, its case).
    pub fn with_options(options: Options) -> Self {
        EditorService { analysis: Analysis::new(options) }
    }

    pub fn analysis(&mut self) -> &mut Analysis {
        &mut self.analysis
    }

    /// The keyword case asked for, in the analysis's options (only when it
    /// changes: new options drop every analysis).
    fn set_keyword_case(&mut self, keywords: KeywordCase) {
        if self.analysis.options().case.keywords != keywords {
            let mut options = self.analysis.options().clone();
            options.case.keywords = keywords;
            self.analysis.set_options(options);
        }
    }
}

/// Makes RapidR's language service the one this thread's editors ask.
pub fn install() {
    ed::install(Box::new(EditorService::new()));
}

fn path(file: &str) -> PathBuf {
    PathBuf::from(file)
}

fn name(file: &Path) -> String {
    file.to_string_lossy().into_owned()
}

fn completion_kind(k: CompletionKind) -> ed::CompletionKind {
    match k {
        CompletionKind::Keyword => ed::CompletionKind::Keyword,
        CompletionKind::Builtin => ed::CompletionKind::Builtin,
        CompletionKind::Sub => ed::CompletionKind::Sub,
        CompletionKind::Function => ed::CompletionKind::Function,
        CompletionKind::Method => ed::CompletionKind::Method,
        CompletionKind::Property => ed::CompletionKind::Property,
        CompletionKind::Event => ed::CompletionKind::Event,
        CompletionKind::Variable => ed::CompletionKind::Variable,
        CompletionKind::Parameter => ed::CompletionKind::Parameter,
        CompletionKind::Field => ed::CompletionKind::Field,
        CompletionKind::Constant => ed::CompletionKind::Constant,
        CompletionKind::Component => ed::CompletionKind::Component,
        CompletionKind::Type => ed::CompletionKind::Type,
        CompletionKind::Directive => ed::CompletionKind::Directive,
        CompletionKind::Label => ed::CompletionKind::Label,
    }
}

fn outline_kind(k: OutlineKind) -> ed::OutlineKind {
    match k {
        OutlineKind::Sub => ed::OutlineKind::Sub,
        OutlineKind::Function => ed::OutlineKind::Function,
        OutlineKind::Type => ed::OutlineKind::Type,
        OutlineKind::Field => ed::OutlineKind::Field,
        OutlineKind::Method => ed::OutlineKind::Method,
        OutlineKind::Event => ed::OutlineKind::Event,
        OutlineKind::Component => ed::OutlineKind::Component,
        OutlineKind::Constant => ed::OutlineKind::Constant,
        OutlineKind::Variable => ed::OutlineKind::Variable,
        OutlineKind::Label => ed::OutlineKind::Label,
    }
}

fn outline_item(i: crate::OutlineItem) -> ed::OutlineItem {
    ed::OutlineItem {
        name: i.name,
        detail: i.detail.unwrap_or_default(),
        kind: outline_kind(i.kind),
        start: i.start,
        end: i.end,
        name_start: i.name_start,
        name_end: i.name_end,
        children: i.children.into_iter().map(outline_item).collect(),
    }
}

/// The editor's token kind (its theme's scope) for a name the model knows.
fn token_kind(k: TokenKind) -> &'static str {
    match k {
        TokenKind::Function => "function",
        TokenKind::Variable => "variable",
        TokenKind::Parameter => "variable.parameter",
        TokenKind::Property => "variable.property",
        TokenKind::Type => "type",
        TokenKind::Constant => "constant",
        TokenKind::Component => "type.component",
        TokenKind::Label => "label",
    }
}

fn severity(s: Severity) -> ed::Severity {
    match s {
        Severity::Error => ed::Severity::Error,
        Severity::Warning => ed::Severity::Warning,
        Severity::Note => ed::Severity::Info,
    }
}

fn location(l: crate::Location) -> ed::Location {
    ed::Location { file: name(&l.file), start: l.start, end: l.end }
}

fn edit(e: crate::TextEdit) -> ed::Edit {
    ed::Edit { start: e.start, end: e.end, text: e.text }
}

impl ed::LanguageService for EditorService {
    fn serves(&self, language: &str) -> bool {
        language.eq_ignore_ascii_case(LANGUAGE)
    }

    fn update(&mut self, file: &str, text: &str) {
        self.analysis.update(path(file), text);
    }

    fn close(&mut self, file: &str) {
        self.analysis.close(&path(file));
    }

    fn completions(&mut self, file: &str, offset: usize) -> ed::Completions {
        let got = self.analysis.completions(&path(file), offset);
        ed::Completions {
            items: got
                .items
                .into_iter()
                .map(|c| ed::Completion {
                    label: c.label,
                    kind: completion_kind(c.kind),
                    detail: c.detail.unwrap_or_default(),
                    doc: c.doc.unwrap_or_default(),
                    insert: c.insert,
                    snippet: c.snippet,
                    sort: c.sort,
                })
                .collect(),
            start: got.start,
            end: got.end,
        }
    }

    fn hover(&mut self, file: &str, offset: usize) -> Option<ed::Hover> {
        self.analysis.hover(&path(file), offset).map(|h| ed::Hover { text: h.markdown, start: h.start, end: h.end })
    }

    fn signature(&mut self, file: &str, offset: usize) -> Option<ed::SignatureHelp> {
        self.analysis.signature(&path(file), offset).map(|h| ed::SignatureHelp {
            signatures: h.signatures.into_iter().map(|s| ed::Signature { label: s.label, params: s.params, doc: s.doc.unwrap_or_default() }).collect(),
            active_signature: h.active_signature,
            active_param: h.active_param,
        })
    }

    fn diagnostics(&mut self, file: &str) -> Vec<ed::Diagnostic> {
        let file = path(file);
        // (an `$INCLUDE`d file: the program of an open main file that
        // includes it, as `rapidr lsp` publishes them)
        let root = self.analysis.snapshot(&file).map_or_else(|| file.clone(), |s| s.parsed.root.clone());
        self.analysis
            .diagnostics(&root)
            .into_iter()
            .map(|d| ed::Diagnostic {
                file: name(&d.file),
                start: d.start,
                end: d.end,
                severity: severity(d.severity),
                message: d.message,
                code: d.code.unwrap_or_default(),
            })
            .collect()
    }

    fn definition(&mut self, file: &str, offset: usize) -> Vec<ed::Location> {
        self.analysis.definition(&path(file), offset).into_iter().map(location).collect()
    }

    fn references(&mut self, file: &str, offset: usize) -> Vec<ed::Location> {
        self.analysis.references(&path(file), offset, true).into_iter().map(location).collect()
    }

    fn rename(&mut self, file: &str, offset: usize, new_name: &str) -> Result<Vec<(String, Vec<ed::Edit>)>, String> {
        let file = path(file);
        // (what `rapidr lsp` asks first: whether there is a name to rename)
        self.analysis.prepare_rename(&file, offset)?;
        let edits = self.analysis.rename(&file, offset, new_name)?;
        Ok(edits.into_iter().map(|(f, e)| (name(&f), e.into_iter().map(edit).collect())).collect())
    }

    fn outline(&mut self, file: &str) -> Vec<ed::OutlineItem> {
        self.analysis.outline(&path(file)).into_iter().map(outline_item).collect()
    }

    fn semantic_tokens(&mut self, file: &str) -> Vec<ed::SemanticToken> {
        self.analysis.semantic_tokens(&path(file)).into_iter().map(|t| ed::SemanticToken { start: t.start, end: t.end, kind: token_kind(t.kind) }).collect()
    }

    fn format(&mut self, file: &str, indent: &str) -> Vec<ed::Edit> {
        self.analysis.format(&path(file), indent).into_iter().map(edit).collect()
    }

    fn case_edits(&mut self, file: &str, offset: usize, ch: char, case: &str) -> Vec<ed::Edit> {
        // (the case the editor asks is the one formatting uses from now on;
        // `preserve` too: off)
        let Some(keywords) = KeywordCase::parse(case) else { return Vec::new() };
        self.set_keyword_case(keywords);
        if keywords == KeywordCase::Preserve {
            return Vec::new();
        }
        self.analysis.case_edits(&path(file), CaseScope::Typed { offset, ch }).into_iter().map(edit).collect()
    }

    fn code_actions(&mut self, file: &str, start: usize, end: usize) -> Vec<ed::CodeAction> {
        let file = path(file);
        let mut out: Vec<ed::CodeAction> = self
            .analysis
            .code_actions(&file, start, end)
            .into_iter()
            .filter_map(|a| {
                // (this file's edits only: a fix never reaches into another)
                let edits: Vec<ed::Edit> = a.edit.iter().filter(|(f, _)| *f == file).flat_map(|(_, e)| e.iter().cloned().map(edit)).collect();
                (!edits.is_empty() && a.edit.iter().all(|(f, _)| *f == file)).then_some(ed::CodeAction { title: a.title, edits, preferred: a.preferred })
            })
            .collect();
        out.sort_by_key(|a| !a.preferred);
        out
    }

    fn case_triggers(&self) -> &'static [char] {
        case::TRIGGERS
    }

    fn position(&mut self, file: &str, offset: usize) -> Option<(usize, usize)> {
        let text = self.analysis.text(&path(file))?;
        let before = text.get(..offset)?;
        let line_start = before.rfind('\n').map_or(0, |i| i + 1);
        Some((before.matches('\n').count() + 1, before[line_start..].chars().count() + 1))
    }
}
