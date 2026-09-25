use std::error::Error;
use std::fmt;
use std::path::Path;

use rapidr_ast::*;
use rapidr_diagnostics::{Diagnostic, Severity, SourceLocation, TextSpan};
use rapidr_lexer::{lex_file, Token, TokenType};

/// Every error found while lexing/parsing. The parser recovers line by line,
/// so one run reports all syntax errors in the file, not just the first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub diagnostics: Vec<Diagnostic>,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, d) in self.diagnostics.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{d}")?;
        }
        Ok(())
    }
}

impl Error for ParseError {}

pub fn parse_file(path: impl AsRef<Path>) -> Result<Program, ParseError> {
    let path = path.as_ref();
    let tokens = lex_file(path).map_err(|e| ParseError { diagnostics: vec![e.diagnostic] })?;
    parse_tokens(&tokens).map_err(|mut e| {
        for d in &mut e.diagnostics {
            d.file_path.get_or_insert_with(|| path.display().to_string());
        }
        e
    })
}

/// Parses a token stream, failing if any statement could not be parsed.
/// Nothing is ever silently skipped: an unparseable line is an error.
pub fn parse_tokens(tokens: &[Token]) -> Result<Program, ParseError> {
    let (program, diagnostics) = parse_tokens_recovering(tokens);
    if diagnostics.iter().any(|d| d.severity == Severity::Error) {
        Err(ParseError { diagnostics })
    } else {
        Ok(program)
    }
}

/// Parses as much as possible and returns the program together with every
/// diagnostic, for tools (editors) that want a best-effort tree.
pub fn parse_tokens_recovering(tokens: &[Token]) -> (Program, Vec<Diagnostic>) {
    let retagged = retag_routine_names(tokens);
    let mut parser = Parser::new(retagged.as_deref().unwrap_or(tokens));
    let program = parser.parse_program();
    (program, parser.diagnostics)
}

/// Statement keywords of QBasic-style file I/O that RapidQ doesn't reserve
/// (it uses QFileStream), so programs name their own SUBs `Open`, `Close`, …
const SOFT_KEYWORDS: &[TokenType] = &[
    TokenType::Open,
    TokenType::Close,
    TokenType::Write,
    TokenType::Seek,
    TokenType::Kill,
    TokenType::Input,
    TokenType::Bind,
];

/// When a program declares a SUB/FUNCTION named like a soft keyword
/// (`SUB Close`, `DECLARE SUB Open`), every use of that word is the routine:
/// retag those tokens as identifiers. `None` when nothing needs retagging.
fn retag_routine_names(tokens: &[Token]) -> Option<Vec<Token>> {
    let mut names: Vec<String> = Vec::new();
    for pair in tokens.windows(2) {
        if matches!(pair[0].kind, TokenType::Sub | TokenType::Function) && SOFT_KEYWORDS.contains(&pair[1].kind) {
            let upper = pair[1].lexeme.to_ascii_uppercase();
            if !names.contains(&upper) {
                names.push(upper);
            }
        }
    }
    if names.is_empty() {
        return None;
    }
    Some(
        tokens
            .iter()
            .map(|t| {
                let mut t = t.clone();
                if SOFT_KEYWORDS.contains(&t.kind) && names.contains(&t.lexeme.to_ascii_uppercase()) {
                    t.kind = TokenType::Identifier;
                }
                t
            })
            .collect(),
    )
}

