//! `rapidr lsp`: the Language Server Protocol over stdio, on
//! [`rapidr_langsvc`] (docs/ide-plan.md, I3). VS Code (the RapidR
//! extension), and any editor that speaks LSP, get the same IntelliSense as
//! RapidR Studio: completion, hover, signature help, go to definition,
//! references, rename, diagnostics in the compiler's own words, the
//! outline, semantic highlighting, formatting and quick fixes.
//!
//! Synchronous: one thread answers requests in order; diagnostics are
//! published once the messages waiting are handled (typing doesn't compile
//! the program at every key).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::str::FromStr;

use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::Notification as _;
use lsp_types::request::Request as _;
use lsp_types::*;
use rapidr_langsvc::{Analysis, CompletionKind, LineIndex, OutlineKind, Severity, TokenKind};

/// `rapidr lsp`: serves one client on stdin / stdout until it exits.
pub fn run_stdio() -> ExitCode {
    let (connection, io_threads) = Connection::stdio();
    let result = serve(&connection);
    drop(connection);
    let joined = io_threads.join();
    match (result, joined) {
        (Ok(()), Ok(())) => ExitCode::SUCCESS,
        (Err(e), _) => {
            eprintln!("rapidr lsp: {e}");
            ExitCode::from(1)
        }
        (_, Err(e)) => {
            eprintln!("rapidr lsp: {e}");
            ExitCode::from(1)
        }
    }
}

/// The semantic token types and modifiers, by index (the legend).
const TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::FUNCTION,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::PARAMETER,
    SemanticTokenType::PROPERTY,
    SemanticTokenType::TYPE,
    SemanticTokenType::ENUM_MEMBER,
];
const TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[SemanticTokenModifier::DECLARATION, SemanticTokenModifier::READONLY, SemanticTokenModifier::STATIC];

