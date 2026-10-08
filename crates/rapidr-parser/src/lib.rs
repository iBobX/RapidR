use std::error::Error;
use std::fmt;
use std::path::Path;

use rapidr_ast::*;
use rapidr_diagnostics::{Diagnostic, Severity, SourceLocation, TextSpan};
use rapidr_lexer::{lex_file, Token, TokenType};

pub mod tools;
pub use tools::{parse_file_for_tools, parse_source_for_tools, Location, ToolsParse};

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
    let structs = retag_structs(tokens);
    let tokens = structs.as_deref().unwrap_or(tokens);
    let retagged = retag_routine_names(tokens);
    // (the program's TYPE names, before anything names a type: one called
    // like a RapidR-only component stays the program's — rapidr_ast)
    let type_names: Vec<String> = tokens.windows(2).filter(|w| w[0].kind == TokenType::Type && w[1].kind == TokenType::Identifier).map(|w| w[1].lexeme.clone()).collect();
    rapidr_ast::set_program_types(&type_names);
    let mut parser = Parser::new(retagged.as_deref().unwrap_or(tokens));
    let program = parser.parse_program();
    (program, parser.diagnostics)
}

/// RapidQ's `STRUCT name` … `END STRUCT`: a user-defined type as TYPE is
/// ("a UDT defined by STRUCT or TYPE", RapidQ's pointer notes): the tokens
/// become TYPE's. `None` when there is none.
fn retag_structs(tokens: &[Token]) -> Option<Vec<Token>> {
    let is_struct = |t: &Token| t.kind == TokenType::Identifier && t.lexeme.eq_ignore_ascii_case("STRUCT");
    let starts_line = |i: usize| i == 0 || matches!(tokens[i - 1].kind, TokenType::Newline | TokenType::Colon);
    let at: Vec<usize> = (0..tokens.len())
        .filter(|&i| {
            is_struct(&tokens[i])
                && ((starts_line(i)
                    && tokens.get(i + 1).is_some_and(|n| n.kind == TokenType::Identifier)
                    && tokens.get(i + 2).is_none_or(|n| n.kind == TokenType::Newline))
                    || (i > 0 && tokens[i - 1].lexeme.eq_ignore_ascii_case("END")))
        })
        .collect();
    if at.is_empty() {
        return None;
    }
    let mut out = tokens.to_vec();
    for i in at {
        out[i].kind = TokenType::Type;
        out[i].lexeme = "TYPE".to_string();
    }
    Some(out)
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
    /// Parameters of the routine being parsed that a keyword names
    /// (`SetMultiSelect(select AS BOOLEAN)`), lowercase: in its code they
    /// are the parameter.
    keyword_params: Vec<String>,
    /// `$OPTION BYREF` seen: parameters without BYVAL are passed by
    /// reference from there on (RapidQ's default is BYVAL).
    default_by_ref: bool,
    /// Keywords the program uses as variables (`type = 2`), lowercase.
    keyword_vars: Vec<String>,
    /// The program's own TYPEs named like one of RapidQ's include-library
    /// components (`TYPE QBEVEL EXTENDS QPANEL`, QBevel.inc), upper case:
    /// those names are the program's TYPE, not RapidR's built-in.
    own_types: Vec<String>,
    /// The objects of the WITH blocks being parsed, innermost last, as
    /// RC.EXE names them in its errors ([`member_path`]).
    with_objects: Vec<(String, String)>,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0, diagnostics: Vec::new(), pending: Vec::new(), variadic: Vec::new(), for_counter: 0, data_items: Vec::new(), data_labels: Vec::new(), labels_awaiting_data: Vec::new(), keyword_params: Vec::new(), keyword_vars: Vec::new(), default_by_ref: false, own_types: Vec::new(), with_objects: Vec::new() }
    }

    /// A type's name as written: RapidR's name for a RapidQ component
    /// (`rapidr_ast::component_type_reference`), the program's own TYPE
    /// first.
    fn type_ref(&self, name: &str) -> String {
        rapidr_ast::component_type_reference(name, &self.own_types)
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

    /// Whether a member's name follows the `.` just read, right after it.
    /// RC.EXE takes nothing else: `Form.` at a line's end (the next line
    /// isn't joined to it) or `Form. Caption` is its `Member  not part of
    /// class FORM` — the member it read is empty. A `_` continuation
    /// joins the next line's name to it (`REDITPOP(i)._` then `POPUP(x, y)`,
    /// RapidQ's MultiCaptiveChildWnds.bas). (RC refuses `Form. _` too — a
    /// space before the `_` —, which the tokens can't tell apart: open.)
    fn member_follows(&self) -> bool {
        let (Some(dot), Some(next)) = (self.tokens.get(self.pos.wrapping_sub(1)), self.peek()) else { return false };
        !matches!(next.kind, TokenType::Newline | TokenType::Eof | TokenType::Colon) && (next.span.start == dot.span.end || next.line > dot.line)
    }

    /// An object as RC.EXE names it in `Member P not part of class O`: the
    /// name it starts from in upper case, and the members read after it so
    /// far, each with its dot (`Form.Font.` is `FONT.` of `FORM`). Inside a
    /// WITH, `.Font` starts from the WITH's object.
    fn member_path(&self, e: &Expression) -> (String, String) {
        match e {
            Expression::Identifier(i) if i.name == "_with_" => self.with_objects.last().cloned().unwrap_or_default(),
            Expression::Identifier(i) => (i.name.to_ascii_uppercase(), String::new()),
            Expression::MemberAccess(m) => {
                let (object, path) = self.member_path(&m.object);
                (object, format!("{path}{}.", m.member.to_ascii_uppercase()))
            }
            Expression::FunctionCall(f) => self.member_path(&f.callee),
            Expression::ArrayAccess(a) => self.member_path(&a.array),
            Expression::MethodCall(m) => {
                let (object, path) = self.member_path(&m.object);
                (object, format!("{path}{}.", m.method.to_ascii_uppercase()))
            }
            _ => (String::new(), String::new()),
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

    /// Whether the token `n` ahead ends the line (or the program).
    fn at_eol_at(&self, n: usize) -> bool {
        matches!(self.peek_kind_at(n), None | Some(TokenType::Newline | TokenType::Colon))
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
                // (`END STRUCT` ends a TYPE: STRUCT is its synonym)
                t.lexeme.eq_ignore_ascii_case(kw) || (kw == "TYPE" && t.kind == TokenType::Type)
            })
    }

    // -----------------------------------------------------------------------
    // Program
    // -----------------------------------------------------------------------

    fn parse_program(&mut self) -> Program {
        let start = self.pos;
        let mut body = self.parse_body(&[]);
        declared_parameters(&mut body);
        pack_variadic_calls(&mut body, &self.variadic);
        input_chars(&mut body);
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
                // `310 NEXT I`: the line's label, then the block's end.
                if self.is_at_terminator(terminators) {
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
                let message = format!("Expected end-of-line but got {}", tok.lexeme); // (RapidQ's compiler's words)
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

    /// `INPUT #n, a, b` and `LINE INPUT #n, s`: reads from the file open as
    /// #n, as assignments of the `input_field` / `line_input` builtins.
    fn parse_file_input(&mut self, line: bool) -> Option<Statement> {
        let start = self.pos;
        if line {
            self.advance()?; // LINE
        }
        self.advance()?; // INPUT
        self.expect(TokenType::Hash)?;
        let file = self.parse_expression()?;
        self.expect(TokenType::Comma)?;
        let builtin = if line { "line_input" } else { "input_field" };
        let mut assignments = Vec::new();
        loop {
            let target = self.parse_postfix_expression()?;
            let span = self.span_from(start);
            let read = Expression::FunctionCall(FunctionCallExpression {
                span,
                callee: Box::new(Expression::Identifier(Identifier { span, name: builtin.into() })),
                args: vec![file.clone()],
            });
            assignments.push(Statement::Assignment(AssignmentStatement { span, target, value: read }));
            if line || !self.match_kind(TokenType::Comma) {
                break;
            }
        }
        let mut rest = assignments.into_iter();
        let first = rest.next()?;
        self.pending.extend(rest);
        Some(first)
    }

    fn parse_statement_inner(&mut self) -> Option<Statement> {
        // `LINE INPUT #n, s`
        if self.peek().is_some_and(|t| t.kind == TokenType::Identifier && t.lexeme.eq_ignore_ascii_case("LINE"))
            && self.peek_kind_at(1) == Some(TokenType::Input)
            && self.peek_kind_at(2) == Some(TokenType::Hash)
        {
            return self.parse_file_input(true);
        }
        // `type = 2`: a keyword RapidQ lets name a variable (a keyword and
        // `=` start no statement); from here on it is that variable.
        if self.peek_kind() != Some(TokenType::Identifier) && self.peek_is_word() && self.peek_kind_at(1) == Some(TokenType::Eq) {
            let name = self.peek()?.lexeme.to_ascii_lowercase();
            if !self.keyword_vars.contains(&name) {
                self.keyword_vars.push(name);
            }
        }
        match self.peek_kind()? {
            // (only before `=` or `.`: `CASE 1` stays a CASE in a routine
            // with a parameter named case)
            _ if self.peek().is_some_and(|t| t.kind != TokenType::Identifier && self.is_keyword_name(&t.lexeme))
                && matches!(self.peek_kind_at(1), Some(TokenType::Eq | TokenType::Dot)) =>
            {
                self.parse_assignment_or_call()
            }
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
            TokenType::Create => self.parse_create(),
            TokenType::With => self.parse_with().map(Statement::With),
            TokenType::Exit => self.parse_exit().map(Statement::Exit),
            TokenType::Return => self.parse_return().map(Statement::Return),
            TokenType::Input if self.peek_kind_at(1) == Some(TokenType::Hash) => self.parse_file_input(false),
            TokenType::Input => self.parse_input().map(Statement::Input),
            TokenType::Bind => self.parse_bind().map(Statement::Bind),
            TokenType::Declare => self.parse_declare().map(Statement::Declare),
            // `LPRINT …`: as PRINT, to the printer (`__lprint(newline, item,
            // zone, …)`; rapidr_value::lprint).
            TokenType::Identifier if self.peek_identifier_eq("LPRINT") && self.peek_kind_at(1) != Some(TokenType::Eq) => self.parse_lprint(),
            TokenType::RustStart => self.parse_rust_block().map(Statement::RustBlock),
            // A bare `END` ends the program (the END builtin). `END IF`,
            // `END SUB`, … are block terminators and never reach here.
            // A stray `END TYPE` / `END STRUCT` (no TYPE open: one that is
            // ends in parse_type_def) is RC.EXE's END too — the program
            // ends there, in a SUB as well (probes in the Windows VM;
            // RapidQ's Network/Download/qdownload.bas ends with one).
            TokenType::End
                if matches!(self.peek_kind_at(1), None | Some(TokenType::Newline | TokenType::Colon | TokenType::Eof))
                    || (self.peek_kind_at(1) == Some(TokenType::Type)
                        && matches!(self.peek_kind_at(2), None | Some(TokenType::Newline | TokenType::Colon | TokenType::Eof))) =>
            {
                let tok = self.advance()?;
                if self.peek_kind() == Some(TokenType::Type) {
                    self.advance();
                }
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
            // VB's `ON ERROR RESUME NEXT` / `ON ERROR GOTO label|0`: accepted, not acted on
            // (a run-time error still ends the program, as RapidQ itself does).
            TokenType::Identifier
                if self.peek_identifier_eq("ON")
                    && self.tokens.get(self.pos + 1).is_some_and(|t| t.lexeme.eq_ignore_ascii_case("ERROR")) =>
            {
                let start = self.pos;
                self.skip_to_eol();
                let span = self.span_from(start);
                let text = self.tokens[start..self.pos].iter().map(|t| t.lexeme.as_str()).collect::<Vec<_>>().join(" ");
                Some(Statement::Comment(CommentStatement { span, text: format!("{text} (not supported: a run-time error still ends the program)") }))
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
        if name.eq_ignore_ascii_case("$OPTION") {
            if let Some(v) = value.as_deref().map(|v| v.trim().to_ascii_uppercase()) {
                if v.starts_with("BYREF") {
                    self.default_by_ref = true;
                }
                if let Some(mode) = v.strip_prefix("INKEY$") {
                    // `$OPTION INKEY$ TRAPALL` / `DEFAULT`: set from here on.
                    let span = tok.span;
                    let on = mode.trim().starts_with("TRAPALL") as i64;
                    self.pending.push(Statement::Call(CallStatement { span, callee: ident(span, "__inkey_trapall"), args: vec![Expression::Literal(Literal { span, value: LiteralValue::Integer(on) })] }));
                }
                if let Some(d) = v.strip_prefix("DECIMAL") {
                    // `$OPTION DECIMAL ","` or `$OPTION DECIMAL 44`: set from here on.
                    let d = d.trim();
                    let span = tok.span;
                    let value = match d.strip_prefix('"') {
                        Some(q) => LiteralValue::String(q.trim_end_matches('"').to_string()),
                        None => LiteralValue::Integer(d.parse().unwrap_or(46)),
                    };
                    self.pending.push(Statement::Call(CallStatement { span, callee: ident(span, "__decimal"), args: vec![Expression::Literal(Literal { span, value })] }));
                }
            }
        }
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
            } else if fixed_type.is_some()
                && self.peek_kind() != Some(TokenType::Identifier)
                && self.peek_kind() != Some(TokenType::As)
                && self.peek_is_word()
                && (self.at_eol_at(1) || matches!(self.peek_kind_at(1), Some(TokenType::Comma | TokenType::As | TokenType::LParen | TokenType::Eq)))
            {
                // `DEFSTR return` (qcgi.inc): a keyword RapidQ lets name a
                // variable; from here on it is that variable.
                let tok = self.advance()?.clone();
                let name = tok.lexeme.to_ascii_lowercase();
                if !self.keyword_vars.contains(&name) {
                    self.keyword_vars.push(name);
                }
                names.push(tok);
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
            let explicit_type = fixed_type.is_some() || self.peek_kind() == Some(TokenType::As);
            let type_name = match fixed_type {
                Some(t) => t.to_string(),
                None if self.match_kind(TokenType::As) => {
                    let t = { let n = self.advance()?.lexeme.clone(); self.type_ref(&n) };
                    t + &self.template_args()
                }
                // `DIM m` with no type: a DOUBLE, whatever `$OPTION DIM`
                // says (it types undeclared names only — RC.EXE: `$OPTION
                // DIM INTEGER : DIM m : m = 9 / 4` holds 2.25).
                None => "DOUBLE".to_string(),
            };
            // `AS STRING * 20`: a fixed-length string (a STRING cut to 20).
            let mut fixed_len = None;
            if self.match_kind(TokenType::Star) {
                let len = self.parse_unary()?;
                if type_name == "STRING" {
                    fixed_len = literal_length(&len);
                }
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
                    // `DIM n%` (no AS): the suffix says the type.
                    type_name: match rapidr_ast::suffix_type(&name_tok.lexeme) {
                        Some(t) if !explicit_type => t.to_string(),
                        _ => type_name.clone(),
                    },
                    fixed_len,
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
        // Only a trailing `;` or `,` keeps the cursor on the line. In
        // RapidQ "the comma and semi-colon have the same effect" (its
        // manual; RC.EXE: `PRINT 1, 2` prints 12) — no QBasic print zones —
        // and a separator may come first (`PRINT , "y"`). An ELSE ends the
        // PRINT of a single-line IF (`IF c THEN PRINT "a"; ELSE …`).
        let mut append_newline = true;
        let ends = |p: &Self| p.at_eol() || matches!(p.peek_kind(), Some(TokenType::Colon | TokenType::Else));
        while !ends(self) {
            if self.match_kind(TokenType::Comma) || self.match_kind(TokenType::Semi) {
                append_newline = false;
                continue;
            }
            items.push(self.parse_expression()?);
            let separated = self.match_kind(TokenType::Comma) || self.match_kind(TokenType::Semi);
            zones.push(false);
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

    fn parse_lprint(&mut self) -> Option<Statement> {
        let start = self.pos;
        let span = self.advance()?.span;
        let mut args = Vec::new();
        let mut append_newline = true;
        // ("just like PRINT": `,` as `;`, no print zones)
        while !self.at_eol() && !matches!(self.peek_kind(), Some(TokenType::Colon | TokenType::Else)) {
            if self.match_kind(TokenType::Comma) || self.match_kind(TokenType::Semi) {
                append_newline = false;
                continue;
            }
            args.push(self.parse_expression()?);
            let separated = self.match_kind(TokenType::Comma) || self.match_kind(TokenType::Semi);
            args.push(Expression::Literal(Literal { span, value: LiteralValue::Integer(0) }));
            append_newline = !separated;
            if !separated {
                break;
            }
        }
        args.insert(0, Expression::Literal(Literal { span, value: LiteralValue::Integer(append_newline as i64) }));
        Some(Statement::Call(CallStatement { span: self.span_from(start), callee: ident(span, "__lprint"), args }))
    }

    fn parse_explicit_call(&mut self) -> Option<CallStatement> {
        let start = self.pos;
        self.expect(TokenType::Call)?;
        let callee = self.parse_postfix_expression()?;
        // (`IF x THEN CALL S(1, 2): x = 0` — the call ends at the `:` or ELSE too)
        let args = if self.at_eol() || matches!(self.peek_kind(), Some(TokenType::Colon | TokenType::Else)) {
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
            Some({ let n = self.advance()?.lexeme.clone(); self.type_ref(&n) })
        } else {
            None
        };
        self.consume_eol();
        let keyword = if is_function { "FUNCTIONI" } else { "SUBI" };
        // (RapidQ also closed them with END FUNCTION / END SUB: QAVI.inc)
        let plain = if is_function { "FUNCTION" } else { "SUB" };
        let mut body = self.parse_body(&[Terminator::EndPair(keyword), Terminator::EndPair(plain)]);
        if self.peek_is_end_followed_by(keyword) || self.peek_is_end_followed_by(plain) {
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

        // `IF c THEN: a: b: END IF` (or `THEN :a` with `ELSE :b` and END IF on
        // later lines): a colon right after THEN starts a block, whose
        // statements are separated by colons or lines.
        let block_after_colon = self.match_kind(TokenType::Colon);

        // Single-line IF: non-empty remainder after THEN on the same line
        if !block_after_colon && !self.at_eol() {
            let mut then_body = Vec::new();
            // Parse one or more colon-separated statements on the THEN line
            loop {
                if let Some(s) = self.parse_statement() {
                    then_body.push(s);
                    then_body.append(&mut self.pending);
                }
                if !self.match_kind(TokenType::Colon) || self.at_eol() {
                    break;
                }
            }
            // RapidQ's PRINT right before the ELSE of a single-line IF ends
            // without a new line (RC.EXE: `IF 1 THEN PRINT "a" ELSE PRINT
            // "b" : PRINT "c"` prints "ac").
            if self.peek_kind() == Some(TokenType::Else) {
                if let Some(Statement::Print(p)) = then_body.last_mut() {
                    p.append_newline = false;
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
                    if !self.match_kind(TokenType::Colon) || self.at_eol() {
                        break;
                    }
                }
            }
            // `IF c THEN x END IF` on one line: the END IF is redundant.
            if self.peek_kind() == Some(TokenType::End) && self.peek_kind_at(1) == Some(TokenType::If) {
                self.advance();
                self.advance();
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
            self.match_kind(TokenType::Colon);
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
            // The comparison's right side stops at AND / OR / XOR, which go
            // on with the comparison as their first operand (`CASE IS = "l"
            // AND x = "d"` is `(sel = "l") AND (x = "d")`, as RapidQ reads it).
            let operand = self.parse_comparison_operand()?;
            let mut rest = Vec::new();
            loop {
                let logic = match self.peek_kind() {
                    Some(TokenType::And) => BinaryOperator::And,
                    Some(TokenType::Or) => BinaryOperator::Or,
                    Some(TokenType::Xor) => BinaryOperator::Xor,
                    _ => break,
                };
                self.advance();
                rest.push((logic, self.parse_not()?));
            }
            return Some(if rest.is_empty() { CaseValue::Is(op, operand) } else { CaseValue::IsLogic(op, operand, rest) });
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
            // `CASE 1: PRINT "one"` — the body may start on the same line
            // (and without the colon after a keyword: `CASE 1 PRINT "one"`;
            // `CASE 4, 7  C = -2` is the list 4, 7 — RC.EXE reads the
            // assignment as an operand side by side with the 7:
            // parse_stacked_expression).
            self.match_kind(TokenType::Colon);
            if self.at_eol() {
                self.consume_eol();
            }
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
        // (a keyword names a method too: QDataBase.inc's `FUNCTION Create`)
        let mut name = if self.peek_is_word() && self.peek_kind() != Some(TokenType::Identifier) {
            self.advance()?.lexeme.clone()
        } else {
            self.expect(TokenType::Identifier)?.lexeme.clone()
        };
        while self.peek_kind() == Some(TokenType::Dot) && self.tokens.get(self.pos + 1).is_some_and(|t| t.kind != TokenType::Newline) {
            self.advance();
            name = format!("{name}.{}", self.advance()?.lexeme);
        }
        Some(name)
    }

    /// Whether keyword `word` names a parameter of this routine or a
    /// variable of the program here.
    fn is_keyword_name(&self, word: &str) -> bool {
        let w = word.to_ascii_lowercase();
        self.keyword_params.contains(&w) || self.keyword_vars.contains(&w)
    }

    /// Whether the next token is a word (a name or a keyword), not a symbol.
    /// A template type's arguments after its name (`NewClass<INTEGER, 10>`),
    /// as the type name's `<INTEGER,10>`; "" when there are none.
    fn template_args(&mut self) -> String {
        if self.peek_kind() != Some(TokenType::Lt) {
            return String::new();
        }
        self.advance();
        let mut args = Vec::new();
        let mut current = String::new();
        while !self.at_eol() {
            let Some(tok) = self.advance() else { break };
            match tok.kind {
                TokenType::Gt => break,
                TokenType::Comma => args.push(std::mem::take(&mut current)),
                _ => current.push_str(&tok.lexeme),
            }
        }
        args.push(current);
        format!("<{}>", args.join(","))
    }

    fn peek_is_word(&self) -> bool {
        self.peek_is_word_at(0)
    }

    /// Whether the token `n` ahead is a word (an identifier or a keyword).
    fn peek_is_word_at(&self, n: usize) -> bool {
        self.tokens.get(self.pos + n).is_some_and(|t| t.lexeme.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_') && t.lexeme.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
    }

    fn parse_sub(&mut self) -> Option<SubroutineStatement> {
        let start = self.pos;
        self.expect(TokenType::Sub)?;
        let name = self.parse_routine_name()?;
        self.keyword_params.clear();
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
        self.keyword_params.clear();
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
            rapidr_ast::suffix_type(&name).map(str::to_string)
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
        if rapidr_ast::is_include_library_component(&name) {
            self.own_types.push(name.to_ascii_uppercase());
        }
        // A template (manual 10.8): `TYPE NewClass<DataType, Size> …` —
        // made for each `DIM x AS NewClass<INTEGER, 10>` (rapidr_ast::templates).
        let mut template_params = Vec::new();
        if self.match_kind(TokenType::Lt) {
            while !self.at_eol() && !self.match_kind(TokenType::Gt) {
                let tok = self.advance()?;
                if tok.kind != TokenType::Comma {
                    template_params.push(tok.lexeme.clone());
                }
            }
        }
        // `TYPE X EXTENDS QFORM` or the manual's `TYPE X AS QFORM`. QOBJECT is
        // RapidQ's empty base object: a plain TYPE with methods.
        let extends = if self.match_kind(TokenType::Extends) || self.match_kind(TokenType::As) {
            Some({ let n = self.expect(TokenType::Identifier)?.lexeme.clone(); self.type_ref(&n) })
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
                self.keyword_params.clear();
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
                    // (`END PROPERTY SET`, as QRichEditXt.inc writes it)
                    self.match_kind(TokenType::Set);
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
            // Field: name[(dims)] AS Type [* len] [PROPERTY SET Setter] — a
            // keyword names a field too (`Step AS DOUBLE`, `Open AS INTEGER`).
            if self.peek_kind() == Some(TokenType::Identifier) || (self.peek_is_word() && self.peek_kind_at(1) == Some(TokenType::As)) {
                let field_start = self.pos;
                let mut fname = self.advance()?.lexeme.clone();
                // `hdr.hwndFrom AS LONG`, `Table.Name(150) AS STRING`: a
                // field of a nested record (rapidr_ast::dotted_fields).
                while self.peek_kind() == Some(TokenType::Dot) && (self.peek_kind_at(1) == Some(TokenType::Identifier) || self.peek_is_word_at(1)) {
                    self.advance();
                    let part = self.advance()?.lexeme.clone();
                    fname = format!("{fname}.{part}");
                }
                let mut more_dims: Vec<(Expression, Expression)> = Vec::new();
                let (arr, arr_lower) = if self.peek_kind() == Some(TokenType::LParen) && self.peek_kind_at(1) == Some(TokenType::RParen) {
                    // `Hint() AS STRING` (QtoolBar.inc): an array field whose
                    // size the TYPE doesn't give — room for 0 to 255 (REDIM
                    // makes it another size).
                    self.advance();
                    self.advance();
                    (Some(Expression::Literal(Literal { span: self.span_from(field_start), value: LiteralValue::Integer(255) })), None)
                } else if self.match_kind(TokenType::LParen) {
                    // An array field: `Names(2)`, `Colors(1 TO 16)`, `vertex(9, 2)`.
                    let dims = self.parse_array_dimensions();
                    self.expect(TokenType::RParen);
                    let span = self.span_from(field_start);
                    let bounds = |d: &ArrayDimension| match d {
                        ArrayDimension::Single(upper) => (Expression::Literal(Literal { span, value: LiteralValue::Integer(0) }), upper.clone()),
                        ArrayDimension::Range { start, end } => (start.clone(), end.clone()),
                    };
                    match dims.as_deref() {
                        Some([first, rest @ ..]) => {
                            let (lo, hi) = bounds(first);
                            more_dims = rest.iter().map(bounds).collect();
                            (Some(hi), (!is_zero_literal(&lo)).then_some(lo))
                        }
                        _ => {
                            self.error_at(field_start, format!("Array field {fname} in TYPE {name}: the dimensions aren't valid"));
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
                let mut ftype = self.type_ref(&type_tok.lexeme) + &self.template_args();
                // `OnReady AS EVENT(Template)`: a custom event (holds a SUB).
                if ftype.eq_ignore_ascii_case("EVENT") && self.match_kind(TokenType::LParen) {
                    while !self.at_eol() && !self.match_kind(TokenType::RParen) {
                        self.pos += 1;
                    }
                    ftype = "EVENT".to_string();
                }
                // `Name AS STRING * 20`: a fixed-length string (a STRING cut to 20).
                let mut fixed_len = None;
                if self.match_kind(TokenType::Star) {
                    if let Some(len) = self.parse_unary() {
                        if ftype.eq_ignore_ascii_case("STRING") {
                            fixed_len = literal_length(&len);
                        }
                    }
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
                    fixed_len,
                    array_size: arr,
                    array_lower: arr_lower,
                    more_dims,
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
            template_params,
        })
    }

    fn parse_create(&mut self) -> Option<Statement> {
        let start = self.pos;
        self.expect(TokenType::Create)?;
        let name = self.expect(TokenType::Identifier)?.lexeme.clone();
        // `CREATE cells(0 TO 9, 0 TO 4) AS QBITMAP … END CREATE`: an array of
        // components, as `DIM cells(0 TO 9, 0 TO 4) AS QBITMAP`.
        if self.match_kind(TokenType::LParen) {
            let dimensions = self.parse_array_dimensions().unwrap_or_default();
            self.expect(TokenType::RParen)?;
            self.expect(TokenType::As)?;
            let type_name = { let n = self.expect(TokenType::Identifier)?.lexeme.clone(); self.type_ref(&n) };
            self.consume_eol();
            let body = self.parse_body(&[Terminator::EndPair("CREATE")]);
            self.expect(TokenType::End);
            self.expect(TokenType::Create);
            if body.iter().any(|s| !matches!(s, Statement::Comment(_))) {
                self.error_at(start, format!("CREATE {name}(…) makes an array of components: set their properties after it, one element at a time"));
            }
            let span = self.span_from(start);
            return Some(Statement::Dim(DimStatement {
                span,
                declarators: vec![VariableDeclarator { span, name, dimensions }],
                type_name,
                fixed_len: None,
                is_static: false,
                is_redim: false,
            }));
        }
        self.expect(TokenType::As)?;
        let type_name = { let n = self.expect(TokenType::Identifier)?.lexeme.clone(); self.type_ref(&n) };
        self.consume_eol();
        let body = self.parse_body(&[Terminator::EndPair("CREATE")]);
        self.expect(TokenType::End);
        self.expect(TokenType::Create);
        Some(Statement::Create(CreateStatement {
            span: self.span_from(start),
            name,
            type_name,
            body,
        }))
    }

    fn parse_with(&mut self) -> Option<WithStatement> {
        let start = self.pos;
        self.expect(TokenType::With)?;
        let object = self.parse_expression()?;
        self.consume_eol();
        let path = self.member_path(&object);
        self.with_objects.push(path);
        // RapidQ lets `END SUB` / `END FUNCTION` close a WITH left open.
        let body = self.parse_body(&[Terminator::EndPair("WITH"), Terminator::EndPair("SUB"), Terminator::EndPair("FUNCTION")]);
        self.with_objects.pop();
        if self.peek_is_end_followed_by("WITH") {
            self.expect(TokenType::End);
            self.expect(TokenType::With);
        }
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
                self.default_by_ref
            };
            // A keyword may name a parameter (`(hwnd AS LONG, type AS LONG)`
            // in RAPIDQ2.INC's API declarations).
            let pname = match self.peek() {
                Some(t) if t.kind == TokenType::Identifier => self.advance()?.lexeme.clone(),
                Some(t) if t.lexeme.chars().all(|c| c.is_ascii_alphabetic())
                    && self.peek_kind_at(1) == Some(TokenType::As) =>
                {
                    let name = self.advance()?.lexeme.clone();
                    self.keyword_params.push(name.to_ascii_lowercase());
                    name
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
                ({ let n = self.advance()?.lexeme.clone(); self.type_ref(&n) }) + &self.template_args()
            } else {
                rapidr_ast::suffix_type(&pname).unwrap_or("VARIANT").to_string()
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
        if let Some(e) = self.try_postfix() {
            return Some(e);
        }
        let start = self.pos;
        let e = self.parse_logical_or();
        // An operand right after a whole expression (`A B OR C`,
        // `-9(COS(x))`), or inside its parentheses (`(2 3) + 1`, which the
        // infix reading refuses): RapidQ reads the whole expression on, as
        // parse_stacked_expression does.
        if e.is_none() || self.operand_follows() {
            let end = self.pos;
            self.pos = start;
            if let Some(stacked) = self.parse_stacked_expression() {
                return Some(stacked);
            }
            self.pos = end;
        }
        e
    }

    /// A parenthesised group's expression: infix only — operands side by
    /// side in it make the whole expression a stacked one
    /// (parse_expression).
    fn parse_group_expression(&mut self) -> Option<Expression> {
        if let Some(e) = self.try_postfix() {
            return Some(e);
        }
        self.parse_logical_or()
    }

    /// Whether the next token starts an operand: a number, a string, a
    /// name, or `(` (after a value that can't be called, such as a number).
    fn operand_follows(&self) -> bool {
        match self.peek_kind() {
            Some(TokenType::Number | TokenType::StringLit | TokenType::LParen) => true,
            Some(TokenType::Identifier) => !(self.peek_identifier_eq("SHL") || self.peek_identifier_eq("SHR") || self.peek_identifier_eq("INV")),
            _ => false,
        }
    }

    /// An expression as RapidQ's compiler (RC.EXE) reads one: operators and
    /// operands through an operator stack (a shunting-yard), operands side
    /// by side allowed — two with no operator between are both pushed, the
    /// parentheses only group, an operator still pending applies to what
    /// comes after — and the value is the operand stack's bottom; the
    /// others are still worked out (a FUNCTION among them is called).
    /// Checked against RC.EXE: `A B OR C` is A, `-9(COS(x))` is 9, `2(3)`
    /// is 2, `10 - 2 3` is 10, `2 * 3 + 4 5` is 6, `(2 3) + 1` is 2,
    /// `NOT 0 5` is 0, `"a" "b"` is "a", `1 Side(5)` is 1 and calls Side.
    /// Ends where RapidQ's would: a token that is neither an operand nor an
    /// operator (`,`, `;`, a keyword, the line's end), or a `)` it didn't
    /// open. `None` (nothing read) if it isn't one.
    fn parse_stacked_expression(&mut self) -> Option<Expression> {
        enum Op {
            Paren,
            Binary(BinaryOperator, u8),
            /// SHL / SHR / INV: calls of two arguments.
            Named(&'static str, u8),
            Unary(UnaryOperator, Option<TextSpan>, u8),
        }
        fn prec(op: &Op) -> u8 {
            match op {
                Op::Paren => 0,
                Op::Binary(_, p) | Op::Named(_, p) | Op::Unary(_, _, p) => *p,
            }
        }
        fn reduce(ops: &mut Vec<Op>, vals: &mut Vec<Expression>) -> Option<()> {
            match ops.pop()? {
                Op::Paren => None,
                Op::Binary(op, _) => {
                    let r = vals.pop()?;
                    let l = vals.pop()?;
                    vals.push(binary(l, op, r));
                    Some(())
                }
                Op::Named(name, _) => {
                    let r = vals.pop()?;
                    let l = vals.pop()?;
                    let span = TextSpan::new(expression_span(&l).start, expression_span(&r).end);
                    vals.push(call_expr(span, name, vec![l, r]));
                    Some(())
                }
                Op::Unary(operator, span, _) => {
                    let operand = vals.pop()?;
                    let start = span.map_or(expression_span(&operand).start, |s| s.start);
                    vals.push(Expression::Unary(UnaryExpression {
                        span: TextSpan::new(start, expression_span(&operand).end),
                        operator,
                        operand: Box::new(operand),
                    }));
                    Some(())
                }
            }
        }
        let mut ops: Vec<Op> = Vec::new();
        let mut vals: Vec<Expression> = Vec::new();
        let mut want_operand = true;
        loop {
            let kind = self.peek_kind();
            if want_operand {
                // (prefix operators: RapidR's precedence — NOT below the
                // comparisons, `-` `+` `@` above `^`)
                let unary = match kind {
                    Some(TokenType::Minus) => Some((UnaryOperator::Negate, 10)),
                    Some(TokenType::Plus) => Some((UnaryOperator::Positive, 10)),
                    Some(TokenType::At) => Some((UnaryOperator::Ref, 10)),
                    Some(TokenType::Not) => Some((UnaryOperator::Not, 3)),
                    _ => None,
                };
                if let Some((op, p)) = unary {
                    let span = self.advance().map(|t| t.span);
                    ops.push(Op::Unary(op, span, p));
                    continue;
                }
            }
            match kind {
                Some(TokenType::LParen) if want_operand || !vals.is_empty() => {
                    self.advance();
                    ops.push(Op::Paren);
                    want_operand = true;
                    continue;
                }
                Some(TokenType::RParen) if !want_operand && ops.iter().any(|o| matches!(o, Op::Paren)) => {
                    self.advance();
                    while !matches!(ops.last(), Some(Op::Paren)) {
                        reduce(&mut ops, &mut vals)?;
                    }
                    ops.pop();
                    continue;
                }
                _ => {}
            }
            if !want_operand {
                // A binary operator (the forms the infix parser reads).
                let binary_op = match (kind, self.peek_kind_at(1)) {
                    (Some(TokenType::Or), _) => Some((Op::Binary(BinaryOperator::Or, 1), 1)),
                    (Some(TokenType::Xor), _) => Some((Op::Binary(BinaryOperator::Xor, 1), 1)),
                    (Some(TokenType::And), _) => Some((Op::Binary(BinaryOperator::And, 2), 1)),
                    (Some(TokenType::Eq), _) => Some((Op::Binary(BinaryOperator::Equal, 4), 1)),
                    (Some(TokenType::Neq), _) => Some((Op::Binary(BinaryOperator::NotEqual, 4), 1)),
                    (Some(TokenType::Not), Some(TokenType::Eq)) => Some((Op::Binary(BinaryOperator::NotEqual, 4), 2)),
                    (Some(TokenType::Lt), Some(TokenType::Eq)) => Some((Op::Binary(BinaryOperator::LessThanOrEqual, 5), 2)),
                    (Some(TokenType::Gt), Some(TokenType::Eq)) => Some((Op::Binary(BinaryOperator::GreaterThanOrEqual, 5), 2)),
                    (Some(TokenType::Lt), Some(TokenType::Gt)) => Some((Op::Binary(BinaryOperator::NotEqual, 5), 2)),
                    (Some(TokenType::Lt), _) => Some((Op::Binary(BinaryOperator::LessThan, 5), 1)),
                    (Some(TokenType::Lte), _) => Some((Op::Binary(BinaryOperator::LessThanOrEqual, 5), 1)),
                    (Some(TokenType::Gt), _) => Some((Op::Binary(BinaryOperator::GreaterThan, 5), 1)),
                    (Some(TokenType::Gte), _) => Some((Op::Binary(BinaryOperator::GreaterThanOrEqual, 5), 1)),
                    (Some(TokenType::Plus), _) => Some((Op::Binary(BinaryOperator::Add, 6), 1)),
                    (Some(TokenType::Minus), _) => Some((Op::Binary(BinaryOperator::Subtract, 6), 1)),
                    (Some(TokenType::Ampersand), _) => Some((Op::Binary(BinaryOperator::Concat, 6), 1)),
                    (Some(TokenType::Mod), _) => Some((Op::Binary(BinaryOperator::Modulo, 7), 1)),
                    (Some(TokenType::Star), _) => Some((Op::Binary(BinaryOperator::Multiply, 8), 1)),
                    (Some(TokenType::Slash), _) => Some((Op::Binary(BinaryOperator::Divide, 8), 1)),
                    (Some(TokenType::Backslash), _) => Some((Op::Binary(BinaryOperator::IntegerDivide, 8), 1)),
                    (Some(TokenType::Caret), _) => Some((Op::Binary(BinaryOperator::Power, 9), 1)),
                    (Some(TokenType::Identifier), _) if self.peek_identifier_eq("INV") => Some((Op::Named("INV", 7), 1)),
                    (Some(TokenType::Identifier), _) if self.peek_identifier_eq("SHL") => Some((Op::Named("SHL", 8), 1)),
                    (Some(TokenType::Identifier), _) if self.peek_identifier_eq("SHR") => Some((Op::Named("SHR", 8), 1)),
                    _ => None,
                };
                if let Some((op, width)) = binary_op {
                    self.pos += width;
                    let p = prec(&op);
                    while ops.last().is_some_and(|top| !matches!(top, Op::Paren) && prec(top) >= p) {
                        reduce(&mut ops, &mut vals)?;
                    }
                    ops.push(op);
                    want_operand = true;
                    continue;
                }
            }
            // An operand (side by side with the one before, if no operator
            // came between).
            let starts_operand = match kind {
                Some(TokenType::Number | TokenType::StringLit | TokenType::Dot) => true,
                Some(TokenType::Identifier) => want_operand || self.operand_follows(),
                _ => want_operand && self.peek().is_some_and(|t| t.kind != TokenType::Identifier && self.is_keyword_name(&t.lexeme)),
            };
            if !starts_operand {
                break;
            }
            let operand = self.parse_postfix_expression()?;
            vals.push(operand);
            want_operand = false;
        }
        if want_operand {
            // A binary operator with nothing after it (`F("a"+, 5)`, `x = 2
            // +`): RC.EXE drops it — the value is what came before
            // (Qcdaudio.inc's `"status cdaudio position"+,` compiles so).
            if matches!(ops.last(), Some(Op::Binary(..) | Op::Named(..))) && !vals.is_empty() {
                ops.pop();
            } else {
                return None;
            }
        }
        while !ops.is_empty() {
            reduce(&mut ops, &mut vals)?;
        }
        let mut vals = vals.into_iter();
        let first = vals.next()?;
        // (the others, when one of them calls something: worked out too)
        let rest: Vec<Expression> = vals.collect();
        if !rest.iter().any(calls_something) {
            return Some(first);
        }
        let span = TextSpan::new(expression_span(&first).start, rest.last().map_or(expression_span(&first).end, |e| expression_span(e).end));
        let one = Expression::Literal(Literal { span, value: LiteralValue::Integer(1) });
        Some(rest.into_iter().rev().fold(None, |acc: Option<Expression>, e| {
            Some(match acc {
                None => e,
                Some(after) => call_expr(span, "IIF", vec![one.clone(), e, after]),
            })
        }).map_or(first.clone(), |others| call_expr(span, "IIF", vec![one, first, others])))
    }

    /// RapidQ's POSTFIX (RPN) expressions (manual, Appendix C): every operand
    /// and operator in its own parentheses — `(4) (7) (*) (4) (1) (-) (6)
    /// (^) (+)` is `4 * 7 + (4 - 1) ^ 6`. Parenthesised groups side by side
    /// are never an infix expression, so nothing else reads this way.
    fn try_postfix(&mut self) -> Option<Expression> {
        if self.peek_kind() != Some(TokenType::LParen) {
            return None;
        }
        let start = self.pos;
        // The groups, as (index of `(`, index of its `)`).
        let mut groups = Vec::new();
        let mut i = start;
        while self.tokens.get(i).map(|t| t.kind) == Some(TokenType::LParen) {
            let mut depth = 0usize;
            let mut j = i;
            loop {
                match self.tokens.get(j)?.kind {
                    TokenType::LParen => depth += 1,
                    TokenType::RParen => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    TokenType::Newline => return None,
                    _ => {}
                }
                j += 1;
            }
            groups.push((i, j));
            i = j + 1;
        }
        // (a group of one operator is an operator)
        let ops: Vec<Option<BinaryOperator>> = groups
            .iter()
            .map(|&(a, b)| {
                if b != a + 2 {
                    return None;
                }
                Some(match self.tokens[a + 1].kind {
                    TokenType::Plus => BinaryOperator::Add,
                    TokenType::Minus => BinaryOperator::Subtract,
                    TokenType::Star => BinaryOperator::Multiply,
                    TokenType::Slash => BinaryOperator::Divide,
                    TokenType::Backslash => BinaryOperator::IntegerDivide,
                    TokenType::Caret => BinaryOperator::Power,
                    TokenType::Ampersand => BinaryOperator::Concat,
                    TokenType::Mod => BinaryOperator::Modulo,
                    TokenType::And => BinaryOperator::And,
                    TokenType::Or => BinaryOperator::Or,
                    TokenType::Xor => BinaryOperator::Xor,
                    _ => return None,
                })
            })
            .collect();
        if groups.len() < 3 || ops.iter().all(Option::is_none) {
            return None;
        }
        let end = groups.last()?.1 + 1;
        let mut stack: Vec<Expression> = Vec::new();
        let mut ok = true;
        for (&(a, b), op) in groups.iter().zip(ops) {
            match op {
                Some(op) => match (stack.pop(), stack.pop()) {
                    (Some(r), Some(l)) => stack.push(binary(l, op, r)),
                    _ => {
                        ok = false;
                        break;
                    }
                },
                None => {
                    // (what's inside the group: `(4) (7)` mustn't read as an index)
                    self.pos = a + 1;
                    match self.parse_logical_or() {
                        Some(e) if self.pos == b => stack.push(e),
                        _ => {
                            ok = false;
                            break;
                        }
                    }
                }
            }
        }
        if !ok || stack.len() != 1 {
            self.pos = start;
            return None;
        }
        self.pos = end;
        stack.pop()
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

    /// A comparison's operand: arithmetic and concatenation, no comparison
    /// or logic (`CASE IS = "l" AND …`'s "l").
    fn parse_comparison_operand(&mut self) -> Option<Expression> {
        self.parse_term()
    }

    fn parse_comparison(&mut self) -> Option<Expression> {
        let mut expr = self.parse_term()?;
        loop {
            // `a NOT > b`: RapidQ's NOT binds looser than a comparison
            // (manual, Appendix C), so it's `NOT (a > b)` — as `a NOT= b`.
            let negate = self.peek_kind() == Some(TokenType::Not) && matches!(self.peek_kind_at(1), Some(TokenType::Lt | TokenType::Lte | TokenType::Gt | TokenType::Gte));
            if negate {
                self.advance();
            }
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
            if negate {
                expr = Expression::Unary(UnaryExpression { span: expression_span(&expr), operator: UnaryOperator::Not, operand: Box::new(expr) });
            }
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
            // (a number or a string can't be called: `9(COS(x))` is two
            // operands side by side — parse_stacked_expression)
            if self.peek_kind() == Some(TokenType::LParen) && matches!(expr, Expression::Literal(_)) {
                break;
            }
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
                if !self.member_follows() {
                    let (object, path) = self.member_path(&expr);
                    self.error_at(self.pos - 1, format!("Member {path} not part of class {object}"));
                    return None;
                }
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
    /// `COLOR , 1`, `LOCATE , 5`, a last one after a trailing comma
    /// (`AddItems " ",`) — the callee then uses its default.
    fn parse_argument(&mut self) -> Option<Expression> {
        if self.at_eol() || matches!(self.peek_kind(), Some(TokenType::Comma | TokenType::RParen | TokenType::Colon | TokenType::Else)) {
            let span = self.peek().map(|t| t.span).unwrap_or_default();
            return Some(Expression::Identifier(Identifier { span, name: OMITTED_ARGUMENT.to_string() }));
        }
        // `ByVal 0&` in a call (Visual Basic's way to pass a value to an
        // API function): the value.
        if matches!(self.peek_kind(), Some(TokenType::ByVal | TokenType::ByRef)) && !matches!(self.peek_kind_at(1), Some(TokenType::Comma | TokenType::RParen)) {
            self.advance();
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
        // A parameter a keyword names, read in its routine.
        if let Some(t) = self.peek().filter(|t| t.kind != TokenType::Identifier && self.is_keyword_name(&t.lexeme)) {
            let tok = t.clone();
            self.advance();
            return Some(Expression::Identifier(Identifier { span: tok.span, name: tok.lexeme.clone() }));
        }
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
                if !self.member_follows() {
                    let (object, path) = self.with_objects.last().cloned().unwrap_or_default();
                    self.error_at(self.pos - 1, format!("Member {path} not part of class {object}"));
                    return None;
                }
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
                let expr = self.parse_group_expression()?;
                self.expect(TokenType::RParen)?;
                Some(expr)
            }
            _ => None,
        }
    }
}

/// A SUB or FUNCTION written without its parameters (`FUNCTION WinProc`)
/// has the ones it was DECLAREd with — `DECLARE FUNCTION G (a AS INTEGER)
/// AS INTEGER` then `FUNCTION G` reads `a`: RC.EXE runs it so (G(4) is 8),
/// and RapidQ's MinToTaskbar.bas is written so.
fn declared_parameters(body: &mut [Statement]) {
    let declared: Vec<(String, Vec<rapidr_ast::Parameter>, Option<String>)> = body
        .iter()
        .filter_map(|s| match s {
            Statement::Declare(d) if d.lib.is_none() && !d.params.is_empty() => Some((d.name.to_ascii_lowercase(), d.params.clone(), d.return_type.clone())),
            _ => None,
        })
        .collect();
    if declared.is_empty() {
        return;
    }
    let find = |name: &str| declared.iter().find(|(n, _, _)| n.eq_ignore_ascii_case(name));
    for s in body.iter_mut() {
        match s {
            Statement::Subroutine(r) if r.params.is_empty() => {
                if let Some((_, params, _)) = find(&r.name) {
                    r.params = params.clone();
                }
            }
            Statement::Function(f) if f.params.is_empty() => {
                if let Some((_, params, ret)) = find(&f.name) {
                    f.params = params.clone();
                    if f.return_type.is_none() {
                        f.return_type = ret.clone();
                    }
                }
            }
            _ => {}
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

/// A hex number as RapidQ reads it: a 32-bit integer, &H80000000 to
/// &HFFFFFFFF negative (RC.EXE: `&H80000001 SHL 1` is 2), and past 8 digits
/// its low 32 bits (RC.EXE: `&H123456789` is 591751049).
fn hex_32(hex: &str) -> Option<i64> {
    let mut n = 0u32;
    for c in hex.chars() {
        n = (n << 4) | c.to_digit(16)?;
    }
    Some(i64::from(n as i32))
}

fn parse_number_literal(lexeme: &str) -> LiteralValue {
    if let Some(hex) = lexeme.strip_prefix("0x") {
        hex_32(hex).map(LiteralValue::Integer).unwrap_or_else(|| LiteralValue::String(lexeme.to_string()))
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

/// RapidQ's `INPUT$(n)`: waits for n keys and returns them (not echoed) —
/// a FUNCTION over INKEY$ added to the program. Between keys it sleeps in
/// RAPIDR__WAITKEY until the next key arrives (a terminal's poll, the
/// window's event loop, the page's keydown), so it returns the moment the
/// n-th key is pressed.
fn input_chars(body: &mut Vec<Statement>) {
    const NAME: &str = "RAPIDR__INPUTCHARS";
    const SRC: &str = "FUNCTION RAPIDR__INPUTCHARS(RAPIDR__n AS LONG) AS STRING\n\
        DIM RAPIDR__s AS STRING, RAPIDR__k AS STRING, RAPIDR__more AS LONG\n\
        RAPIDR__more = 1\n\
        WHILE LEN(RAPIDR__s) < RAPIDR__n AND RAPIDR__more\n\
        RAPIDR__k = INKEY$\n\
        IF RAPIDR__k = \"\" THEN RAPIDR__more = RAPIDR__WAITKEY() ELSE RAPIDR__s = RAPIDR__s + RAPIDR__k\n\
        WEND\n\
        RAPIDR__INPUTCHARS = LEFT$(RAPIDR__s, RAPIDR__n)\n\
        END FUNCTION\n";
    let is_input = |e: &Expression| matches!(e, Expression::Identifier(i) if i.name.eq_ignore_ascii_case("INPUT$"));
    let own = body.iter().any(|s| matches!(s, Statement::Function(f) if f.name.eq_ignore_ascii_case("INPUT$")));
    let mut used = false;
    if !own {
        walk_expressions_mut(body, true, &mut |e| {
            if let Expression::FunctionCall(c) = e {
                if c.args.len() == 1 && is_input(&c.callee) {
                    let span = expression_span(&c.callee);
                    *c.callee = ident(span, NAME);
                    used = true;
                }
            }
        });
    }
    if used {
        if let Ok(tokens) = rapidr_lexer::Lexer::new(SRC, None).tokenize() {
            let mut p = Parser::new(&tokens);
            body.extend(p.parse_program().statements);
        }
    }
}

fn ident(span: TextSpan, name: &str) -> Expression {
    Expression::Identifier(Identifier { span, name: name.to_string() })
}

fn call_expr(span: TextSpan, name: &str, args: Vec<Expression>) -> Expression {
    Expression::FunctionCall(FunctionCallExpression { span, callee: Box::new(ident(span, name)), args })
}

/// Whether working `e` out may call something (a FUNCTION, a method, a
/// built-in) — an operand RapidQ drops is still worked out then.
fn calls_something(e: &Expression) -> bool {
    match e {
        Expression::FunctionCall(_) | Expression::MethodCall(_) | Expression::ArrayAccess(_) => true,
        Expression::Binary(b) => calls_something(&b.left) || calls_something(&b.right),
        Expression::Unary(u) => calls_something(&u.operand),
        Expression::MemberAccess(m) => calls_something(&m.object),
        Expression::Identifier(_) | Expression::Literal(_) => false,
    }
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

/// The length in `STRING * 20` (a positive literal; anything else leaves the string unbounded).
fn literal_length(e: &Expression) -> Option<usize> {
    match e {
        // (`STRING * 0` too: RapidQ's holds nothing)
        Expression::Literal(Literal { value: LiteralValue::Integer(n), .. }) if *n >= 0 => Some(*n as usize),
        _ => None,
    }
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
        // (`x = 1 y = 2` isn't: RapidQ reads `1 y = 2` as one expression,
        // its operands side by side — operands_side_by_side below)
        let errs = errors("x = 1 THEN y = 2\n");
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert_eq!((errs[0].0, errs[0].1), (1, 7));
        assert!(errs[0].2.contains("Expected end-of-line but got THEN"), "{errs:?}");
    }

    #[test]
    fn operands_side_by_side() {
        // RC.EXE: the operand stack's bottom (`A B OR C` is A) …
        let stmts = parse("x = A B OR C\n");
        assert!(matches!(&stmts[0], Statement::Assignment(a) if matches!(&a.value, Expression::Identifier(i) if i.name == "A")), "{stmts:?}");
        // … the others still worked out when they call something (`-9(COS(t))`
        // is 9, COS called: IIF(1, 9, -COS(t)))
        let stmts = parse("y = -9(COS(t))\n");
        let Statement::Assignment(a) = &stmts[0] else { panic!("{stmts:?}") };
        let Expression::FunctionCall(c) = &a.value else { panic!("{a:?}") };
        assert!(matches!(c.callee.as_ref(), Expression::Identifier(i) if i.name == "IIF"));
        assert!(matches!(&c.args[1], Expression::Literal(Literal { value: LiteralValue::Integer(9), .. })), "{c:?}");
        assert!(matches!(&c.args[2], Expression::Unary(u) if u.operator == UnaryOperator::Negate), "{c:?}");
        // (parentheses only group: `(2 3) + 1` is 2)
        let stmts = parse("z = (2 3) + 1\n");
        assert!(matches!(&stmts[0], Statement::Assignment(a) if matches!(&a.value, Expression::Literal(Literal { value: LiteralValue::Integer(2), .. }))), "{stmts:?}");
        // (an operator with nothing after it is dropped, as RC.EXE does:
        // `w = 1 +` is 1 — tests/conformance/cases/dangling_operator.bas)
        let stmts = parse("w = 1 +\n");
        assert!(matches!(&stmts[0], Statement::Assignment(a) if matches!(&a.value, Expression::Literal(Literal { value: LiteralValue::Integer(1), .. }))), "{stmts:?}");
        assert_eq!(errors("w = 1 + -\n").len(), 1);
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
    fn fixed_length_strings_and_unclosed_with_and_on_error() {
        let stmts = parse("DIM s AS STRING * 8, u AS STRING * 0, n AS INTEGER\nTYPE R\n  Name AS STRING * 5\nEND TYPE\n");
        let lens: Vec<Option<usize>> = stmts.iter().filter_map(|s| if let Statement::Dim(d) = s { Some(d.fixed_len) } else { None }).collect();
        assert_eq!(lens, vec![Some(8), Some(0), None]);
        let field_len = stmts.iter().find_map(|s| if let Statement::Type(t) = s { t.fields.first().map(|f| f.fixed_len) } else { None });
        assert_eq!(field_len, Some(Some(5)));
        // WITH closed by END SUB; ON ERROR accepted.
        let stmts = parse("SUB A\n  ON ERROR RESUME NEXT\n  WITH x\n    .Y = 1\nEND SUB\nON ERROR GOTO 0\n");
        assert!(matches!(stmts[0], Statement::Subroutine(_)));
        assert!(matches!(stmts[1], Statement::Comment(_)));
    }

    #[test]
    fn routines_differing_only_by_suffix_get_their_own_names() {
        let names = |code: &str| -> Vec<String> {
            let program = rapidr_ast::Program { span: rapidr_diagnostics::TextSpan::new(0, 0), statements: parse(code) };
            rapidr_ast::suffix_routines::lower(&program)
                .statements
                .iter()
                .filter_map(|s| if let Statement::Function(f) = s { Some(f.name.clone()) } else { None })
                .collect()
        };
        let pair = "FUNCTION Day$ (k) AS STRING\nDay$ = \"x\"\nEND FUNCTION\nFUNCTION Day (k) AS INTEGER\nDay = 1\nEND FUNCTION\nPRINT Day$(1), Day(2)\n";
        assert_eq!(names(pair), ["day__str", "Day"]);
        assert_eq!(names("FUNCTION Name$ (k) AS STRING\nName$ = \"x\"\nEND FUNCTION\n"), ["Name$"]);
    }

    #[test]
    fn parses_directives_and_dim_statements() {
        // RapidQ manual: in `DIM x, y AS INTEGER` only y is INTEGER (RC.EXE 2006
        // refuses the line; a DIM without AS is a DOUBLE, as RC.EXE has `DIM m`).
        let stmts = parse("$APPTYPE GUI\nDIM x, y AS INTEGER\nDIM (a, b)(3) AS STRING, n AS LONG = 4\nDEFINT i = 1, j(2) = {7, 8, 9}\n");
        assert!(matches!(stmts[0], Statement::Directive(_)));
        let dims: Vec<(String, String, usize)> = stmts
            .iter()
            .filter_map(|s| match s {
                Statement::Dim(d) => Some((d.declarators[0].name.clone(), d.type_name.clone(), d.declarators[0].dimensions.len())),
                _ => None,
            })
            .collect();
        let expect = [("x", "DOUBLE", 0), ("y", "INTEGER", 0), ("a", "STRING", 1), ("b", "STRING", 1), ("n", "LONG", 0), ("i", "INTEGER", 0), ("j", "INTEGER", 1)];
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
        // (`CASE 1 2` isn't one: RapidQ reads it as the list 1)
        let errs = errors("SELECT CASE n\n  CASE 1 )\n    x = 1\nEND SELECT\n");
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert_eq!(errs[0].0, 2, "{errs:?}");
    }
}