// ---------------------------------------------------------------------------
// Parser — recursive descent over the full token stream
// ---------------------------------------------------------------------------

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    diagnostics: Vec<Diagnostic>,
    /// Extra statements produced by the last parsed statement (a DIM with
    /// several types or initializers becomes several statements); the
    /// statement lists drain them right after it.
    pending: Vec<Statement>,
    /// Numbers the hidden variables of rewritten `FOR obj.x` loops.
    for_counter: usize,
    /// SUBI/FUNCTIONI names (lowercase): their calls pack the arguments.
    variadic: Vec<String>,
    /// Every DATA item of the program, in source order.
    data_items: Vec<Expression>,
    /// `RESTORE label` targets: label → index of the next DATA item after it.
    data_labels: Vec<(String, usize)>,
    /// Labels seen since the last DATA statement.
    labels_awaiting_data: Vec<String>,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0, diagnostics: Vec::new(), pending: Vec::new(), variadic: Vec::new(), for_counter: 0, data_items: Vec::new(), data_labels: Vec::new(), labels_awaiting_data: Vec::new() }
    }

    // --- diagnostics ---

    /// Records an error at token `index`. Only the first error per line is
    /// kept: after a failure the rest of the line is skipped, and follow-on
    /// errors from enclosing constructs would just repeat it.
    fn error_at(&mut self, index: usize, message: String) {
        let tok = match self.tokens.get(index).or_else(|| self.tokens.last()) {
            Some(t) => t,
            None => return,
        };
        if self.diagnostics.iter().any(|d| d.location.line == tok.line) {
            return;
        }
        self.diagnostics.push(Diagnostic::error(
            message,
            tok.span,
            SourceLocation::new(tok.line, tok.column),
            None,
        ));
    }

    /// Explains why the statement starting at token `start` failed to parse.
    fn describe_failure(&self, start: usize) -> String {
        let Some(tok) = self.tokens.get(start) else {
            return "Syntax error: unexpected end of file".into();
        };
        let word = tok.lexeme.to_uppercase();
        // `DIM step AS ...`: a keyword where a name is expected.
        if matches!(tok.kind, TokenType::Dim | TokenType::Const) {
            if let Some(next) = self.tokens.get(start + 1) {
                let is_word = next.lexeme.chars().all(|c| c.is_ascii_alphabetic());
                if next.kind != TokenType::Identifier && is_word {
                    return format!(
                        "'{}' is a reserved word and can't be used as a name",
                        next.lexeme.to_uppercase()
                    );
                }
            }
        }
        match tok.kind {
            TokenType::Goto | TokenType::Gosub => format!("{word} needs a label: `{word} Name` or `{word} 100`"),
            TokenType::Identifier | TokenType::Number | TokenType::StringLit => {
                format!("Syntax error near '{}'", tok.lexeme)
            }
            _ if tok.lexeme.chars().all(|c| c.is_ascii_alphabetic()) => {
                format!("Syntax error in {word} statement")
            }
            _ => format!("Syntax error: unexpected '{}'", tok.lexeme),
        }
    }

    // --- token helpers ---

    fn peek(&self) -> Option<&'a Token> {
        self.tokens.get(self.pos)
    }

    fn peek_kind(&self) -> Option<TokenType> {
        self.peek().map(|t| t.kind)
    }

    fn peek_kind_at(&self, offset: usize) -> Option<TokenType> {
        self.tokens.get(self.pos + offset).map(|t| t.kind)
    }

    fn advance(&mut self) -> Option<&'a Token> {
        let t = self.peek()?;
        self.pos += 1;
        Some(t)
    }

    fn expect(&mut self, kind: TokenType) -> Option<&'a Token> {
        if self.peek_kind() == Some(kind) {
            self.advance()
        } else {
            None
        }
    }

    fn match_kind(&mut self, kind: TokenType) -> bool {
        if self.peek_kind() == Some(kind) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn skip_newlines(&mut self) {
        while self.peek_kind() == Some(TokenType::Newline) {
            self.pos += 1;
        }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
            || self.peek_kind() == Some(TokenType::Eof)
    }

    /// True when the current position is at the end of a logical line —
    /// i.e. at a newline, EOF, or past-the-end.
    fn at_eol(&self) -> bool {
        self.at_end() || self.peek_kind() == Some(TokenType::Newline)
    }

    fn skip_to_eol(&mut self) {
        while !self.at_eol() {
            self.pos += 1;
        }
    }

    /// Skips line ends and `:` separators, so a block may start on its
    /// header's line: `FOR i = 1 TO 3: PRINT i: NEXT`.
    fn consume_eol(&mut self) {
        while self.match_kind(TokenType::Newline) || self.match_kind(TokenType::Colon) {}
    }

    fn previous(&self) -> Option<&'a Token> {
        self.pos.checked_sub(1).and_then(|i| self.tokens.get(i))
    }

    fn current_span(&self) -> TextSpan {
        self.peek().map(|t| t.span).unwrap_or_default()
    }

    fn span_from(&self, start: usize) -> TextSpan {
        let s = self.tokens.get(start).map(|t| t.span.start).unwrap_or(0);
        let e = self
            .pos
            .checked_sub(1)
            .and_then(|i| self.tokens.get(i))
            .map(|t| t.span.end)
            .unwrap_or(s);
        TextSpan::new(s, e)
    }

    // --- identifiers (case-insensitive keyword check) ---

    fn peek_identifier_eq(&self, value: &str) -> bool {
        matches!(self.peek(), Some(t) if t.lexeme.eq_ignore_ascii_case(value))
    }

    fn peek_is_end_followed_by(&self, kw: &str) -> bool {
        self.peek_kind() == Some(TokenType::End)
            && self.tokens.get(self.pos + 1).map_or(false, |t| {
                t.lexeme.eq_ignore_ascii_case(kw)
            })
    }

    // -----------------------------------------------------------------------
    // Program
    // -----------------------------------------------------------------------

    fn parse_program(&mut self) -> Program {
        let start = self.pos;
        let mut body = self.parse_body(&[]);
        pack_variadic_calls(&mut body, &self.variadic);
        if !self.data_items.is_empty() {
            let mut init = data_table_init(&self.data_items, &self.data_labels);
            init.append(&mut body);
            body = init;
        }
        let span = if body.is_empty() {
            TextSpan::default()
        } else {
            self.span_from(start)
        };
        Program {
            span,
            statements: body,
        }
    }

    /// Parse a sequence of statements until we hit one of `terminators`
    /// (case-insensitive keyword pairs such as `("END", "SUB")`).
    ///
    /// Block-level terminators like `NEXT`, `WEND`, `LOOP`, `END IF` etc.
    /// are checked here, and we return when we hit one so the calling
    /// block-level parser can consume it.
    fn parse_body(&mut self, terminators: &[Terminator]) -> Vec<Statement> {
        let mut stmts = Vec::new();
        loop {
            self.skip_newlines();
            if self.at_end() {
                break;
            }
            if self.is_at_terminator(terminators) {
                break;
            }
            if let Some(label) = self.parse_line_label() {
                if let Statement::Label(l) = &label {
                    self.labels_awaiting_data.push(l.name.clone());
                }
                stmts.push(label);
                if self.at_eol() {
                    self.consume_eol();
                    continue;
                }
            }
            if let Some(stmt) = self.parse_statement() {
                stmts.push(stmt);
                stmts.append(&mut self.pending);
                // Handle colon-separated statements on the same line
                while self.match_kind(TokenType::Colon) {
                    if self.at_end() || self.peek_kind() == Some(TokenType::Newline) {
                        break;
                    }
                    // `…: NEXT` / `…: WEND` ends the block on this line.
                    if self.is_at_terminator(terminators) {
                        break;
                    }
                    if let Some(s) = self.parse_statement() {
                        stmts.push(s);
                        stmts.append(&mut self.pending);
                    } else {
                        break;
                    }
                }
            }
            // Anything left on the line was not consumed by the statement
            // (e.g. `x = 1 y = 2`): report it instead of re-parsing it as a
            // new statement, then recover at the next line.
            if self.is_at_terminator(terminators) {
                continue;
            }
            if !self.at_eol() {
                let tok = &self.tokens[self.pos];
                let message = format!("Unexpected '{}' after the end of the statement", tok.lexeme);
                self.error_at(self.pos, message);
                self.skip_to_eol();
            }
            self.consume_eol();
        }
        stmts
    }

    fn is_at_terminator(&self, terminators: &[Terminator]) -> bool {
        for term in terminators {
            match term {
                Terminator::Keyword(kw) => {
                    if self.peek().map_or(false, |t| t.lexeme.eq_ignore_ascii_case(kw)) {
                        return true;
                    }
                }
                Terminator::EndPair(kw) => {
                    if self.peek_is_end_followed_by(kw) {
                        return true;
                    }
                }
            }
        }
        false
    }

    // -----------------------------------------------------------------------
    // Statement dispatch
    // -----------------------------------------------------------------------

    /// Parses one statement. On failure an error is recorded — callers may
    /// then skip ahead, but the program will not compile.
    fn parse_statement(&mut self) -> Option<Statement> {
        let start = self.pos;
        let stmt = self.parse_statement_inner();
        if stmt.is_none() {
            let message = self.describe_failure(start);
            self.error_at(start, message);
        }
        stmt
    }

    fn parse_statement_inner(&mut self) -> Option<Statement> {
        match self.peek_kind()? {
            TokenType::Directive => self.parse_directive(),
            TokenType::Dim => self.parse_declaration(None),
            TokenType::DefStr => self.parse_declaration(Some("STRING")),
            TokenType::DefInt => self.parse_declaration(Some("INTEGER")),
            TokenType::DefByte => self.parse_declaration(Some("BYTE")),
            TokenType::DefWord => self.parse_declaration(Some("WORD")),
            TokenType::DefDword => self.parse_declaration(Some("DWORD")),
            TokenType::DefLong => self.parse_declaration(Some("LONG")),
            TokenType::DefShort => self.parse_declaration(Some("SHORT")),
            TokenType::DefSng => self.parse_declaration(Some("SINGLE")),
            TokenType::DefDbl => self.parse_declaration(Some("DOUBLE")),
            TokenType::DefCur => self.parse_declaration(Some("CURRENCY")),
            TokenType::Const => self.parse_const().map(Statement::Const),
            TokenType::Import => self.parse_import().map(Statement::Import),
            TokenType::Print => {
                // Check for PRINT #n  (file I/O)
                if self.peek_kind_at(1) == Some(TokenType::Hash) {
                    self.parse_print_hash().map(Statement::PrintHash)
                } else {
                    self.parse_print().map(Statement::Print)
                }
            }
            TokenType::Open => self.parse_open().map(Statement::Open),
            TokenType::Close => self.parse_close().map(Statement::Close),
            TokenType::Write => {
                // WRITE #n  (file I/O)
                if self.peek_kind_at(1) == Some(TokenType::Hash) {
                    self.parse_write_hash().map(Statement::WriteHash)
                } else {
                    self.parse_assignment_or_call()
                }
            }
            TokenType::Seek => self.parse_seek().map(Statement::Seek),
            TokenType::Kill => self.parse_kill(),
            TokenType::Call => self.parse_explicit_call().map(Statement::Call),
            TokenType::If => self.parse_if().map(Statement::If),
            // `FOR THIS.lx = 0 TO n` (a member as the loop variable)
            TokenType::For if self.peek_kind_at(2) == Some(TokenType::Dot) => self.parse_for_member(),
            TokenType::For => self.parse_for().map(Statement::For),
            TokenType::While => self.parse_while().map(Statement::While),
            TokenType::Do => self.parse_do_loop().map(Statement::DoLoop),
            TokenType::Select => self.parse_select_case().map(Statement::SelectCase),
            TokenType::Sub => self.parse_sub().map(Statement::Subroutine),
            TokenType::Function => self.parse_function().map(Statement::Function),
            TokenType::Type => self.parse_type_def().map(Statement::Type),
            TokenType::Create => self.parse_create().map(Statement::Create),
            TokenType::With => self.parse_with().map(Statement::With),
            TokenType::Exit => self.parse_exit().map(Statement::Exit),
            TokenType::Return => self.parse_return().map(Statement::Return),
            TokenType::Input => self.parse_input().map(Statement::Input),
            TokenType::Bind => self.parse_bind().map(Statement::Bind),
            TokenType::Declare => self.parse_declare().map(Statement::Declare),
            TokenType::RustStart => self.parse_rust_block().map(Statement::RustBlock),
            // A bare `END` ends the program (the END builtin). `END IF`,
            // `END SUB`, … are block terminators and never reach here.
            TokenType::End
                if matches!(self.peek_kind_at(1), None | Some(TokenType::Newline | TokenType::Colon | TokenType::Eof)) =>
            {
                let tok = self.advance()?;
                Some(Statement::Call(CallStatement {
                    span: tok.span,
                    callee: Expression::Identifier(Identifier { span: tok.span, name: "END".into() }),
                    args: Vec::new(),
                }))
            }
            // RapidQ `SUBI Name (...)` / `FUNCTIONI Name (...) AS T`: any number of arguments.
            TokenType::Identifier
                if (self.peek_identifier_eq("SUBI") || self.peek_identifier_eq("FUNCTIONI"))
                    && self.peek_kind_at(1) == Some(TokenType::Identifier) =>
            {
                self.parse_variadic_routine()
            }
            TokenType::Data => {
                let tok = self.advance()?;
                for label in std::mem::take(&mut self.labels_awaiting_data) {
                    self.data_labels.push((label, self.data_items.len()));
                }
                let span = tok.span;
                self.data_items.extend(
                    data_items(&tok.lexeme).into_iter().map(|value| Expression::Literal(Literal { span, value })),
                );
                Some(Statement::Comment(CommentStatement { span, text: format!("DATA{}", tok.lexeme) }))
            }
            // `READ a, s$, p(1, 2)`: each variable takes the next DATA item.
            TokenType::Identifier
                if self.peek_identifier_eq("READ")
                    && matches!(self.peek_kind_at(1), Some(TokenType::Identifier)) =>
            {
                let start = self.pos;
                self.advance();
                let mut reads = Vec::new();
                loop {
                    let target = self.parse_postfix_expression()?;
                    let span = self.span_from(start);
                    reads.push(Statement::Assignment(AssignmentStatement { span, target, value: call_expr(span, "__READ", Vec::new()) }));
                    if !self.match_kind(TokenType::Comma) {
                        break;
                    }
                }
                let first = reads.remove(0);
                self.pending = reads;
                Some(first)
            }
            // `RESTORE` / `RESTORE label`: where the next READ starts.
            TokenType::Identifier
                if self.peek_identifier_eq("RESTORE")
                    && matches!(self.peek_kind_at(1), None | Some(TokenType::Newline | TokenType::Colon | TokenType::Identifier | TokenType::Number | TokenType::Eof)) =>
            {
                let tok = self.advance()?;
                let span = tok.span;
                let args = match self.peek() {
                    Some(t) if matches!(t.kind, TokenType::Identifier | TokenType::Number) => {
                        let label = self.advance()?.lexeme.clone();
                        vec![Expression::Literal(Literal { span, value: LiteralValue::String(label) })]
                    }
                    _ => Vec::new(),
                };
                Some(Statement::Call(CallStatement { span, callee: ident(span, "__RESTORE"), args }))
            }
            // `SWAP a, b` exchanges two variables (through a hidden temporary).
            TokenType::Identifier
                if self.peek_identifier_eq("SWAP")
                    && matches!(self.peek_kind_at(1), Some(TokenType::Identifier)) =>
            {
                let start = self.pos;
                self.advance();
                let a = self.parse_postfix_expression()?;
                self.expect(TokenType::Comma)?;
                let b = self.parse_postfix_expression()?;
                let span = self.span_from(start);
                let tmp = ident(span, "__swap_tmp");
                let assign = |target: Expression, value: Expression| Statement::Assignment(AssignmentStatement { span, target, value });
                self.pending = vec![assign(a.clone(), b.clone()), assign(b, tmp.clone())];
                Some(assign(tmp, a))
            }
            // `REDIM a(n) AS T`: resize an array, keeping its data.
            TokenType::Identifier
                if self.peek_identifier_eq("REDIM")
                    && matches!(self.peek_kind_at(1), Some(TokenType::Identifier | TokenType::LParen)) =>
            {
                self.parse_declaration(None)
            }
            // `STATIC x AS T` — a DIM whose value survives between calls.
            TokenType::Identifier
                if self.peek_identifier_eq("STATIC")
                    && matches!(self.peek_kind_at(1), Some(TokenType::Identifier | TokenType::LParen)) =>
            {
                self.parse_declaration(None)
            }
            // VB-style scope modifiers: `Public Const X = 1`, `Private Sub F`,
            // `Global Const Y = 2`, `Public n As Long` (a DIM).
            TokenType::Identifier
                if (self.peek_identifier_eq("PUBLIC") || self.peek_identifier_eq("PRIVATE") || self.peek_identifier_eq("GLOBAL"))
                    && self.peek_kind_at(1) != Some(TokenType::Colon)
                    && !matches!(self.peek_kind_at(1), Some(TokenType::Eq | TokenType::LParen | TokenType::Dot | TokenType::Newline) | None) =>
            {
                if self.peek_kind_at(1) == Some(TokenType::Identifier) && !self.tokens[self.pos + 1].lexeme.eq_ignore_ascii_case("STATIC") {
                    self.parse_declaration(None)
                } else {
                    self.advance();
                    self.parse_statement_inner()
                }
            }
            TokenType::Goto => self.parse_jump().map(Statement::Goto),
            TokenType::Gosub => self.parse_jump().map(Statement::Gosub),
            _ => self.parse_assignment_or_call(),
        }
    }

    // -----------------------------------------------------------------------
    // Simple statements
    // -----------------------------------------------------------------------

    /// `GOTO label` / `GOSUB label` (a name or a line number).
    fn parse_jump(&mut self) -> Option<JumpStatement> {
        let start = self.pos;
        self.advance()?; // GOTO / GOSUB
        let tok = self.peek()?;
        if !matches!(tok.kind, TokenType::Identifier | TokenType::Number) {
            return None; // don't consume the next line
        }
        self.advance();
        Some(JumpStatement { span: self.span_from(start), label: tok.lexeme.clone() })
    }

    /// A label at the very start of a line: `Name:` or a line number
    /// (`100 PRINT "x"`). Returns None, consuming nothing, otherwise.
    fn parse_line_label(&mut self) -> Option<Statement> {
        let at_line_start = self.pos == 0 || self.previous().map_or(true, |t| t.kind == TokenType::Newline);
        if !at_line_start {
            return None;
        }
        let tok = self.peek()?;
        let is_label = match tok.kind {
            TokenType::Identifier => self.peek_kind_at(1) == Some(TokenType::Colon),
            TokenType::Number => tok.lexeme.chars().all(|c| c.is_ascii_digit()),
            _ => false,
        };
        if !is_label {
            return None;
        }
        let start = self.pos;
        let name = tok.lexeme.clone();
        self.advance();
        if tok.kind == TokenType::Identifier {
            self.advance(); // the ':'
        }
        Some(Statement::Label(LabelStatement { span: self.span_from(start), name }))
    }

    fn parse_directive(&mut self) -> Option<Statement> {
        let tok = self.advance()?;
        let span_start = tok.span.start;
        let name = tok.lexeme.clone();
        let value = tok.trailing.clone();
        // Directives consume the rest of the line
        self.skip_to_eol();
        let span = TextSpan::new(span_start, self.previous().map(|t| t.span.end).unwrap_or(span_start));
        Some(Statement::Directive(DirectiveStatement { span, name, value }))
    }

    /// `DIM` and RapidQ's `DEFINT`/`DEFSTR`/… (`fixed_type`), following the
    /// RapidQ manual:
    /// - `DIM a AS INTEGER, b(5) AS STRING` — each name takes the `AS` after it;
    ///   a name without one is a VARIANT (`DIM a, b AS LONG`: only b is LONG).
    /// - `DIM (a, b, c)(5) AS INTEGER` — one type (and dimensions) for a group.
    /// - `DIM x AS INTEGER = 5`, `DEFSTR s = "hi"`, `DEFINT a(1 TO 3) = {1, 2, 3}`.
    ///
    /// Each name becomes its own DIM statement followed by assignments for its
    /// initializer (queued in `pending`), so code generators see plain DIMs.
    fn parse_declaration(&mut self, fixed_type: Option<&str>) -> Option<Statement> {
        let keyword = self.advance()?.lexeme.to_ascii_uppercase();
        let is_static = keyword == "STATIC";
        let is_redim = keyword == "REDIM";
        // VB's `REDIM PRESERVE`: RapidQ's REDIM always keeps the data.
        if is_redim && self.peek_identifier_eq("PRESERVE") {
            self.advance();
        }
        let mut out = Vec::new();
        loop {
            let group_start = self.pos;
            let mut names = Vec::new();
            if self.match_kind(TokenType::LParen) {
                loop {
                    names.push(self.expect(TokenType::Identifier)?.clone());
                    if !self.match_kind(TokenType::Comma) {
                        break;
                    }
                }
                self.expect(TokenType::RParen)?;
            } else {
                names.push(self.expect(TokenType::Identifier)?.clone());
            }
            let dimensions = if self.match_kind(TokenType::LParen) {
                let dims = self.parse_array_dimensions()?;
                self.expect(TokenType::RParen)?;
                dims
            } else {
                Vec::new()
            };
            let type_name = match fixed_type {
                Some(t) => t.to_string(),
                None if self.match_kind(TokenType::As) => canonical_type_name(&self.advance()?.lexeme),
                None => "VARIANT".to_string(),
            };
            // `AS STRING * 20`: a fixed-length string, kept as a STRING.
            if self.match_kind(TokenType::Star) {
                self.parse_unary()?;
            }
            let init = if self.match_kind(TokenType::Eq) {
                if self.match_kind(TokenType::LBrace) {
                    let mut values = Vec::new();
                    self.skip_newlines();
                    while !self.match_kind(TokenType::RBrace) {
                        values.push(self.parse_expression()?);
                        self.skip_newlines();
                        if !self.match_kind(TokenType::Comma) {
                            self.skip_newlines();
                            self.expect(TokenType::RBrace)?;
                            break;
                        }
                        self.skip_newlines();
                    }
                    Some(Err(values))
                } else {
                    Some(Ok(self.parse_expression()?))
                }
            } else {
                None
            };
            let span = self.span_from(group_start);
            for name_tok in names {
                let decl_span = TextSpan::new(name_tok.span.start, span.end);
                out.push(Statement::Dim(DimStatement {
                    span: decl_span,
                    declarators: vec![VariableDeclarator {
                        span: decl_span,
                        name: name_tok.lexeme.clone(),
                        dimensions: dimensions.clone(),
                    }],
                    type_name: type_name.clone(),
                    is_static,
                    is_redim,
                }));
                match &init {
                    None => {}
                    Some(Ok(value)) => out.push(assign(decl_span, &name_tok.lexeme, Vec::new(), value.clone())),
                    Some(Err(values)) => {
                        let indices = initializer_indices(&dimensions, values.len()).or_else(|| {
                            self.error_at(group_start, "Array initializers need constant bounds on every dimension but the first".to_string());
                            None
                        })?;
                        for (index, value) in indices.into_iter().zip(values) {
                            out.push(assign(decl_span, &name_tok.lexeme, index, value.clone()));
                        }
                    }
                }
            }
            if !self.match_kind(TokenType::Comma) {
                break;
            }
        }
        let first = out.remove(0);
        self.pending = out;
        Some(first)
    }

    fn parse_array_dimensions(&mut self) -> Option<Vec<ArrayDimension>> {
        let mut dims = Vec::new();
        loop {
            let expr = self.parse_expression()?;
            if self.match_kind(TokenType::To) {
                let end = self.parse_expression()?;
                dims.push(ArrayDimension::Range { start: expr, end });
            } else {
                dims.push(ArrayDimension::Single(expr));
            }
            if !self.match_kind(TokenType::Comma) {
                break;
            }
        }
        Some(dims)
    }

    fn parse_const(&mut self) -> Option<ConstStatement> {
        let start = self.pos;
        self.expect(TokenType::Const)?;
        let name = self.expect(TokenType::Identifier)?.lexeme.clone();
        let declared_type = if self.match_kind(TokenType::As) {
            Some(self.advance()?.lexeme.clone())
        } else {
            None
        };
        self.expect(TokenType::Eq)?;
        let value = self.parse_expression()?;
        Some(ConstStatement {
            span: self.span_from(start),
            name,
            declared_type,
            value,
        })
    }

    fn parse_import(&mut self) -> Option<ImportStatement> {
        let start = self.pos;
        self.expect(TokenType::Import)?;
        let module = self.advance()?.lexeme.clone();
        let alias = if self.match_kind(TokenType::As) {
            Some(self.expect(TokenType::Identifier)?.lexeme.clone())
        } else {
            None
        };
        Some(ImportStatement {
            span: self.span_from(start),
            module_name: module,
            alias,
        })
    }

    fn parse_print(&mut self) -> Option<PrintStatement> {
        let start = self.pos;
        self.expect(TokenType::Print)?;
        let mut items = Vec::new();
        let mut zones = Vec::new();
        // Only a trailing `;` or `,` keeps the cursor on the line.
        let mut append_newline = true;
        while !self.at_eol() && self.peek_kind() != Some(TokenType::Colon) {
            items.push(self.parse_expression()?);
            let comma = self.match_kind(TokenType::Comma);
            let separated = comma || self.match_kind(TokenType::Semi);
            zones.push(comma);
            append_newline = !separated;
            if !separated {
                break;
            }
        }
        Some(PrintStatement {
            span: self.span_from(start),
            items,
            zones,
            append_newline,
        })
    }

    fn parse_explicit_call(&mut self) -> Option<CallStatement> {
        let start = self.pos;
        self.expect(TokenType::Call)?;
        let callee = self.parse_postfix_expression()?;
        let args = if self.at_eol() {
            extract_existing_call_args(&callee).unwrap_or_default()
        } else {
            self.parse_argument_list_without_parens()?
        };
        Some(CallStatement {
            span: self.span_from(start),
            callee: strip_inline_call_args(callee),
            args,
        })
    }

    fn parse_exit(&mut self) -> Option<ExitStatement> {
        let start = self.pos;
        self.advance()?; // EXIT
        let exit_type = match self.advance()?.lexeme.to_uppercase().as_str() {
            // SUBI/FUNCTIONI, EVENT and PROPERTY SET handlers are SUBs/FUNCTIONs.
            "SUBI" | "EVENT" | "PROPERTY" => "SUB".to_string(),
            "FUNCTIONI" => "FUNCTION".to_string(),
            other => other.to_string(),
        };
        Some(ExitStatement {
            span: self.span_from(start),
            exit_type,
        })
    }

    fn parse_return(&mut self) -> Option<ReturnStatement> {
        let start = self.pos;
        self.advance()?; // RETURN
        let value = if self.at_eol() {
            None
        } else {
            Some(self.parse_expression()?)
        };
        Some(ReturnStatement {
            span: self.span_from(start),
            value,
        })
    }

    fn parse_input(&mut self) -> Option<InputStatement> {
        let start = self.pos;
        self.advance()?; // INPUT
        let prompt = if self.peek_kind() == Some(TokenType::StringLit) {
            let p = self.parse_expression()?;
            if self.match_kind(TokenType::Comma) || self.match_kind(TokenType::Semi) {
                Some(p)
            } else {
                // No separator — the string was the target identifier? Unlikely, treat as prompt-less.
                return Some(InputStatement {
                    span: self.span_from(start),
                    prompt: None,
                    target: p,
                });
            }
        } else {
            None
        };
        let target = self.parse_expression()?;
        Some(InputStatement {
            span: self.span_from(start),
            prompt,
            target,
        })
    }

    /// OPEN filename FOR mode AS #n
    fn parse_open(&mut self) -> Option<OpenStatement> {
        let start = self.pos;
        self.advance()?; // OPEN
        let filename = self.parse_expression()?;
        // expect FOR
        if !self.peek_identifier_eq("FOR") {
            return None;
        }
        self.advance(); // FOR
        // mode: INPUT, OUTPUT, APPEND, BINARY
        let mode_tok = self.advance()?;
        let mode = mode_tok.lexeme.to_ascii_uppercase();
        // expect AS
        if !self.peek_identifier_eq("AS") {
            return None;
        }
        self.advance(); // AS
        // optional #
        self.match_kind(TokenType::Hash);
        let file_number = self.parse_expression()?;
        Some(OpenStatement {
            span: self.span_from(start),
            filename,
            mode,
            file_number,
        })
    }

    /// CLOSE #n
    fn parse_close(&mut self) -> Option<CloseStatement> {
        let start = self.pos;
        self.advance()?; // CLOSE
        // optional #
        self.match_kind(TokenType::Hash);
        let file_number = self.parse_expression()?;
        Some(CloseStatement {
            span: self.span_from(start),
            file_number,
        })
    }

    /// PRINT #n, items...
    fn parse_print_hash(&mut self) -> Option<PrintHashStatement> {
        let start = self.pos;
        self.advance()?; // PRINT
        self.expect(TokenType::Hash)?;
        let file_number = self.parse_expression()?;
        let mut items = Vec::new();
        if self.match_kind(TokenType::Comma) {
            while !self.at_eol() {
                items.push(self.parse_expression()?);
                if !self.match_kind(TokenType::Comma) && !self.match_kind(TokenType::Semi) {
                    break;
                }
            }
        }
        Some(PrintHashStatement {
            span: self.span_from(start),
            file_number,
            items,
        })
    }

    /// WRITE #n, items...
    fn parse_write_hash(&mut self) -> Option<WriteHashStatement> {
        let start = self.pos;
        self.advance()?; // WRITE
        self.expect(TokenType::Hash)?;
        let file_number = self.parse_expression()?;
        let mut items = Vec::new();
        if self.match_kind(TokenType::Comma) {
            while !self.at_eol() {
                items.push(self.parse_expression()?);
                if !self.match_kind(TokenType::Comma) && !self.match_kind(TokenType::Semi) {
                    break;
                }
            }
        }
        Some(WriteHashStatement {
            span: self.span_from(start),
            file_number,
            items,
        })
    }

    /// SEEK #n, position
    fn parse_seek(&mut self) -> Option<SeekStatement> {
        let start = self.pos;
        self.advance()?; // SEEK
        // optional #
        self.match_kind(TokenType::Hash);
        let file_number = self.parse_expression()?;
        self.expect(TokenType::Comma)?;
        let position = self.parse_expression()?;
        Some(SeekStatement {
            span: self.span_from(start),
            file_number,
            position,
        })
    }

    /// KILL "filename"  — emit as a function call to KILL()
    fn parse_kill(&mut self) -> Option<Statement> {
        let start = self.pos;
        self.advance()?; // KILL
        let path_expr = self.parse_expression()?;
        let span = self.span_from(start);
        let callee = Expression::Identifier(Identifier {
            span,
            name: "KILL".to_string(),
        });
        Some(Statement::Call(CallStatement {
            span,
            callee,
            args: vec![path_expr],
        }))
    }

    fn parse_bind(&mut self) -> Option<BindStatement> {
        let start = self.pos;
        self.advance()?; // BIND
        let target = self.parse_expression()?;
        // expect TO
        self.advance()?;
        let handler = self.parse_expression()?;
        Some(BindStatement {
            span: self.span_from(start),
            target,
            handler,
        })
    }

    /// `SUBI Name (...)` … `END SUBI` / `FUNCTIONI Name (...) AS T` …
    /// `END FUNCTIONI` (manual ch. 9): a routine taking any number of
    /// arguments, read with ParamStr$(i) / ParamVal(i) / ParamStrCount /
    /// ParamValCount. It gets one hidden parameter holding the packed
    /// arguments; `pack_variadic_calls` packs them at every call.
    fn parse_variadic_routine(&mut self) -> Option<Statement> {
        let start = self.pos;
        let is_function = self.advance()?.lexeme.eq_ignore_ascii_case("FUNCTIONI");
        let name = self.advance()?.lexeme.clone();
        self.variadic.push(name.to_ascii_lowercase());
        self.skip_variadic_params();
        let return_type = if is_function && self.match_kind(TokenType::As) {
            Some(canonical_type_name(&self.advance()?.lexeme))
        } else {
            None
        };
        self.consume_eol();
        let keyword = if is_function { "FUNCTIONI" } else { "SUBI" };
        let mut body = self.parse_body(&[Terminator::EndPair(keyword)]);
        if self.peek_is_end_followed_by(keyword) {
            self.advance();
            self.advance();
        } else {
            self.error_at(start, format!("{keyword} {name} is missing END {keyword}"));
        }
        rewrite_param_access(&mut body);
        let span = self.span_from(start);
        let params = vec![Parameter { span, name: VARIADIC_PARAM.into(), type_name: "VARIANT".into(), by_ref: false, is_array: false }];
        Some(if is_function {
            Statement::Function(FunctionStatement { span, name, params, return_type, body })
        } else {
            Statement::Subroutine(SubroutineStatement { span, name, params, body })
        })
    }

    /// `(...)` or `(anything)` after a SUBI/FUNCTIONI name.
    fn skip_variadic_params(&mut self) {
        if self.match_kind(TokenType::LParen) {
            while !self.at_eol() && !self.match_kind(TokenType::RParen) {
                self.pos += 1;
            }
        }
    }

    fn parse_declare(&mut self) -> Option<DeclareStatement> {
        let start = self.pos;
        self.advance()?; // DECLARE
        // `DECLARE SUBI/FUNCTIONI Name (...)` only announces a variadic routine.
        if (self.peek_identifier_eq("SUBI") || self.peek_identifier_eq("FUNCTIONI"))
            && self.peek_kind_at(1) == Some(TokenType::Identifier)
        {
            let is_function = self.advance()?.lexeme.eq_ignore_ascii_case("FUNCTIONI");
            let name = self.advance()?.lexeme.clone();
            self.variadic.push(name.to_ascii_lowercase());
            self.skip_variadic_params();
            let return_type = if self.match_kind(TokenType::As) { Some(self.advance()?.lexeme.clone()) } else { None };
            return Some(DeclareStatement { span: self.span_from(start), is_function, name, lib: None, alias: None, params: Vec::new(), return_type });
        }
        let is_function = match self.peek_kind()? {
            TokenType::Function => {
                self.advance();
                true
            }
            _ => {
                self.advance(); // SUB
                false
            }
        };
        let mut name = self.expect(TokenType::Identifier)?.lexeme.clone();
        // RAPIDQ2.INC declares dotted names: `DECLARE SUB SLEEP.ms LIB …`.
        while self.peek_kind() == Some(TokenType::Dot) && self.peek_kind_at(1) == Some(TokenType::Identifier) {
            self.advance();
            name = format!("{name}.{}", self.advance()?.lexeme);
        }
        let lib = if self.peek_identifier_eq("LIB") {
            self.advance();
            Some(self.advance()?.lexeme.clone())
        } else {
            None
        };
        let alias = if self.peek_identifier_eq("ALIAS") {
            self.advance();
            Some(self.advance()?.lexeme.clone())
        } else {
            None
        };
        let params = if self.match_kind(TokenType::LParen) {
            let p = self.parse_parameter_list()?;
            self.expect(TokenType::RParen);
            p
        } else {
            Vec::new()
        };
        let return_type = if self.match_kind(TokenType::As) {
            Some(self.advance()?.lexeme.clone())
        } else {
            None
        };
        Some(DeclareStatement {
            span: self.span_from(start),
            is_function,
            name,
            lib,
            alias,
            params,
            return_type,
        })
    }

    fn parse_rust_block(&mut self) -> Option<RustBlockStatement> {
        let start = self.pos;
        let tok = self.advance()?; // RustStart token — lexeme contains the raw Rust body
        let code = tok.lexeme.clone();
        self.skip_newlines();
        Some(RustBlockStatement {
            span: self.span_from(start),
            code,
        })
    }

    fn parse_assignment_or_call(&mut self) -> Option<Statement> {
        let start = self.pos;
        let left = self.parse_postfix_expression()?;

        // RapidQ `i++` / `i--` and `x += y`, `-=`, `*=`, `/=`, `&=`.
        let compound = match (self.peek_kind(), self.peek_kind_at(1)) {
            (Some(TokenType::Plus), Some(TokenType::Plus)) => Some((BinaryOperator::Add, 2, false)),
            (Some(TokenType::Minus), Some(TokenType::Minus)) => Some((BinaryOperator::Subtract, 2, false)),
            (Some(TokenType::Plus), Some(TokenType::Eq)) => Some((BinaryOperator::Add, 2, true)),
            (Some(TokenType::Minus), Some(TokenType::Eq)) => Some((BinaryOperator::Subtract, 2, true)),
            (Some(TokenType::Star), Some(TokenType::Eq)) => Some((BinaryOperator::Multiply, 2, true)),
            (Some(TokenType::Slash), Some(TokenType::Eq)) => Some((BinaryOperator::Divide, 2, true)),
            (Some(TokenType::Ampersand), Some(TokenType::Eq)) => Some((BinaryOperator::Concat, 2, true)),
            _ => None,
        };
        if let Some((operator, width, has_value)) = compound {
            self.pos += width;
            let right = if has_value {
                self.parse_expression()?
            } else {
                Expression::Literal(Literal { span: self.span_from(start), value: LiteralValue::Integer(1) })
            };
            let span = self.span_from(start);
            return Some(Statement::Assignment(AssignmentStatement {
                span,
                target: left.clone(),
                value: Expression::Binary(BinaryExpression { span, left: Box::new(left), operator, right: Box::new(right) }),
            }));
        }

        if self.match_kind(TokenType::Eq) {
            let value = self.parse_expression()?;
            return Some(Statement::Assignment(AssignmentStatement {
                span: self.span_from(start),
                target: left,
                value,
            }));
        }

        // A statement also ends at `:` (`Foo : Bar`) and at the ELSE of a
        // single-line IF (`IF x THEN Foo ELSE Bar`).
        if self.at_eol() || matches!(self.peek_kind(), Some(TokenType::Colon | TokenType::Else)) {
            if let Some(args) = extract_existing_call_args(&left) {
                return Some(Statement::Call(CallStatement {
                    span: self.span_from(start),
                    callee: strip_inline_call_args(left),
                    args,
                }));
            }
            // Member access as a call (e.g. `obj.Show`)
            if matches!(left, Expression::MemberAccess(_) | Expression::Identifier(_)) {
                return Some(Statement::Call(CallStatement {
                    span: self.span_from(start),
                    callee: left,
                    args: vec![],
                }));
            }
            return None;
        }

        let args = self.parse_argument_list_without_parens()?;
        Some(Statement::Call(CallStatement {
            span: self.span_from(start),
            callee: left,
            args,
        }))
    }

    // -----------------------------------------------------------------------
    // Block statements
    // -----------------------------------------------------------------------

    fn parse_if(&mut self) -> Option<IfStatement> {
        let start = self.pos;
        self.expect(TokenType::If)?;
        let condition = self.parse_expression()?;
        self.expect(TokenType::Then)?;

        // Single-line IF: non-empty remainder after THEN on the same line
        if !self.at_eol() {
            let mut then_body = Vec::new();
            // Parse one or more colon-separated statements on the THEN line
            loop {
                if let Some(s) = self.parse_statement() {
                    then_body.push(s);
                    then_body.append(&mut self.pending);
                }
                if !self.match_kind(TokenType::Colon) {
                    break;
                }
            }
            let mut else_body = Vec::new();
            if self.match_kind(TokenType::Else) || self.peek_kind() == Some(TokenType::Else) {
                self.match_kind(TokenType::Else);
                loop {
                    if let Some(s) = self.parse_statement() {
                        else_body.push(s);
                        else_body.append(&mut self.pending);
                    }
                    if !self.match_kind(TokenType::Colon) {
                        break;
                    }
                }
            }
            return Some(IfStatement {
                span: self.span_from(start),
                condition,
                then_body,
                elseif_branches: Vec::new(),
                else_body,
            });
        }

        // Multi-line IF
        let terminators = &[
            Terminator::Keyword("ELSEIF"),
            Terminator::Keyword("ELSE"),
            Terminator::EndPair("IF"),
        ];
        let then_body = self.parse_body(terminators);

        let mut elseif_branches = Vec::new();
        while self.peek_kind() == Some(TokenType::ElseIf) {
            let ei_start = self.pos;
            self.advance(); // ELSEIF
            let ei_cond = self.parse_expression()?;
            self.expect(TokenType::Then);
            self.consume_eol();
            let ei_body = self.parse_body(terminators);
            elseif_branches.push(ElseIfBranch {
                span: self.span_from(ei_start),
                condition: ei_cond,
                body: ei_body,
            });
        }

        let else_body = if self.match_kind(TokenType::Else) {
            self.consume_eol();
            self.parse_body(&[Terminator::EndPair("IF")])
        } else {
            Vec::new()
        };

        // consume END IF
        self.expect(TokenType::End);
        self.expect(TokenType::If);

        Some(IfStatement {
            span: self.span_from(start),
            condition,
            then_body,
            elseif_branches,
            else_body,
        })
    }

    fn parse_for(&mut self) -> Option<ForStatement> {
        let start = self.pos;
        self.expect(TokenType::For)?;
        let variable = self.expect(TokenType::Identifier)?.lexeme.clone();
        self.expect(TokenType::Eq)?;
        let from = self.parse_expression()?;
        self.expect(TokenType::To)?;
        let to = self.parse_expression()?;
        let step = if self.match_kind(TokenType::Step) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.consume_eol();
        let body = self.parse_body(&[Terminator::Keyword("NEXT")]);
        // consume NEXT [var]
        self.expect(TokenType::Next);
        // optional variable name after NEXT
        if self.peek_kind() == Some(TokenType::Identifier) {
            self.advance();
        }
        Some(ForStatement {
            span: self.span_from(start),
            variable,
            start: from,
            end: to,
            step,
            body,
        })
    }

    /// `FOR obj.x = a TO b [STEP s] … NEXT` (RAPIDQ2.INC loops over
    /// `THIS.lx`): becomes `obj.x = a` and a WHILE that checks the bound in
    /// the step's direction and adds the step after the body. `EXIT FOR` in
    /// the body becomes `EXIT WHILE`.
    fn parse_for_member(&mut self) -> Option<Statement> {
        let start = self.pos;
        self.expect(TokenType::For)?;
        let target = self.parse_postfix_expression()?;
        self.expect(TokenType::Eq)?;
        let from = self.parse_expression()?;
        self.expect(TokenType::To)?;
        let to = self.parse_expression()?;
        let step = if self.match_kind(TokenType::Step) { Some(self.parse_expression()?) } else { None };
        self.consume_eol();
        let mut body = self.parse_body(&[Terminator::Keyword("NEXT")]);
        self.expect(TokenType::Next);
        if !self.at_eol() && self.peek_kind() != Some(TokenType::Colon) {
            self.parse_postfix_expression();
        }
        let span = self.span_from(start);
        self.for_counter += 1;
        let end_var = ident(span, &format!("__forend{}", self.for_counter));
        let step_var = ident(span, &format!("__forstep{}", self.for_counter));
        let int = |n: i64| Expression::Literal(Literal { span, value: LiteralValue::Integer(n) });
        let assign = |target: Expression, value: Expression| Statement::Assignment(AssignmentStatement { span, target, value });
        let bin = |l: Expression, op: BinaryOperator, r: Expression| Expression::Binary(BinaryExpression { span, left: Box::new(l), operator: op, right: Box::new(r) });
        let condition = bin(
            bin(
                bin(step_var.clone(), BinaryOperator::GreaterThanOrEqual, int(0)),
                BinaryOperator::And,
                bin(target.clone(), BinaryOperator::LessThanOrEqual, end_var.clone()),
            ),
            BinaryOperator::Or,
            bin(
                bin(step_var.clone(), BinaryOperator::LessThan, int(0)),
                BinaryOperator::And,
                bin(target.clone(), BinaryOperator::GreaterThanOrEqual, end_var.clone()),
            ),
        );
        retarget_exit_for(&mut body);
        body.push(assign(target.clone(), bin(target.clone(), BinaryOperator::Add, step_var.clone())));
        self.pending = vec![
            assign(end_var, to),
            assign(step_var, step.unwrap_or_else(|| int(1))),
            Statement::While(WhileStatement { span, condition, body }),
        ];
        Some(assign(target, from))
    }

    fn parse_while(&mut self) -> Option<WhileStatement> {
        let start = self.pos;
        self.expect(TokenType::While)?;
        let condition = self.parse_expression()?;
        self.consume_eol();
        let body = self.parse_body(&[Terminator::Keyword("WEND")]);
        self.expect(TokenType::Wend);
        Some(WhileStatement {
            span: self.span_from(start),
            condition,
            body,
        })
    }

    fn parse_do_loop(&mut self) -> Option<DoLoopStatement> {
        let start = self.pos;
        self.expect(TokenType::Do)?;

        // DO WHILE / DO UNTIL
        let (pre_condition, is_until, pre_cond) =
            if self.match_kind(TokenType::While) {
                (true, false, Some(self.parse_expression()?))
            } else if self.match_kind(TokenType::Until) {
                (true, true, Some(self.parse_expression()?))
            } else {
                (false, false, None)
            };

        self.consume_eol();
        let body = self.parse_body(&[Terminator::Keyword("LOOP")]);
        self.expect(TokenType::Loop);

        // LOOP WHILE / LOOP UNTIL (post-condition)
        let (condition, post_until) = if !pre_condition {
            if self.match_kind(TokenType::While) {
                (Some(self.parse_expression()?), false)
            } else if self.match_kind(TokenType::Until) {
                (Some(self.parse_expression()?), true)
            } else {
                (None, false)
            }
        } else {
            (pre_cond, is_until)
        };

        Some(DoLoopStatement {
            span: self.span_from(start),
            condition,
            pre_condition,
            is_until: if pre_condition { is_until } else { post_until },
            body,
        })
    }

    /// One `CASE` item: `expr`, `low TO high`, or `IS <op> expr`.
    fn parse_case_value(&mut self) -> Option<CaseValue> {
        let comparison = |k: Option<TokenType>| {
            matches!(k, Some(TokenType::Eq | TokenType::Neq | TokenType::Lt | TokenType::Lte | TokenType::Gt | TokenType::Gte))
        };
        // `CASE IS > 5`, or without IS: `CASE = fdOpen`, `CASE > 5`.
        let is_is = self.peek_identifier_eq("IS") && comparison(self.peek_kind_at(1));
        if is_is || comparison(self.peek_kind()) {
            if is_is {
                self.advance(); // IS
            }
            let op = match self.advance()?.kind {
                TokenType::Eq => BinaryOperator::Equal,
                TokenType::Neq => BinaryOperator::NotEqual,
                TokenType::Lt => BinaryOperator::LessThan,
                TokenType::Lte => BinaryOperator::LessThanOrEqual,
                TokenType::Gt => BinaryOperator::GreaterThan,
                _ => BinaryOperator::GreaterThanOrEqual,
            };
            return Some(CaseValue::Is(op, self.parse_expression()?));
        }
        let value = self.parse_expression()?;
        if self.match_kind(TokenType::To) {
            return Some(CaseValue::Range(value, self.parse_expression()?));
        }
        Some(CaseValue::Value(value))
    }

    fn parse_select_case(&mut self) -> Option<SelectCaseStatement> {
        let start = self.pos;
        self.expect(TokenType::Select)?;
        self.expect(TokenType::Case)?;
        let expression = self.parse_expression()?;
        self.consume_eol();

        let mut cases = Vec::new();
        let mut case_else = Vec::new();

        let terminators = &[
            Terminator::Keyword("CASE"),
            Terminator::EndPair("SELECT"),
        ];

        loop {
            self.skip_newlines();
            if self.peek_is_end_followed_by("SELECT") || self.at_end() {
                break;
            }
            if self.peek_kind() != Some(TokenType::Case) {
                break;
            }
            let case_start = self.pos;
            self.advance(); // CASE

            // CASE ELSE
            if self.match_kind(TokenType::Else) {
                // `CASE ELSE : stmt` — the body may start on the same line.
                self.match_kind(TokenType::Colon);
                self.consume_eol();
                case_else = self.parse_body(&[
                    Terminator::Keyword("CASE"),
                    Terminator::EndPair("SELECT"),
                ]);
                continue;
            }

            // CASE value1, low TO high, IS > n
            let mut values = vec![self.parse_case_value()?];
            while self.match_kind(TokenType::Comma) {
                values.push(self.parse_case_value()?);
            }
            // `CASE 1: PRINT "one"` — the body may start on the same line.
            let same_line = self.match_kind(TokenType::Colon);
            if !same_line && !self.at_eol() {
                let message = format!("Unexpected '{}' in CASE list", self.tokens[self.pos].lexeme);
                self.error_at(self.pos, message);
                self.skip_to_eol();
            }
            self.consume_eol();
            let body = self.parse_body(terminators);
            cases.push(CaseBranch {
                span: self.span_from(case_start),
                values,
                body,
            });
        }

        self.expect(TokenType::End);
        self.expect(TokenType::Select);

        Some(SelectCaseStatement {
            span: self.span_from(start),
            expression,
            cases,
            case_else,
        })
    }

    /// A SUB/FUNCTION name; RAPIDQ2.INC adds methods to built-in objects
    /// with dotted names (`FUNCTION Screen.MousePresent() AS LONG`), called
    /// as `Screen.MousePresent`.
    fn parse_routine_name(&mut self) -> Option<String> {
        let mut name = self.expect(TokenType::Identifier)?.lexeme.clone();
        while self.peek_kind() == Some(TokenType::Dot) && self.tokens.get(self.pos + 1).is_some_and(|t| t.kind != TokenType::Newline) {
            self.advance();
            name = format!("{name}.{}", self.advance()?.lexeme);
        }
        Some(name)
    }

    fn parse_sub(&mut self) -> Option<SubroutineStatement> {
        let start = self.pos;
        self.expect(TokenType::Sub)?;
        let name = self.parse_routine_name()?;
        let params = if self.match_kind(TokenType::LParen) {
            let p = self.parse_parameter_list()?;
            self.expect(TokenType::RParen);
            p
        } else {
            Vec::new()
        };
        self.consume_eol();
        // RapidQ accepted a SUB closed by END FUNCTION and vice versa (RAPIDQ2.INC does it).
        let body = self.parse_body(&[Terminator::EndPair("SUB"), Terminator::EndPair("FUNCTION")]);
        self.expect(TokenType::End);
        if !self.match_kind(TokenType::Sub) {
            self.expect(TokenType::Function);
        }
        Some(SubroutineStatement {
            span: self.span_from(start),
            name,
            params,
            body,
        })
    }

    fn parse_function(&mut self) -> Option<FunctionStatement> {
        let start = self.pos;
        self.expect(TokenType::Function)?;
        let name = self.parse_routine_name()?;
        let params = if self.match_kind(TokenType::LParen) {
            let p = self.parse_parameter_list()?;
            self.expect(TokenType::RParen);
            p
        } else {
            Vec::new()
        };
        let return_type = if self.match_kind(TokenType::As) {
            Some(self.advance()?.lexeme.clone())
        } else {
            None
        };
        self.consume_eol();
        let body = self.parse_body(&[Terminator::EndPair("FUNCTION"), Terminator::EndPair("SUB")]);
        self.expect(TokenType::End);
        if !self.match_kind(TokenType::Function) {
            self.expect(TokenType::Sub);
        }
        Some(FunctionStatement {
            span: self.span_from(start),
            name,
            params,
            return_type,
            body,
        })
    }

    fn parse_type_def(&mut self) -> Option<TypeStatement> {
        let start = self.pos;
        self.expect(TokenType::Type)?;
        let name = self.expect(TokenType::Identifier)?.lexeme.clone();
        // `TYPE X EXTENDS QFORM` or the manual's `TYPE X AS QFORM`. QOBJECT is
        // RapidQ's empty base object: a plain TYPE with methods.
        let extends = if self.match_kind(TokenType::Extends) || self.match_kind(TokenType::As) {
            Some(canonical_type_name(&self.expect(TokenType::Identifier)?.lexeme))
                .filter(|base| !base.eq_ignore_ascii_case("QOBJECT") && !base.eq_ignore_ascii_case("ROBJECT"))
        } else {
            None
        };
        self.consume_eol();

        let mut fields = Vec::new();
        let mut methods = Vec::new();
        let mut constructor = Vec::new();
        let mut events = Vec::new();

        loop {
            self.skip_newlines();
            if self.peek_is_end_followed_by("TYPE") || self.at_end() {
                break;
            }
            // `WITH TypeName` … `END WITH` around the members (RAPIDQ2.INC):
            // `.Member` in the methods means the instance anyway.
            if self.peek_kind() == Some(TokenType::With) {
                self.skip_to_eol();
                continue;
            }
            if self.peek_is_end_followed_by("WITH") {
                self.advance();
                self.advance();
                continue;
            }
            // PRIVATE: / PUBLIC: / PROTECTED: sections (access isn't enforced)
            if self.peek_identifier_eq("PRIVATE") || self.peek_identifier_eq("PUBLIC") || self.peek_identifier_eq("PROTECTED") {
                self.advance();
                self.match_kind(TokenType::Colon);
                continue;
            }
            // CONSTRUCTOR block
            if self.peek_identifier_eq("CONSTRUCTOR") {
                self.advance();
                self.consume_eol();
                constructor = self.parse_body(&[Terminator::EndPair("CONSTRUCTOR")]);
                self.expect(TokenType::End);
                // consume CONSTRUCTOR
                self.advance();
                continue;
            }
            // EVENT OnClick [(params)] / EVENT(OnClick) … END EVENT
            if self.peek_identifier_eq("EVENT")
                && matches!(self.peek_kind_at(1), Some(TokenType::Identifier | TokenType::LParen))
            {
                let event_start = self.pos;
                self.advance(); // EVENT
                let wrapped = self.match_kind(TokenType::LParen);
                let Some(name_tok) = self.expect(TokenType::Identifier) else {
                    self.error_at(self.pos, "EVENT needs an event name, e.g. EVENT OnClick".into());
                    self.skip_to_eol();
                    continue;
                };
                let mut name = name_tok.lexeme.clone();
                // `EVENT Panel.OnClick`: an event of a component field.
                while self.peek_kind() == Some(TokenType::Dot) {
                    self.advance();
                    let Some(part) = self.advance() else { break };
                    name = format!("{name}.{}", part.lexeme);
                }
                if wrapped {
                    self.expect(TokenType::RParen);
                }
                let params = if !wrapped && self.match_kind(TokenType::LParen) {
                    let p = self.parse_parameter_list().unwrap_or_default();
                    self.expect(TokenType::RParen);
                    p
                } else {
                    Vec::new()
                };
                self.consume_eol();
                let body = self.parse_body(&[Terminator::EndPair("EVENT")]);
                if self.peek_is_end_followed_by("EVENT") {
                    self.advance();
                    self.advance();
                } else {
                    self.error_at(event_start, format!("EVENT {name} is missing END EVENT"));
                }
                events.push(TypeEvent { span: self.span_from(event_start), name, params, body });
                continue;
            }
            // SUBI / FUNCTIONI methods (any number of arguments; RAPIDQ2.INC
            // names one `PRINT`)
            if (self.peek_identifier_eq("SUBI") || self.peek_identifier_eq("FUNCTIONI"))
                && self.tokens.get(self.pos + 1).is_some_and(|t| t.lexeme.chars().all(|c| c.is_alphanumeric() || c == '_'))
            {
                if let Some(m) = self.parse_variadic_routine() {
                    methods.push(m);
                }
                continue;
            }
            // Methods
            if self.peek_kind() == Some(TokenType::Sub) {
                if let Some(s) = self.parse_sub() {
                    methods.push(Statement::Subroutine(s));
                    continue;
                }
            }
            if self.peek_kind() == Some(TokenType::Function) {
                if let Some(f) = self.parse_function() {
                    methods.push(Statement::Function(f));
                    continue;
                }
            }
            // `PROPERTY SET Name (v AS T) … END PROPERTY`: the setter method.
            if self.peek_kind() == Some(TokenType::Property) && self.peek_kind_at(1) == Some(TokenType::Set) {
                let prop_start = self.pos;
                self.advance();
                self.advance();
                let Some(name_tok) = self.expect(TokenType::Identifier) else {
                    self.skip_to_eol();
                    continue;
                };
                let setter_name = name_tok.lexeme.clone();
                let params = if self.match_kind(TokenType::LParen) {
                    let p = self.parse_parameter_list().unwrap_or_default();
                    self.expect(TokenType::RParen);
                    p
                } else {
                    Vec::new()
                };
                self.consume_eol();
                let body = self.parse_body(&[Terminator::EndPair("PROPERTY")]);
                if self.peek_is_end_followed_by("PROPERTY") {
                    self.advance();
                    self.advance();
                } else {
                    self.error_at(prop_start, format!("PROPERTY SET {setter_name} is missing END PROPERTY"));
                }
                methods.push(Statement::Subroutine(SubroutineStatement {
                    span: self.span_from(prop_start),
                    name: setter_name,
                    params,
                    body,
                }));
                continue;
            }
            if self.peek_kind() == Some(TokenType::Property) {
                self.error_at(self.pos, "Only PROPERTY SET blocks are supported inside a TYPE".into());
                self.skip_to_eol();
                continue;
            }
            // `DECLARE SUB Name …` inside a TYPE only announces a method.
            if self.peek_kind() == Some(TokenType::Declare) {
                self.skip_to_eol();
                continue;
            }
            // Field: name[(dims)] AS Type [* len] [PROPERTY SET Setter]
            if self.peek_kind() == Some(TokenType::Identifier) {
                let field_start = self.pos;
                let fname = self.advance()?.lexeme.clone();
                let (arr, arr_lower) = if self.match_kind(TokenType::LParen) {
                    // A one-dimensional array field: `Names(2)` or `Colors(1 TO 16)`.
                    let dims = self.parse_array_dimensions();
                    self.expect(TokenType::RParen);
                    match dims.as_deref() {
                        Some([ArrayDimension::Single(upper)]) => (Some(upper.clone()), None),
                        Some([ArrayDimension::Range { start, end }]) => {
                            (Some(end.clone()), (!is_zero_literal(start)).then(|| start.clone()))
                        }
                        _ => {
                            self.error_at(field_start, format!("Array field {fname} in TYPE {name}: only one dimension is supported yet"));
                            self.skip_to_eol();
                            continue;
                        }
                    }
                } else {
                    (None, None)
                };
                if !self.match_kind(TokenType::As) {
                    let message = format!("Field {fname} in TYPE {name} needs a type: `{fname} AS INTEGER`");
                    self.error_at(self.pos, message);
                    self.skip_to_eol();
                    continue;
                }
                let Some(type_tok) = self.advance() else { break };
                let mut ftype = canonical_type_name(&type_tok.lexeme);
                // `OnReady AS EVENT(Template)`: a custom event (holds a SUB).
                if ftype.eq_ignore_ascii_case("EVENT") && self.match_kind(TokenType::LParen) {
                    while !self.at_eol() && !self.match_kind(TokenType::RParen) {
                        self.pos += 1;
                    }
                    ftype = "EVENT".to_string();
                }
                // `Name AS STRING * 20`: fixed-length string, stored as STRING.
                if self.match_kind(TokenType::Star) {
                    self.parse_unary();
                }
                let setter = if self.peek_kind() == Some(TokenType::Property) && self.peek_kind_at(1) == Some(TokenType::Set) {
                    self.advance();
                    self.advance();
                    self.expect(TokenType::Identifier).map(|t| t.lexeme.clone())
                } else {
                    None
                };
                fields.push(TypeField {
                    span: self.span_from(field_start),
                    name: fname,
                    type_name: ftype,
                    array_size: arr,
                    array_lower: arr_lower,
                    setter,
                });
                if !self.at_eol() {
                    let tok = &self.tokens[self.pos];
                    let message = format!("Unexpected '{}' after field declaration", tok.lexeme);
                    self.error_at(self.pos, message);
                    self.skip_to_eol();
                }
                self.consume_eol();
                continue;
            }
            // Anything else inside TYPE is an error, not silently dropped.
            let tok = self.peek()?;
            let message = format!("Unexpected '{}' inside TYPE {name} (expected a field, SUB, FUNCTION, EVENT or CONSTRUCTOR)", tok.lexeme);
            self.error_at(self.pos, message);
            self.skip_to_eol();
            self.consume_eol();
        }

        self.expect(TokenType::End);
        self.expect(TokenType::Type);

        // `.Member` outside any WITH inside the TYPE's code is the instance
        // (the RapidQ manual writes `WITH TForm … END WITH`; RAPIDQ2.INC wraps
        // whole TYPEs in `WITH QFormEx`).
        let this = Expression::Identifier(Identifier { span: TextSpan::default(), name: name.clone() });
        for body in methods.iter_mut().filter_map(|m| match m {
            Statement::Subroutine(s) => Some(&mut s.body),
            Statement::Function(f) => Some(&mut f.body),
            _ => None,
        }) {
            *body = resolve_with_body(body, &this);
        }
        constructor = resolve_with_body(&constructor, &this);
        for e in &mut events {
            e.body = resolve_with_body(&e.body, &this);
        }

        Some(TypeStatement {
            span: self.span_from(start),
            name,
            extends,
            fields,
            methods,
            constructor,
            events,
        })
    }

    fn parse_create(&mut self) -> Option<CreateStatement> {
        let start = self.pos;
        self.expect(TokenType::Create)?;
        let name = self.expect(TokenType::Identifier)?.lexeme.clone();
        self.expect(TokenType::As)?;
        let type_name = canonical_type_name(&self.expect(TokenType::Identifier)?.lexeme);
        self.consume_eol();
        let body = self.parse_body(&[Terminator::EndPair("CREATE")]);
        self.expect(TokenType::End);
        self.expect(TokenType::Create);
        Some(CreateStatement {
            span: self.span_from(start),
            name,
            type_name,
            body,
        })
    }

    fn parse_with(&mut self) -> Option<WithStatement> {
        let start = self.pos;
        self.expect(TokenType::With)?;
        let object = self.parse_expression()?;
        self.consume_eol();
        let body = self.parse_body(&[Terminator::EndPair("WITH")]);
        self.expect(TokenType::End);
        self.expect(TokenType::With);
        Some(WithStatement {
            span: self.span_from(start),
            object,
            body,
        })
    }

    // -----------------------------------------------------------------------
    // Parameters
    // -----------------------------------------------------------------------

    fn parse_parameter_list(&mut self) -> Option<Vec<Parameter>> {
        let mut params = Vec::new();
        if self.peek_kind() == Some(TokenType::RParen) {
            return Some(params);
        }
        loop {
            let p_start = self.pos;
            let by_ref = if self.match_kind(TokenType::ByVal) {
                false
            } else if self.match_kind(TokenType::ByRef) {
                true
            } else {
                false
            };
            // A keyword may name a parameter (`(hwnd AS LONG, type AS LONG)`
            // in RAPIDQ2.INC's API declarations).
            let pname = match self.peek() {
                Some(t) if t.kind == TokenType::Identifier => self.advance()?.lexeme.clone(),
                Some(t) if t.lexeme.chars().all(|c| c.is_ascii_alphabetic())
                    && self.peek_kind_at(1) == Some(TokenType::As) =>
                {
                    self.advance()?.lexeme.clone()
                }
                _ => self.expect(TokenType::Identifier)?.lexeme.clone(),
            };
            // `list() AS STRING`: an array parameter (arrays are shared, so
            // the SUB works on the caller's array).
            let is_array = self.peek_kind() == Some(TokenType::LParen) && self.peek_kind_at(1) == Some(TokenType::RParen);
            if is_array {
                self.pos += 2;
            }
            let ptype = if self.match_kind(TokenType::As) {
                canonical_type_name(&self.advance()?.lexeme)
            } else {
                "VARIANT".to_string()
            };
            params.push(Parameter {
                span: self.span_from(p_start),
                name: pname,
                type_name: ptype,
                by_ref,
                is_array,
            });
            if !self.match_kind(TokenType::Comma) {
                break;
            }
        }
        Some(params)
    }

    // -----------------------------------------------------------------------
    // Expressions
    // -----------------------------------------------------------------------

    fn parse_expression(&mut self) -> Option<Expression> {
        self.parse_logical_or()
    }

    fn parse_logical_or(&mut self) -> Option<Expression> {
        let mut expr = self.parse_logical_and()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenType::Or) => BinaryOperator::Or,
                Some(TokenType::Xor) => BinaryOperator::Xor,
                _ => break,
            };
            self.advance();
            let right = self.parse_logical_and()?;
            expr = binary(expr, op, right);
        }
        Some(expr)
    }

    fn parse_logical_and(&mut self) -> Option<Expression> {
        let mut expr = self.parse_not()?;
        while self.match_kind(TokenType::And) {
            let right = self.parse_not()?;
            expr = binary(expr, BinaryOperator::And, right);
        }
        Some(expr)
    }

    /// NOT binds looser than comparisons (RapidQ manual, operator
    /// precedence): `NOT x = 5` is `NOT (x = 5)`.
    fn parse_not(&mut self) -> Option<Expression> {
        if self.peek_kind() == Some(TokenType::Not) {
            let tok = self.advance()?;
            let operand = self.parse_not()?;
            return Some(Expression::Unary(UnaryExpression {
                span: TextSpan::new(tok.span.start, expression_span(&operand).end),
                operator: UnaryOperator::Not,
                operand: Box::new(operand),
            }));
        }
        self.parse_equality()
    }

    fn parse_equality(&mut self) -> Option<Expression> {
        let mut expr = self.parse_comparison()?;
        loop {
            let op = match (self.peek_kind(), self.peek_kind_at(1)) {
                (Some(TokenType::Eq), _) => BinaryOperator::Equal,
                (Some(TokenType::Neq), _) => BinaryOperator::NotEqual,
                // `a NOT= b`, accepted by RapidQ as `a <> b`
                (Some(TokenType::Not), Some(TokenType::Eq)) => {
                    self.advance();
                    BinaryOperator::NotEqual
                }
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison()?;
            expr = binary(expr, op, right);
        }
        Some(expr)
    }

    fn parse_comparison(&mut self) -> Option<Expression> {
        let mut expr = self.parse_term()?;
        loop {
            let op = match (self.peek_kind(), self.peek_kind_at(1)) {
                // `a < = b` / `a > = b` / `a < > b`, written with a space
                (Some(TokenType::Lt), Some(TokenType::Eq)) => {
                    self.advance();
                    BinaryOperator::LessThanOrEqual
                }
                (Some(TokenType::Gt), Some(TokenType::Eq)) => {
                    self.advance();
                    BinaryOperator::GreaterThanOrEqual
                }
                (Some(TokenType::Lt), Some(TokenType::Gt)) => {
                    self.advance();
                    BinaryOperator::NotEqual
                }
                (Some(TokenType::Lt), _) => BinaryOperator::LessThan,
                (Some(TokenType::Lte), _) => BinaryOperator::LessThanOrEqual,
                (Some(TokenType::Gt), _) => BinaryOperator::GreaterThan,
                (Some(TokenType::Gte), _) => BinaryOperator::GreaterThanOrEqual,
                _ => break,
            };
            self.advance();
            let right = self.parse_term()?;
            expr = binary(expr, op, right);
        }
        Some(expr)
    }

    fn parse_term(&mut self) -> Option<Expression> {
        let mut expr = self.parse_modulo()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenType::Plus) => BinaryOperator::Add,
                Some(TokenType::Minus) => BinaryOperator::Subtract,
                Some(TokenType::Ampersand) => BinaryOperator::Concat,
                _ => break,
            };
            self.advance();
            let right = self.parse_modulo()?;
            expr = binary(expr, op, right);
        }
        Some(expr)
    }

    /// MOD binds looser than `*` and `/` (RapidQ manual): `a MOD b * c` is
    /// `a MOD (b * c)`.
    fn parse_modulo(&mut self) -> Option<Expression> {
        let mut expr = self.parse_factor()?;
        loop {
            if self.match_kind(TokenType::Mod) {
                let right = self.parse_factor()?;
                expr = binary(expr, BinaryOperator::Modulo, right);
            } else if self.peek_identifier_eq("INV") {
                // `3 INV 26`: modular inverse (same precedence as MOD).
                self.advance();
                let right = self.parse_factor()?;
                let span = TextSpan::new(expression_span(&expr).start, expression_span(&right).end);
                expr = call_expr(span, "INV", vec![expr, right]);
            } else {
                break;
            }
        }
        Some(expr)
    }

    fn parse_factor(&mut self) -> Option<Expression> {
        let mut expr = self.parse_power()?;
        loop {
            // `a SHL n` / `a SHR n` (same precedence as * and /).
            if self.peek_identifier_eq("SHL") || self.peek_identifier_eq("SHR") {
                let tok = self.advance()?;
                let name = tok.lexeme.to_ascii_uppercase();
                let right = self.parse_power()?;
                let span = TextSpan::new(expression_span(&expr).start, expression_span(&right).end);
                expr = call_expr(span, &name, vec![expr, right]);
                continue;
            }
            let op = match self.peek_kind() {
                Some(TokenType::Star) => BinaryOperator::Multiply,
                Some(TokenType::Slash) => BinaryOperator::Divide,
                Some(TokenType::Backslash) => BinaryOperator::IntegerDivide,
                _ => break,
            };
            self.advance();
            let right = self.parse_power()?;
            expr = binary(expr, op, right);
        }
        Some(expr)
    }

    fn parse_power(&mut self) -> Option<Expression> {
        let mut expr = self.parse_unary()?;
        while self.match_kind(TokenType::Caret) {
            let right = self.parse_unary()?;
            expr = binary(expr, BinaryOperator::Power, right);
        }
        Some(expr)
    }

    fn parse_unary(&mut self) -> Option<Expression> {
        let op = match self.peek_kind() {
            Some(TokenType::Minus) => Some(UnaryOperator::Negate),
            Some(TokenType::Plus) => Some(UnaryOperator::Positive),
            Some(TokenType::Not) => Some(UnaryOperator::Not),
            Some(TokenType::At) => Some(UnaryOperator::Ref),
            _ => None,
        };
        if let Some(op) = op {
            let tok = self.advance()?;
            let operand = self.parse_unary()?;
            return Some(Expression::Unary(UnaryExpression {
                span: TextSpan::new(tok.span.start, expression_span(&operand).end),
                operator: op,
                operand: Box::new(operand),
            }));
        }
        self.parse_postfix_expression()
    }

    fn parse_postfix_expression(&mut self) -> Option<Expression> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.match_kind(TokenType::LParen) {
                let args = self.parse_argument_list_in_parens()?;
                expr = Expression::FunctionCall(FunctionCallExpression {
                    span: TextSpan::new(
                        expression_span(&expr).start,
                        self.previous()?.span.end,
                    ),
                    callee: Box::new(expr),
                    args,
                });
                continue;
            }
            if self.match_kind(TokenType::LBracket) {
                // RapidQ string index: `s$[i]` is the i-th character, MID$(s$, i, 1).
                let index = self.parse_expression()?;
                self.expect(TokenType::RBracket)?;
                let span = TextSpan::new(expression_span(&expr).start, self.previous()?.span.end);
                let one = Expression::Literal(Literal { span, value: LiteralValue::Integer(1) });
                expr = Expression::FunctionCall(FunctionCallExpression {
                    span,
                    callee: Box::new(Expression::Identifier(Identifier { span, name: "MID$".to_string() })),
                    args: vec![expr, index, one],
                });
                continue;
            }
            if self.match_kind(TokenType::Dot) {
                // Accept any token as member name (keywords like Close, Show, Open, Clear are valid method/property names)
                let member = self.advance()?;
                expr = Expression::MemberAccess(MemberAccessExpression {
                    span: TextSpan::new(expression_span(&expr).start, member.span.end),
                    object: Box::new(expr),
                    member: member.lexeme.clone(),
                });
                continue;
            }
            break;
        }
        Some(expr)
    }

    fn parse_argument_list_in_parens(&mut self) -> Option<Vec<Expression>> {
        let mut args = Vec::new();
        if self.match_kind(TokenType::RParen) {
            return Some(args);
        }
        loop {
            args.push(self.parse_argument()?);
            if self.match_kind(TokenType::Comma) {
                continue;
            }
            self.expect(TokenType::RParen)?;
            break;
        }
        Some(args)
    }

    /// One argument, which RapidQ lets you leave out: `INSTR(, a, b)`,
    /// `COLOR , 1`, `LOCATE , 5` (the callee then uses its default).
    fn parse_argument(&mut self) -> Option<Expression> {
        if matches!(self.peek_kind(), Some(TokenType::Comma | TokenType::RParen)) {
            let span = self.peek().map(|t| t.span).unwrap_or_default();
            return Some(Expression::Identifier(Identifier { span, name: OMITTED_ARGUMENT.to_string() }));
        }
        self.parse_expression()
    }

    fn parse_argument_list_without_parens(&mut self) -> Option<Vec<Expression>> {
        let mut args = vec![self.parse_argument()?];
        while self.match_kind(TokenType::Comma) {
            args.push(self.parse_argument()?);
        }
        Some(args)
    }

    fn parse_primary(&mut self) -> Option<Expression> {
        match self.peek_kind()? {
            // A type name as a value (`SIZEOF(SINGLE)`): its name as text.
            TokenType::Integer | TokenType::String | TokenType::Double | TokenType::Single | TokenType::Byte
            | TokenType::Word | TokenType::Dword | TokenType::Long | TokenType::Int64 | TokenType::Currency
            | TokenType::Variant => {
                let tok = self.advance()?;
                Some(Expression::Literal(Literal { span: tok.span, value: LiteralValue::String(tok.lexeme.to_ascii_uppercase()) }))
            }
            // WITH-dot access: `.Property`
            TokenType::Dot => {
                let dot_tok = self.advance()?;
                // Accept any token as member name (keywords like Close, Show, Clear are valid after dot)
                let member = self.advance()?;
                Some(Expression::MemberAccess(MemberAccessExpression {
                    span: TextSpan::new(dot_tok.span.start, member.span.end),
                    object: Box::new(Expression::Identifier(Identifier {
                        span: dot_tok.span,
                        name: "_with_".to_string(),
                    })),
                    member: member.lexeme.clone(),
                }))
            }
            TokenType::Identifier => {
                let tok = self.advance()?;
                // Handle compound identifiers like LINE INPUT → line_input
                if tok.lexeme.eq_ignore_ascii_case("LINE")
                    && self.peek_kind() == Some(TokenType::Input)
                {
                    let input_tok = self.advance()?;
                    return Some(Expression::Identifier(Identifier {
                        span: TextSpan::new(tok.span.start, input_tok.span.end),
                        name: "LINE_INPUT".to_string(),
                    }));
                }
                Some(Expression::Identifier(Identifier {
                    span: tok.span,
                    name: tok.lexeme.clone(),
                }))
            }
            TokenType::Number => {
                let tok = self.advance()?;
                Some(Expression::Literal(Literal {
                    span: tok.span,
                    value: parse_number_literal(&tok.lexeme),
                }))
            }
            TokenType::StringLit => {
                let tok = self.advance()?;
                Some(Expression::Literal(Literal {
                    span: tok.span,
                    value: LiteralValue::String(tok.lexeme.clone()),
                }))
            }
            TokenType::LParen => {
                self.advance()?;
                let expr = self.parse_expression()?;
                self.expect(TokenType::RParen)?;
                Some(expr)
            }
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Terminators used by parse_body
// ---------------------------------------------------------------------------

enum Terminator {
    /// Simple keyword match (case-insensitive), e.g. `NEXT`, `WEND`, `LOOP`.
    Keyword(&'static str),
    /// `END <keyword>` pair, e.g. `END IF`, `END SUB`.
    EndPair(&'static str),
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_number_literal(lexeme: &str) -> LiteralValue {
    if let Some(hex) = lexeme.strip_prefix("0x") {
        i64::from_str_radix(hex, 16)
            .map(LiteralValue::Integer)
            .unwrap_or_else(|_| LiteralValue::String(lexeme.to_string()))
    } else if let Some(oct) = lexeme.strip_prefix("0o") {
        i64::from_str_radix(oct, 8)
            .map(LiteralValue::Integer)
            .unwrap_or_else(|_| LiteralValue::String(lexeme.to_string()))
    } else if let Some(bin) = lexeme.strip_prefix("0b") {
        i64::from_str_radix(bin, 2)
            .map(LiteralValue::Integer)
            .unwrap_or_else(|_| LiteralValue::String(lexeme.to_string()))
    } else if lexeme.contains('.') || lexeme.contains('e') || lexeme.contains('E') {
        lexeme
            .parse::<f64>()
            .map(LiteralValue::Float)
            .unwrap_or_else(|_| LiteralValue::String(lexeme.to_string()))
    } else {
        lexeme
            .parse::<i64>()
            .map(LiteralValue::Integer)
            .unwrap_or_else(|_| LiteralValue::String(lexeme.to_string()))
    }
}

fn binary(left: Expression, operator: BinaryOperator, right: Expression) -> Expression {
    Expression::Binary(BinaryExpression {
        span: TextSpan::new(
            expression_span(&left).start,
            expression_span(&right).end,
        ),
        left: Box::new(left),
        operator,
        right: Box::new(right),
    })
}

pub fn expression_span(expression: &Expression) -> TextSpan {
    match expression {
        Expression::ArrayAccess(n) => n.span,
        Expression::Binary(n) => n.span,
        Expression::FunctionCall(n) => n.span,
        Expression::Identifier(n) => n.span,
        Expression::Literal(n) => n.span,
        Expression::MemberAccess(n) => n.span,
        Expression::MethodCall(n) => n.span,
        Expression::Unary(n) => n.span,
    }
}

pub fn statement_span(statement: &Statement) -> TextSpan {
    match statement {
        Statement::Assignment(n) => n.span,
        Statement::Bind(n) => n.span,
        Statement::Call(n) => n.span,
        Statement::Comment(n) => n.span,
        Statement::Const(n) => n.span,
        Statement::Create(n) => n.span,
        Statement::Declare(n) => n.span,
        Statement::Dim(n) => n.span,
        Statement::Directive(n) => n.span,
        Statement::DoLoop(n) => n.span,
        Statement::Exit(n) => n.span,
        Statement::For(n) => n.span,
        Statement::Function(n) => n.span,
        Statement::If(n) => n.span,
        Statement::Import(n) => n.span,
        Statement::Input(n) => n.span,
        Statement::Label(n) => n.span,
        Statement::Goto(n) | Statement::Gosub(n) => n.span,
        Statement::Line(n) => n.span,
        Statement::Open(n) => n.span,
        Statement::Close(n) => n.span,
        Statement::Print(n) => n.span,
        Statement::PrintHash(n) => n.span,
        Statement::Return(n) => n.span,
        Statement::Seek(n) => n.span,
        Statement::SelectCase(n) => n.span,
        Statement::Subroutine(n) => n.span,
        Statement::Type(n) => n.span,
        Statement::While(n) => n.span,
        Statement::With(n) => n.span,
        Statement::WriteHash(n) => n.span,
        Statement::RustBlock(n) => n.span,
    }
}

fn extract_existing_call_args(expression: &Expression) -> Option<Vec<Expression>> {
    match expression {
        Expression::FunctionCall(node) => Some(node.args.clone()),
        _ => None,
    }
}

fn strip_inline_call_args(expression: Expression) -> Expression {
    match expression {
        Expression::FunctionCall(node) => *node.callee,
        other => other,
    }
}

/// `EXIT FOR` → `EXIT WHILE` in a rewritten FOR body, outside inner loops
/// (whose EXITs are their own).
fn retarget_exit_for(body: &mut [Statement]) {
    for stmt in body {
        match stmt {
            Statement::Exit(e) if e.exit_type.eq_ignore_ascii_case("FOR") => e.exit_type = "WHILE".to_string(),
            Statement::If(i) => {
                retarget_exit_for(&mut i.then_body);
                for b in &mut i.elseif_branches {
                    retarget_exit_for(&mut b.body);
                }
                retarget_exit_for(&mut i.else_body);
            }
            Statement::SelectCase(s) => {
                for c in &mut s.cases {
                    retarget_exit_for(&mut c.body);
                }
                retarget_exit_for(&mut s.case_else);
            }
            Statement::With(w) => retarget_exit_for(&mut w.body),
            _ => {}
        }
    }
}

/// Statements registering the program's DATA table when it starts: a
/// reset, the items (in calls of at most 200 arguments) and RESTORE labels.
fn data_table_init(items: &[Expression], labels: &[(String, usize)]) -> Vec<Statement> {
    let span = TextSpan::default();
    let call = |name: &str, args: Vec<Expression>| Statement::Call(CallStatement { span, callee: ident(span, name), args });
    let mut out = vec![call("__DATA_RESET", Vec::new())];
    for chunk in items.chunks(200) {
        out.push(call("__DATA_ADD", chunk.to_vec()));
    }
    for (label, index) in labels {
        out.push(call(
            "__DATA_LABEL",
            vec![
                Expression::Literal(Literal { span, value: LiteralValue::String(label.clone()) }),
                Expression::Literal(Literal { span, value: LiteralValue::Integer(*index as i64) }),
            ],
        ));
    }
    out
}

/// The hidden parameter of a SUBI/FUNCTIONI holding its packed arguments.
const VARIADIC_PARAM: &str = "__params";

fn ident(span: TextSpan, name: &str) -> Expression {
    Expression::Identifier(Identifier { span, name: name.to_string() })
}

fn call_expr(span: TextSpan, name: &str, args: Vec<Expression>) -> Expression {
    Expression::FunctionCall(FunctionCallExpression { span, callee: Box::new(ident(span, name)), args })
}

/// Inside a SUBI/FUNCTIONI: ParamStr$(i) → __PARAMSTR(__params, i), ParamVal(i),
/// and the ParamStrCount / ParamValCount counts.
fn rewrite_param_access(body: &mut [Statement]) {
    walk_expressions_mut(body, true, &mut |e| {
        let replacement = match e {
            Expression::FunctionCall(c) => match c.callee.as_ref() {
                Expression::Identifier(id) if c.args.len() == 1 => {
                    let key = id.name.trim_end_matches('$').to_ascii_uppercase();
                    matches!(key.as_str(), "PARAMSTR" | "PARAMVAL").then(|| {
                        call_expr(c.span, &format!("__{key}"), vec![ident(c.span, VARIADIC_PARAM), c.args[0].clone()])
                    })
                }
                _ => None,
            },
            Expression::Identifier(id) => {
                let key = id.name.to_ascii_uppercase();
                matches!(key.as_str(), "PARAMSTRCOUNT" | "PARAMVALCOUNT")
                    .then(|| call_expr(id.span, &format!("__{key}"), vec![ident(id.span, VARIADIC_PARAM)]))
            }
            _ => None,
        };
        if let Some(r) = replacement {
            *e = r;
        }
    });
}

/// Every call to a SUBI/FUNCTIONI passes its arguments packed into one
/// value: `Max 1, 2, 3` → `Max __PACK(1, 2, 3)`.
fn pack_variadic_calls(stmts: &mut [Statement], names: &[String]) {
    if names.is_empty() {
        return;
    }
    // `Max 1, 2` or, for a SUBI/FUNCTIONI method, `obj.Max 1, 2`.
    let is_variadic = |callee: &Expression| match callee {
        Expression::Identifier(id) => names.contains(&id.name.to_ascii_lowercase()),
        Expression::MemberAccess(m) => names.contains(&m.member.to_ascii_lowercase()),
        _ => false,
    };
    let pack = |span: TextSpan, args: &mut Vec<Expression>| {
        let packed = call_expr(span, "__PACK", std::mem::take(args));
        args.push(packed);
    };
    walk_expressions_mut(stmts, true, &mut |e| {
        if let Expression::FunctionCall(c) = e {
            if is_variadic(&c.callee) {
                pack(c.span, &mut c.args);
            }
        }
    });
    walk_statements_mut(stmts, &mut |s| {
        if let Statement::Call(c) = s {
            if is_variadic(&c.callee) {
                pack(c.span, &mut c.args);
            }
        }
    });
}

fn is_zero_literal(e: &Expression) -> bool {
    matches!(e, Expression::Literal(Literal { value: LiteralValue::Integer(0), .. }))
}

/// `name = value`, or `name(i, j) = value` when `index` is non-empty.
fn assign(span: TextSpan, name: &str, index: Vec<Expression>, value: Expression) -> Statement {
    let ident = Expression::Identifier(Identifier { span, name: name.to_string() });
    let target = if index.is_empty() {
        ident
    } else {
        Expression::FunctionCall(FunctionCallExpression { span, callee: Box::new(ident), args: index })
    };
    Statement::Assignment(AssignmentStatement { span, target, value })
}

/// Element indices for `= {v1, v2, ...}`, filled in memory order (the last
/// subscript varies fastest, as the RapidQ manual describes). Returns `None`
/// when a bound other than the first dimension's lower bound isn't a literal.
fn initializer_indices(dimensions: &[ArrayDimension], count: usize) -> Option<Vec<Vec<Expression>>> {
    let literal = |e: &Expression| match e {
        Expression::Literal(Literal { value: LiteralValue::Integer(n), .. }) => Some(*n),
        Expression::Unary(u) if matches!(u.operator, UnaryOperator::Negate) => match &*u.operand {
            Expression::Literal(Literal { value: LiteralValue::Integer(n), .. }) => Some(-n),
            _ => None,
        },
        _ => None,
    };
    let span = TextSpan::new(0, 0);
    let int = |n: i64| Expression::Literal(Literal { span, value: LiteralValue::Integer(n) });
    if dimensions.len() <= 1 {
        // One dimension: index = lower bound + i (the lower bound may be any expression).
        let low = match dimensions.first() {
            Some(ArrayDimension::Range { start, .. }) => Some(start.clone()),
            _ => None,
        };
        return Some(
            (0..count as i64)
                .map(|i| {
                    vec![match (&low, low.as_ref().and_then(literal)) {
                        (_, Some(l)) => int(l + i),
                        (Some(l), None) => Expression::Binary(BinaryExpression {
                            span,
                            left: Box::new(l.clone()),
                            operator: BinaryOperator::Add,
                            right: Box::new(int(i)),
                        }),
                        (None, None) => int(i),
                    }]
                })
                .collect(),
        );
    }
    let mut bounds = Vec::new();
    for dim in dimensions {
        bounds.push(match dim {
            ArrayDimension::Single(upper) => (0, literal(upper)?),
            ArrayDimension::Range { start, end } => (literal(start)?, literal(end)?),
        });
    }
    let mut current: Vec<i64> = bounds.iter().map(|b| b.0).collect();
    let mut out = Vec::new();
    for _ in 0..count {
        out.push(current.iter().map(|&n| int(n)).collect());
        for d in (0..current.len()).rev() {
            current[d] += 1;
            if current[d] <= bounds[d].1 || d == 0 {
                break;
            }
            current[d] = bounds[d].0;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use rapidr_ast::*;
    use rapidr_lexer::Lexer;

    use super::parse_tokens;

    fn parse(code: &str) -> Vec<Statement> {
        let tokens = Lexer::new(code, None).tokenize().unwrap();
        match parse_tokens(&tokens) {
            Ok(program) => program.statements,
            Err(e) => panic!("unexpected parse errors:\n{e}"),
        }
    }

    /// (line, column, message) of every error, in order.
    fn errors(code: &str) -> Vec<(usize, usize, String)> {
        let tokens = Lexer::new(code, None).tokenize().unwrap();
        match parse_tokens(&tokens) {
            Ok(_) => Vec::new(),
            Err(e) => e
                .diagnostics
                .iter()
                .map(|d| (d.location.line, d.location.column, d.message.clone()))
                .collect(),
        }
    }

    #[test]
    fn unparseable_line_is_an_error_with_location() {
        let errs = errors("PRINT \"one\"\nDIM AS AS AS\nPRINT \"two\"\n");
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert_eq!((errs[0].0, errs[0].1), (2, 1));
        assert!(errs[0].2.contains("'AS' is a reserved word"), "{errs:?}");
    }

    #[test]
    fn reports_every_bad_line_not_just_the_first() {
        let errs = errors("DIM AS\nx = 1\nGOSUB\ny = 2\nDEFINT 5\n");
        let lines: Vec<usize> = errs.iter().map(|e| e.0).collect();
        assert_eq!(lines, vec![1, 3, 5], "{errs:?}");
        assert!(errs[1].2.contains("GOSUB needs a label"), "{errs:?}");
        assert!(errs[2].2.contains("Syntax error in DEFINT statement"), "{errs:?}");
    }

    #[test]
    fn leftover_tokens_after_a_statement_are_an_error() {
        let errs = errors("x = 1 y = 2\n");
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert_eq!((errs[0].0, errs[0].1), (1, 7));
        assert!(errs[0].2.contains("Unexpected 'y'"), "{errs:?}");
    }

    #[test]
    fn bad_statement_inside_single_line_if_is_an_error() {
        let errs = errors("IF 1 THEN DEFINT 5\n");
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(errs[0].2.contains("DEFINT"), "{errs:?}");
    }

    #[test]
    fn parses_labels_goto_and_gosub() {
        let stmts = parse("Start:\nGOSUB Helper\nGOTO 100\nHelper: PRINT 1\nRETURN\n100 PRINT 2\n");
        assert!(matches!(&stmts[0], Statement::Label(l) if l.name == "Start"));
        assert!(matches!(&stmts[1], Statement::Gosub(j) if j.label == "Helper"));
        assert!(matches!(&stmts[2], Statement::Goto(j) if j.label == "100"));
        assert!(matches!(&stmts[3], Statement::Label(l) if l.name == "Helper"));
        assert!(matches!(&stmts[4], Statement::Print(_)));
        assert!(matches!(&stmts[6], Statement::Label(l) if l.name == "100"));
        assert!(matches!(&stmts[7], Statement::Print(_)));
    }

    #[test]
    fn bare_end_is_a_statement() {
        let stmts = parse("PRINT 1\nEND\nSUB Foo()\nEND SUB\n");
        assert!(matches!(&stmts[1], Statement::Call(c) if matches!(&c.callee, Expression::Identifier(i) if i.name == "END")));
        assert!(matches!(&stmts[2], Statement::Subroutine(_)));
    }

    #[test]
    fn rapidq_component_names_map_to_rapidr() {
        let stmts = parse("DIM f AS QForm\nCREATE b AS QBUTTON\nEND CREATE\nDIM g AS QGauge\nDIM t AS MyType\n");
        assert!(matches!(&stmts[0], Statement::Dim(d) if d.type_name == "RFORM"));
        assert!(matches!(&stmts[1], Statement::Create(c) if c.type_name == "RBUTTON"));
        assert!(matches!(&stmts[2], Statement::Dim(d) if d.type_name == "RPROGRESSBAR"));
        assert!(matches!(&stmts[3], Statement::Dim(d) if d.type_name == "MyType"));
    }

    #[test]
    fn parses_type_with_events_methods_and_constructor() {
        let src = "TYPE TCounter EXTENDS QFORM\n  Count AS INTEGER\n  EVENT OnClick\n    This.Count = This.Count + 1\n  END EVENT\n  EVENT(OnResize)\n  END EVENT\n  SUB Reset\n    This.Count = 0\n  END SUB\n  CONSTRUCTOR\n    Caption = \"x\"\n  END CONSTRUCTOR\nEND TYPE\n";
        let Statement::Type(t) = &parse(src)[0] else { panic!("expected TYPE") };
        assert_eq!(t.extends.as_deref(), Some("RFORM"));
        assert_eq!(t.fields.len(), 1);
        assert_eq!(t.events.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), vec!["OnClick", "OnResize"]);
        assert_eq!(t.methods.len(), 1);
        assert_eq!(t.constructor.len(), 1);
    }

    #[test]
    fn unknown_lines_inside_type_are_errors() {
        let errs = errors("TYPE T\n  x AS INTEGER\n  PRINT 1\nEND TYPE\n");
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert_eq!(errs[0].0, 3);
    }

    #[test]
    fn keyword_used_as_a_name_says_so() {
        let errs = errors("DIM step AS DOUBLE\n");
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(errs[0].2.contains("'STEP' is a reserved word"), "{errs:?}");
    }

    #[test]
    fn errors_inside_blocks_keep_their_own_line() {
        let errs = errors("SUB Foo()\n  PRINT 1\n  DEFINT 5\nEND SUB\n");
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert_eq!(errs[0].0, 3);
        assert!(errs[0].2.contains("Syntax error in DEFINT statement"), "{errs:?}");
    }

    #[test]
    fn parses_directives_and_dim_statements() {
        // RapidQ manual: in `DIM x, y AS INTEGER` only y is INTEGER; x is a VARIANT.
        let stmts = parse("$APPTYPE GUI\nDIM x, y AS INTEGER\nDIM (a, b)(3) AS STRING, n AS LONG = 4\nDEFINT i = 1, j(2) = {7, 8, 9}\n");
        assert!(matches!(stmts[0], Statement::Directive(_)));
        let dims: Vec<(String, String, usize)> = stmts
            .iter()
            .filter_map(|s| match s {
                Statement::Dim(d) => Some((d.declarators[0].name.clone(), d.type_name.clone(), d.declarators[0].dimensions.len())),
                _ => None,
            })
            .collect();
        let expect = [("x", "VARIANT", 0), ("y", "INTEGER", 0), ("a", "STRING", 1), ("b", "STRING", 1), ("n", "LONG", 0), ("i", "INTEGER", 0), ("j", "INTEGER", 1)];
        assert_eq!(dims, expect.map(|(n, t, d)| (n.to_string(), t.to_string(), d)).to_vec());
        let assigns = stmts.iter().filter(|s| matches!(s, Statement::Assignment(_))).count();
        assert_eq!(assigns, 5, "n = 4, i = 1 and three elements of j");
    }

    #[test]
    fn parses_assignment_expression_tree() {
        let stmts = parse("x = 10 + 5\n");
        match &stmts[0] {
            Statement::Assignment(a) => {
                assert!(matches!(a.target, Expression::Identifier(_)));
                match &a.value {
                    Expression::Binary(b) => assert_eq!(b.operator, BinaryOperator::Add),
                    other => panic!("expected binary, got {other:?}"),
                }
            }
            other => panic!("expected assignment, got {other:?}"),
        }
    }

    #[test]
    fn parses_print_and_import() {
        let stmts = parse("PRINT \"Hi\", LEN(\"x\")\nIMPORT \"math\" AS math\n");
        match &stmts[0] {
            Statement::Print(p) => {
                assert_eq!(p.items.len(), 2);
                assert!(matches!(p.items[1], Expression::FunctionCall(_)));
            }
            other => panic!("expected print, got {other:?}"),
        }
        match &stmts[1] {
            Statement::Import(i) => {
                assert_eq!(i.module_name, "math");
                assert_eq!(i.alias.as_deref(), Some("math"));
            }
            other => panic!("expected import, got {other:?}"),
        }
    }

    #[test]
    fn parses_const_and_array_assignment() {
        let stmts = parse("CONST MAX_SIZE = 3\nA(0) = 42\n");
        match &stmts[0] {
            Statement::Const(c) => match &c.value {
                Expression::Literal(l) => assert_eq!(l.value, LiteralValue::Integer(3)),
                other => panic!("expected literal, got {other:?}"),
            },
            other => panic!("expected const, got {other:?}"),
        }
        match &stmts[1] {
            Statement::Assignment(a) => {
                assert!(matches!(a.target, Expression::FunctionCall(_)));
            }
            other => panic!("expected assignment, got {other:?}"),
        }
    }

    #[test]
    fn parses_member_and_explicit_call_statements() {
        let stmts = parse("form.ShowModal()\nCALL TestSub(\"It works!\")\n");
        match &stmts[0] {
            Statement::Call(c) => {
                assert!(matches!(c.callee, Expression::MemberAccess(_)));
                assert_eq!(c.args.len(), 0);
            }
            other => panic!("expected call, got {other:?}"),
        }
        match &stmts[1] {
            Statement::Call(c) => {
                assert!(matches!(c.callee, Expression::Identifier(_)));
                assert_eq!(c.args.len(), 1);
            }
            other => panic!("expected call, got {other:?}"),
        }
    }

    #[test]
    fn parses_for_loop() {
        let stmts = parse("FOR i = 1 TO 5\n  PRINT i\nNEXT i\n");
        match &stmts[0] {
            Statement::For(f) => {
                assert_eq!(f.variable, "i");
                assert_eq!(f.body.len(), 1);
                assert!(f.step.is_none());
            }
            other => panic!("expected for, got {other:?}"),
        }
    }

    #[test]
    fn parses_for_with_step() {
        let stmts = parse("FOR x = 10 TO 0 STEP -2\n  PRINT x\nNEXT\n");
        match &stmts[0] {
            Statement::For(f) => {
                assert_eq!(f.variable, "x");
                assert!(f.step.is_some());
                assert_eq!(f.body.len(), 1);
            }
            other => panic!("expected for, got {other:?}"),
        }
    }

    #[test]
    fn parses_while_loop() {
        let stmts = parse("WHILE x > 0\n  x = x - 1\nWEND\n");
        match &stmts[0] {
            Statement::While(w) => {
                assert_eq!(w.body.len(), 1);
            }
            other => panic!("expected while, got {other:?}"),
        }
    }

    #[test]
    fn parses_if_block() {
        let stmts = parse("IF x > 5 THEN\n  PRINT \"big\"\nELSE\n  PRINT \"small\"\nEND IF\n");
        match &stmts[0] {
            Statement::If(i) => {
                assert_eq!(i.then_body.len(), 1);
                assert_eq!(i.else_body.len(), 1);
                assert!(i.elseif_branches.is_empty());
            }
            other => panic!("expected if, got {other:?}"),
        }
    }

    #[test]
    fn parses_sub_definition() {
        let stmts = parse("SUB MySub(a AS INTEGER, b AS STRING)\n  PRINT a, b\nEND SUB\n");
        match &stmts[0] {
            Statement::Subroutine(s) => {
                assert_eq!(s.name, "MySub");
                assert_eq!(s.params.len(), 2);
                assert_eq!(s.params[0].type_name, "INTEGER");
                assert_eq!(s.body.len(), 1);
            }
            other => panic!("expected sub, got {other:?}"),
        }
    }

    #[test]
    fn parses_function_definition() {
        let stmts = parse("FUNCTION Add(a AS INTEGER, b AS INTEGER) AS INTEGER\n  Add = a + b\nEND FUNCTION\n");
        match &stmts[0] {
            Statement::Function(f) => {
                assert_eq!(f.name, "Add");
                assert_eq!(f.params.len(), 2);
                assert_eq!(f.return_type.as_deref(), Some("INTEGER"));
                assert_eq!(f.body.len(), 1);
            }
            other => panic!("expected function, got {other:?}"),
        }
    }

    #[test]
    fn parses_create_block() {
        let stmts = parse("CREATE frm AS RForm\n  Caption = \"Hello\"\n  Width = 400\nEND CREATE\n");
        match &stmts[0] {
            Statement::Create(c) => {
                assert_eq!(c.name, "frm");
                assert_eq!(c.type_name, "RForm");
                assert_eq!(c.body.len(), 2);
            }
            other => panic!("expected create, got {other:?}"),
        }
    }

    #[test]
    fn parses_with_block() {
        let stmts = parse("WITH obj\n  .X = 1\n  .Y = 2\nEND WITH\n");
        match &stmts[0] {
            Statement::With(w) => {
                assert_eq!(w.body.len(), 2);
            }
            other => panic!("expected with, got {other:?}"),
        }
    }

    #[test]
    fn parses_type_definition() {
        let stmts = parse("TYPE Rect\n  Left AS INTEGER\n  Top AS INTEGER\nEND TYPE\n");
        match &stmts[0] {
            Statement::Type(t) => {
                assert_eq!(t.name, "Rect");
                assert_eq!(t.fields.len(), 2);
                assert_eq!(t.fields[0].name, "Left");
            }
            other => panic!("expected type, got {other:?}"),
        }
    }

    #[test]
    fn parses_select_case() {
        let stmts = parse("SELECT CASE x\n  CASE 1\n    PRINT \"one\"\n  CASE 2, 3\n    PRINT \"two or three\"\n  CASE ELSE\n    PRINT \"other\"\nEND SELECT\n");
        match &stmts[0] {
            Statement::SelectCase(s) => {
                assert_eq!(s.cases.len(), 2);
                assert_eq!(s.cases[1].values.len(), 2);
                assert_eq!(s.case_else.len(), 1);
            }
            other => panic!("expected select case, got {other:?}"),
        }
    }

    #[test]
    fn parses_case_ranges_and_is_comparisons() {
        let stmts = parse("SELECT CASE n\n  CASE 1 TO 5, 9\n    x = 1\n  CASE IS >= 10\n    x = 2\nEND SELECT\n");
        let Statement::SelectCase(s) = &stmts[0] else { panic!("expected select case") };
        assert!(matches!(s.cases[0].values[0], CaseValue::Range(_, _)));
        assert!(matches!(s.cases[0].values[1], CaseValue::Value(_)));
        assert!(matches!(s.cases[1].values[0], CaseValue::Is(BinaryOperator::GreaterThanOrEqual, _)));
    }

    #[test]
    fn bad_case_list_is_reported_on_its_own_line() {
        let errs = errors("SELECT CASE n\n  CASE 1 2\n    x = 1\nEND SELECT\n");
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert_eq!(errs[0].0, 2, "{errs:?}");
    }
}