/// Serves a client on a connection (stdio, or memory in tests).
pub fn serve(connection: &Connection) -> Result<(), String> {
    let (id, params) = connection.initialize_start().map_err(|e| e.to_string())?;
    let params: InitializeParams = serde_json::from_value(params).map_err(|e| e.to_string())?;
    let utf8 = params
        .capabilities
        .general
        .as_ref()
        .and_then(|g| g.position_encodings.as_ref())
        .is_some_and(|encs| encs.contains(&PositionEncodingKind::UTF8));
    let options = params.initialization_options.as_ref();
    let rapidq_compatible = options.and_then(|o| o.get("rapidqCompatible")).and_then(|v| v.as_bool()).unwrap_or(false);
    let include_dirs = options
        .and_then(|o| o.get("includeDirs"))
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|d| d.as_str()).map(PathBuf::from).collect())
        .unwrap_or_default();
    // Automatic case: `keywordCase` (upper, lower, proper, preserve) and
    // `identifierCase` (declaration, preserve).
    let setting = |name: &str| options.and_then(|o| o.get(name)).and_then(|v| v.as_str());
    let case = rapidr_langsvc::CaseOptions {
        keywords: setting("keywordCase").and_then(rapidr_langsvc::KeywordCase::parse).unwrap_or_default(),
        identifiers: setting("identifierCase").and_then(rapidr_langsvc::IdentifierCase::parse).unwrap_or_default(),
    };
    let mut triggers = rapidr_langsvc::case::TRIGGERS.iter().map(|c| c.to_string());
    let capabilities = ServerCapabilities {
        position_encoding: Some(if utf8 { PositionEncodingKind::UTF8 } else { PositionEncodingKind::UTF16 }),
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        completion_provider: Some(CompletionOptions { trigger_characters: Some(vec![".".into(), "$".into()]), ..Default::default() }),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        signature_help_provider: Some(SignatureHelpOptions {
            trigger_characters: Some(vec!["(".into(), ",".into()]),
            retrigger_characters: Some(vec![",".into()]),
            ..Default::default()
        }),
        definition_provider: Some(OneOf::Left(true)),
        references_provider: Some(OneOf::Left(true)),
        rename_provider: Some(OneOf::Right(RenameOptions { prepare_provider: Some(true), work_done_progress_options: Default::default() })),
        document_symbol_provider: Some(OneOf::Left(true)),
        document_formatting_provider: Some(OneOf::Left(true)),
        document_range_formatting_provider: Some(OneOf::Left(true)),
        document_on_type_formatting_provider: Some(DocumentOnTypeFormattingOptions {
            first_trigger_character: triggers.next().unwrap_or_default(),
            more_trigger_character: Some(triggers.collect()),
        }),
        code_action_provider: Some(CodeActionProviderCapability::Options(CodeActionOptions {
            code_action_kinds: Some(vec![CodeActionKind::QUICKFIX]),
            ..Default::default()
        })),
        semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
            legend: SemanticTokensLegend { token_types: TOKEN_TYPES.to_vec(), token_modifiers: TOKEN_MODIFIERS.to_vec() },
            full: Some(SemanticTokensFullOptions::Bool(true)),
            range: None,
            ..Default::default()
        })),
        ..Default::default()
    };
    let result = InitializeResult {
        capabilities,
        server_info: Some(ServerInfo { name: "rapidr".into(), version: Some(env!("CARGO_PKG_VERSION").into()) }),
    };
    connection.initialize_finish(id, serde_json::to_value(result).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;

    let mut server = Server {
        analysis: Analysis::new(rapidr_langsvc::Options { rapidq_compatible, include_dirs, case }),
        open: HashSet::new(),
        dirty: false,
        published: HashMap::new(),
        utf8,
    };
    loop {
        let Ok(msg) = connection.receiver.recv() else { return Ok(()) };
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req).map_err(|e| e.to_string())? {
                    return Ok(());
                }
                let resp = server.request(req);
                connection.sender.send(Message::Response(resp)).map_err(|e| e.to_string())?;
            }
            Message::Notification(n) => {
                if n.method == notification::Exit::METHOD {
                    return Ok(());
                }
                server.notification(n);
            }
            Message::Response(_) => {}
        }
        // Diagnostics once nothing else is waiting.
        if server.dirty && connection.receiver.is_empty() {
            server.dirty = false;
            for n in server.publish_diagnostics() {
                connection.sender.send(Message::Notification(n)).map_err(|e| e.to_string())?;
            }
        }
    }
}

struct Server {
    analysis: Analysis,
    /// The files open in the editor.
    open: HashSet<PathBuf>,
    /// A file changed since diagnostics were published.
    dirty: bool,
    /// The files each main file's diagnostics were published for.
    published: HashMap<PathBuf, HashSet<PathBuf>>,
    utf8: bool,
}

impl Server {
    fn notification(&mut self, n: Notification) {
        match n.method.as_str() {
            notification::DidOpenTextDocument::METHOD => {
                if let Ok(p) = serde_json::from_value::<DidOpenTextDocumentParams>(n.params) {
                    let path = uri_to_path(&p.text_document.uri);
                    self.analysis.update(path.clone(), p.text_document.text);
                    self.open.insert(path);
                    self.dirty = true;
                }
            }
            notification::DidChangeTextDocument::METHOD => {
                if let Ok(p) = serde_json::from_value::<DidChangeTextDocumentParams>(n.params) {
                    let path = uri_to_path(&p.text_document.uri);
                    // (full sync: the last change is the whole text)
                    if let Some(change) = p.content_changes.into_iter().last() {
                        if change.range.is_none() {
                            self.analysis.update(path, change.text);
                            self.dirty = true;
                        }
                    }
                }
            }
            notification::DidCloseTextDocument::METHOD => {
                if let Ok(p) = serde_json::from_value::<DidCloseTextDocumentParams>(n.params) {
                    let path = uri_to_path(&p.text_document.uri);
                    self.analysis.close(&path);
                    self.open.remove(&path);
                    self.dirty = true;
                }
            }
            notification::DidSaveTextDocument::METHOD => self.dirty = true,
            _ => {}
        }
    }

    fn request(&mut self, req: Request) -> Response {
        let id = req.id.clone();
        match self.answer(req) {
            Ok(value) => Response::new_ok(id, value),
            Err((code, message)) => Response::new_err(id, code as i32, message),
        }
    }

    fn answer(&mut self, req: Request) -> Result<serde_json::Value, (ErrorCode, String)> {
        fn params<P: serde::de::DeserializeOwned>(req: &Request) -> Result<P, (ErrorCode, String)> {
            serde_json::from_value(req.params.clone()).map_err(|e| (ErrorCode::InvalidParams, e.to_string()))
        }
        let json = |v: serde_json::Value| Ok(v);
        match req.method.as_str() {
            request::Completion::METHOD => {
                let p: CompletionParams = params(&req)?;
                let (path, text, offset) = self.place(&p.text_document_position)?;
                let got = self.analysis.completions(&path, offset);
                let index = LineIndex::new(&text);
                let range = self.range(&text, &index, got.start, got.end);
                let items: Vec<CompletionItem> = got
                    .items
                    .into_iter()
                    .map(|c| CompletionItem {
                        kind: Some(completion_kind(c.kind)),
                        detail: c.detail,
                        documentation: c.doc.map(|d| Documentation::MarkupContent(MarkupContent { kind: MarkupKind::Markdown, value: d })),
                        sort_text: Some(c.sort),
                        filter_text: Some(c.label.clone()),
                        insert_text_format: Some(if c.snippet { InsertTextFormat::SNIPPET } else { InsertTextFormat::PLAIN_TEXT }),
                        text_edit: Some(CompletionTextEdit::Edit(TextEdit { range, new_text: c.insert.unwrap_or_else(|| c.label.clone()) })),
                        label: c.label,
                        ..Default::default()
                    })
                    .collect();
                json(serde_json::to_value(CompletionResponse::List(CompletionList { is_incomplete: false, items })).unwrap())
            }
            request::HoverRequest::METHOD => {
                let p: HoverParams = params(&req)?;
                let (path, text, offset) = self.place(&p.text_document_position_params)?;
                let index = LineIndex::new(&text);
                let hover = self.analysis.hover(&path, offset).map(|h| Hover {
                    contents: HoverContents::Markup(MarkupContent { kind: MarkupKind::Markdown, value: h.markdown }),
                    range: Some(self.range(&text, &index, h.start, h.end)),
                });
                json(serde_json::to_value(hover).unwrap())
            }
            request::SignatureHelpRequest::METHOD => {
                let p: SignatureHelpParams = params(&req)?;
                let (path, _, offset) = self.place(&p.text_document_position_params)?;
                let help = self.analysis.signature(&path, offset).map(|h| SignatureHelp {
                    signatures: h
                        .signatures
                        .into_iter()
                        .map(|s| SignatureInformation {
                            parameters: Some(
                                s.params
                                    .iter()
                                    .map(|&(a, b)| ParameterInformation {
                                        label: ParameterLabel::LabelOffsets([self.units(&s.label[..a]), self.units(&s.label[..b])]),
                                        documentation: None,
                                    })
                                    .collect(),
                            ),
                            documentation: s.doc.map(|d| Documentation::MarkupContent(MarkupContent { kind: MarkupKind::Markdown, value: d })),
                            label: s.label,
                            active_parameter: None,
                        })
                        .collect(),
                    active_signature: Some(h.active_signature as u32),
                    active_parameter: Some(h.active_param as u32),
                });
                json(serde_json::to_value(help).unwrap())
            }
            request::GotoDefinition::METHOD => {
                let p: GotoDefinitionParams = params(&req)?;
                let (path, text, offset) = self.place(&p.text_document_position_params)?;
                let locs = self.analysis.definition(&path, offset);
                let _ = text;
                let out: Vec<Location> = locs.iter().filter_map(|l| self.location(l)).collect();
                json(serde_json::to_value(GotoDefinitionResponse::Array(out)).unwrap())
            }
            request::References::METHOD => {
                let p: ReferenceParams = params(&req)?;
                let (path, _, offset) = self.place(&p.text_document_position)?;
                let locs = self.analysis.references(&path, offset, p.context.include_declaration);
                let out: Vec<Location> = locs.iter().filter_map(|l| self.location(l)).collect();
                json(serde_json::to_value(out).unwrap())
            }
            request::PrepareRenameRequest::METHOD => {
                let p: TextDocumentPositionParams = params(&req)?;
                let (path, _, offset) = self.place(&p)?;
                match self.analysis.prepare_rename(&path, offset) {
                    Ok(l) => json(serde_json::to_value(self.location(&l).map(|l| PrepareRenameResponse::Range(l.range))).unwrap()),
                    Err(msg) => Err((ErrorCode::RequestFailed, msg)),
                }
            }
            request::Rename::METHOD => {
                let p: RenameParams = params(&req)?;
                let (path, _, offset) = self.place(&p.text_document_position)?;
                let edits = self.analysis.rename(&path, offset, &p.new_name).map_err(|m| (ErrorCode::RequestFailed, m))?;
                json(serde_json::to_value(self.workspace_edit(&edits)).unwrap())
            }
            request::DocumentSymbolRequest::METHOD => {
                let p: DocumentSymbolParams = params(&req)?;
                let path = uri_to_path(&p.text_document.uri);
                let text = self.analysis.text(&path).unwrap_or_default();
                let index = LineIndex::new(&text);
                let items = self.analysis.outline(&path);
                let symbols: Vec<DocumentSymbol> = items.iter().map(|i| self.symbol(&text, &index, i)).collect();
                json(serde_json::to_value(DocumentSymbolResponse::Nested(symbols)).unwrap())
            }
            request::Formatting::METHOD => {
                let p: DocumentFormattingParams = params(&req)?;
                let path = uri_to_path(&p.text_document.uri);
                let text = self.analysis.text(&path).unwrap_or_default();
                let index = LineIndex::new(&text);
                let indent = if p.options.insert_spaces { " ".repeat(p.options.tab_size.clamp(1, 16) as usize) } else { "\t".into() };
                let edits: Vec<TextEdit> =
                    self.analysis.format(&path, &indent).into_iter().map(|e| TextEdit { range: self.range(&text, &index, e.start, e.end), new_text: e.text }).collect();
                json(serde_json::to_value(edits).unwrap())
            }
            request::RangeFormatting::METHOD => {
                let p: DocumentRangeFormattingParams = params(&req)?;
                let path = uri_to_path(&p.text_document.uri);
                let text = self.analysis.text(&path).unwrap_or_default();
                let index = LineIndex::new(&text);
                let indent = if p.options.insert_spaces { " ".repeat(p.options.tab_size.clamp(1, 16) as usize) } else { "\t".into() };
                let (start, end) = (self.offset(&text, &index, p.range.start), self.offset(&text, &index, p.range.end));
                let edits: Vec<TextEdit> = self
                    .analysis
                    .format_range(&path, start, end, &indent)
                    .into_iter()
                    .map(|e| TextEdit { range: self.range(&text, &index, e.start, e.end), new_text: e.text })
                    .collect();
                json(serde_json::to_value(edits).unwrap())
            }
            request::OnTypeFormatting::METHOD => {
                let p: DocumentOnTypeFormattingParams = params(&req)?;
                let path = uri_to_path(&p.text_document_position.text_document.uri);
                let text = self.analysis.text(&path).unwrap_or_default();
                let index = LineIndex::new(&text);
                let offset = self.offset(&text, &index, p.text_document_position.position);
                // (the character typed ends right before the caret; VS Code
                // sends "\n" for Enter, whatever the file's line ends)
                let ch = p.ch.chars().next().unwrap_or(' ');
                let offset = if ch == '\n' { offset } else { text[..offset].rfind(ch).map_or(offset, |i| i + ch.len_utf8()) };
                let edits: Vec<TextEdit> = self
                    .analysis
                    .case_edits(&path, rapidr_langsvc::CaseScope::Typed { offset, ch })
                    .into_iter()
                    .map(|e| TextEdit { range: self.range(&text, &index, e.start, e.end), new_text: e.text })
                    .collect();
                json(serde_json::to_value(edits).unwrap())
            }
            request::SemanticTokensFullRequest::METHOD => {
                let p: SemanticTokensParams = params(&req)?;
                let path = uri_to_path(&p.text_document.uri);
                let text = self.analysis.text(&path).unwrap_or_default();
                let index = LineIndex::new(&text);
                let mut data = Vec::new();
                let (mut last_line, mut last_col) = (0u32, 0u32);
                for t in self.analysis.semantic_tokens(&path) {
                    let start = self.position(&text, &index, t.start);
                    let end = self.position(&text, &index, t.end);
                    if end.line != start.line || end.character <= start.character {
                        continue;
                    }
                    let (ty, mut mods) = match t.kind {
                        TokenKind::Function => (0, 0),
                        TokenKind::Variable | TokenKind::Component => (1, 0),
                        TokenKind::Parameter => (2, 0),
                        TokenKind::Property => (3, 0),
                        TokenKind::Type => (4, 0),
                        TokenKind::Constant => (5, 0),
                        TokenKind::Label => continue,
                    };
                    if t.modifiers & rapidr_langsvc::modifiers::DECLARATION != 0 {
                        mods |= 1;
                    }
                    if t.modifiers & rapidr_langsvc::modifiers::READONLY != 0 {
                        mods |= 2;
                    }
                    if t.modifiers & rapidr_langsvc::modifiers::STATIC != 0 {
                        mods |= 4;
                    }
                    let delta_line = start.line - last_line;
                    let delta_start = if delta_line == 0 { start.character - last_col } else { start.character };
                    data.push(SemanticToken { delta_line, delta_start, length: end.character - start.character, token_type: ty, token_modifiers_bitset: mods });
                    last_line = start.line;
                    last_col = start.character;
                }
                json(serde_json::to_value(SemanticTokensResult::Tokens(SemanticTokens { result_id: None, data })).unwrap())
            }
            request::CodeActionRequest::METHOD => {
                let p: CodeActionParams = params(&req)?;
                let path = uri_to_path(&p.text_document.uri);
                let text = self.analysis.text(&path).unwrap_or_default();
                let index = LineIndex::new(&text);
                let start = self.offset(&text, &index, p.range.start);
                let end = self.offset(&text, &index, p.range.end);
                let actions: Vec<CodeActionOrCommand> = self
                    .analysis
                    .code_actions(&path, start, end)
                    .into_iter()
                    .map(|a| {
                        CodeActionOrCommand::CodeAction(CodeAction {
                            title: a.title,
                            kind: Some(CodeActionKind::QUICKFIX),
                            diagnostics: a.fixes.map(|d| vec![self.diagnostic(&text, &index, &d)]),
                            edit: Some(self.workspace_edit(&a.edit)),
                            is_preferred: Some(a.preferred),
                            ..Default::default()
                        })
                    })
                    .collect();
                json(serde_json::to_value(actions).unwrap())
            }
            _ => Err((ErrorCode::MethodNotFound, format!("rapidr lsp doesn't answer {}", req.method))),
        }
    }

    /// The file, its text and the byte offset of a position.
    fn place(&self, p: &TextDocumentPositionParams) -> Result<(PathBuf, String, usize), (ErrorCode, String)> {
        let path = uri_to_path(&p.text_document.uri);
        let text = self.analysis.text(&path).ok_or((ErrorCode::InvalidParams, format!("unknown document {}", p.text_document.uri.as_str())))?;
        let index = LineIndex::new(&text);
        let offset = self.offset(&text, &index, p.position);
        Ok((path, text, offset))
    }

    fn offset(&self, text: &str, index: &LineIndex, p: Position) -> usize {
        if self.utf8 {
            index.line_start(p.line as usize).map_or(text.len(), |s| (s + p.character as usize).min(index.line_end(text, p.line as usize)))
        } else {
            index.from_utf16(text, p.line, p.character)
        }
    }

    fn position(&self, text: &str, index: &LineIndex, offset: usize) -> Position {
        if self.utf8 {
            let (l, c) = index.line_col(offset);
            Position::new(l as u32, c as u32)
        } else {
            let (l, c) = index.to_utf16(text, offset);
            Position::new(l, c)
        }
    }

    fn range(&self, text: &str, index: &LineIndex, start: usize, end: usize) -> Range {
        Range::new(self.position(text, index, start), self.position(text, index, end))
    }

    /// The length of a text in the client's position units.
    fn units(&self, s: &str) -> u32 {
        if self.utf8 {
            s.len() as u32
        } else {
            s.encode_utf16().count() as u32
        }
    }

    fn location(&self, l: &rapidr_langsvc::Location) -> Option<Location> {
        let text = self.analysis.text(&l.file)?;
        let index = LineIndex::new(&text);
        Some(Location { uri: path_to_uri(&l.file)?, range: self.range(&text, &index, l.start, l.end) })
    }

    // (lsp-types' map is keyed by Uri, whose hash is its text: never mutated)
    #[allow(clippy::mutable_key_type)]
    fn workspace_edit(&self, edits: &rapidr_langsvc::WorkspaceEdit) -> WorkspaceEdit {
        let mut changes = HashMap::new();
        for (file, es) in edits {
            let Some(text) = self.analysis.text(file) else { continue };
            let index = LineIndex::new(&text);
            let Some(uri) = path_to_uri(file) else { continue };
            changes.insert(uri, es.iter().map(|e| TextEdit { range: self.range(&text, &index, e.start, e.end), new_text: e.text.clone() }).collect());
        }
        WorkspaceEdit { changes: Some(changes), ..Default::default() }
    }

    #[allow(deprecated)]
    fn symbol(&self, text: &str, index: &LineIndex, i: &rapidr_langsvc::OutlineItem) -> DocumentSymbol {
        DocumentSymbol {
            name: i.name.clone(),
            detail: i.detail.clone(),
            kind: match i.kind {
                OutlineKind::Sub | OutlineKind::Function => SymbolKind::FUNCTION,
                OutlineKind::Type => SymbolKind::STRUCT,
                OutlineKind::Field => SymbolKind::FIELD,
                OutlineKind::Method => SymbolKind::METHOD,
                OutlineKind::Event => SymbolKind::EVENT,
                OutlineKind::Component => SymbolKind::OBJECT,
                OutlineKind::Constant => SymbolKind::CONSTANT,
                OutlineKind::Variable => SymbolKind::VARIABLE,
                OutlineKind::Label => SymbolKind::KEY,
            },
            tags: None,
            deprecated: None,
            range: self.range(text, index, i.start, i.end),
            selection_range: self.range(text, index, i.name_start, i.name_end),
            children: (!i.children.is_empty()).then(|| i.children.iter().map(|c| self.symbol(text, index, c)).collect()),
        }
    }

    fn diagnostic(&self, text: &str, index: &LineIndex, d: &rapidr_langsvc::FileDiagnostic) -> Diagnostic {
        Diagnostic {
            range: self.range(text, index, d.start, d.end),
            severity: Some(match d.severity {
                Severity::Error => DiagnosticSeverity::ERROR,
                Severity::Warning => DiagnosticSeverity::WARNING,
                Severity::Note => DiagnosticSeverity::INFORMATION,
            }),
            code: d.code.clone().map(NumberOrString::String),
            source: Some("rapidr".into()),
            message: d.message.clone(),
            ..Default::default()
        }
    }

    /// Diagnostics of every open main file (an `$INCLUDE`d file gets those
    /// of the programs that include it).
    fn publish_diagnostics(&mut self) -> Vec<Notification> {
        let mut by_file: HashMap<PathBuf, Vec<Diagnostic>> = HashMap::new();
        let roots: Vec<PathBuf> = self.open.iter().filter(|p| !is_include(p)).cloned().collect();
        let mut now: HashMap<PathBuf, HashSet<PathBuf>> = HashMap::new();
        for root in &roots {
            let diags = self.analysis.diagnostics(root);
            let files = now.entry(root.clone()).or_default();
            files.insert(root.clone());
            for d in diags {
                let Some(text) = self.analysis.text(&d.file) else { continue };
                let index = LineIndex::new(&text);
                files.insert(d.file.clone());
                by_file.entry(d.file.clone()).or_default().push(self.diagnostic(&text, &index, &d));
            }
        }
        // Files that had diagnostics and have none now are cleared.
        let mut targets: HashSet<PathBuf> = now.values().flatten().cloned().collect();
        targets.extend(self.published.values().flatten().cloned());
        self.published = now;
        targets
            .into_iter()
            .filter_map(|file| {
                let uri = path_to_uri(&file)?;
                let diagnostics = by_file.remove(&file).unwrap_or_default();
                Some(Notification::new(notification::PublishDiagnostics::METHOD.into(), PublishDiagnosticsParams { uri, diagnostics, version: None }))
            })
            .collect()
    }
}

fn is_include(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("inc"))
}

fn completion_kind(k: CompletionKind) -> CompletionItemKind {
    match k {
        CompletionKind::Keyword => CompletionItemKind::KEYWORD,
        CompletionKind::Builtin => CompletionItemKind::FUNCTION,
        CompletionKind::Sub | CompletionKind::Function => CompletionItemKind::FUNCTION,
        CompletionKind::Method => CompletionItemKind::METHOD,
        CompletionKind::Property => CompletionItemKind::PROPERTY,
        CompletionKind::Event => CompletionItemKind::EVENT,
        CompletionKind::Variable | CompletionKind::Parameter => CompletionItemKind::VARIABLE,
        CompletionKind::Field => CompletionItemKind::FIELD,
        CompletionKind::Constant => CompletionItemKind::CONSTANT,
        CompletionKind::Component => CompletionItemKind::CLASS,
        CompletionKind::Type => CompletionItemKind::STRUCT,
        CompletionKind::Directive => CompletionItemKind::KEYWORD,
        CompletionKind::Label => CompletionItemKind::REFERENCE,
    }
}

// ---------------------------------------------------------------- URIs

/// A `file:` URI's path (percent-decoded; `file:///c%3A/x` → `c:/x` on
/// Windows). Other schemes (`untitled:`) get a path of their own name.
pub fn uri_to_path(uri: &Uri) -> PathBuf {
    let s = uri.as_str();
    let Some(rest) = s.strip_prefix("file://") else {
        // (an unsaved buffer: a name no file has)
        return PathBuf::from(format!("/__untitled__/{}", percent_decode(s.split_once(':').map_or(s, |(_, r)| r)).replace('/', "_")));
    };
    // (the authority, usually empty)
    let path = rest.find('/').map_or(rest, |i| &rest[i..]);
    let decoded = percent_decode(path);
    if cfg!(windows) {
        let trimmed = decoded.strip_prefix('/').unwrap_or(&decoded);
        if trimmed.as_bytes().get(1) == Some(&b':') {
            return PathBuf::from(trimmed.replace('/', "\\"));
        }
    }
    PathBuf::from(decoded)
}

/// A path's `file:` URI.
pub fn path_to_uri(path: &Path) -> Option<Uri> {
    let s = path.to_string_lossy().replace('\\', "/");
    if let Some(name) = s.strip_prefix("/__untitled__/") {
        return Uri::from_str(&format!("untitled:{name}")).ok();
    }
    let s = if s.starts_with('/') { s } else { format!("/{s}") };
    let mut out = String::from("file://");
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    Uri::from_str(&out).ok()
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Ids are unused by the server's own requests (it sends none).
#[allow(dead_code)]
fn _id(n: i32) -> RequestId {
    RequestId::from(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uris_round_trip() {
        let p = PathBuf::from("/tmp/a b/é.bas");
        let u = path_to_uri(&p).unwrap();
        assert_eq!(u.as_str(), "file:///tmp/a%20b/%C3%A9.bas");
        assert_eq!(uri_to_path(&u), p);
        let untitled = Uri::from_str("untitled:Untitled-1").unwrap();
        let path = uri_to_path(&untitled);
        assert_eq!(path_to_uri(&path).unwrap().as_str(), "untitled:Untitled-1");
    }
}
