use std::path::Path;

use rapidr_ast::*;
use rapidr_diagnostics::TextSpan;
use rapidr_lexer::{lex_file, LexError, Token, TokenType};

pub fn parse_file(path: impl AsRef<Path>) -> Result<Program, LexError> {
    let tokens = lex_file(path)?;
    Ok(parse_tokens(&tokens))
}

pub fn parse_tokens(tokens: &[Token]) -> Program {
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}

// ---------------------------------------------------------------------------
// Parser — recursive descent over the full token stream
// ---------------------------------------------------------------------------

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
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

    fn consume_eol(&mut self) {
        while self.match_kind(TokenType::Newline) {}
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
        let body = self.parse_body(&[]);
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
            if let Some(stmt) = self.parse_statement() {
                stmts.push(stmt);
                // Handle colon-separated statements on the same line
                while self.match_kind(TokenType::Colon) {
                    if self.at_end() || self.peek_kind() == Some(TokenType::Newline) {
                        break;
                    }
                    if let Some(s) = self.parse_statement() {
                        stmts.push(s);
                    } else {
                        break;
                    }
                }
            } else {
                // Skip the rest of this line to recover
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

    fn parse_statement(&mut self) -> Option<Statement> {
        match self.peek_kind()? {
            TokenType::Directive => self.parse_directive(),
            TokenType::Dim => self.parse_dim().map(Statement::Dim),
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
            _ => self.parse_assignment_or_call(),
        }
    }

    // -----------------------------------------------------------------------
    // Simple statements
    // -----------------------------------------------------------------------

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

    fn parse_dim(&mut self) -> Option<DimStatement> {
        let start = self.pos;
        self.expect(TokenType::Dim)?;
        let mut declarators = Vec::new();
        loop {
            let name_tok = self.expect(TokenType::Identifier)?;
            let dimensions = if self.match_kind(TokenType::LParen) {
                let dims = self.parse_array_dimensions()?;
                self.expect(TokenType::RParen)?;
                dims
            } else {
                Vec::new()
            };
            let decl_end = self.previous()?.span.end;
            declarators.push(VariableDeclarator {
                span: TextSpan::new(name_tok.span.start, decl_end),
                name: name_tok.lexeme.clone(),
                dimensions,
            });
            if !self.match_kind(TokenType::Comma) {
                break;
            }
        }
        self.expect(TokenType::As)?;
        let type_name = self.advance()?.lexeme.clone();
        Some(DimStatement {
            span: self.span_from(start),
            declarators,
            type_name,
        })
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
        let mut append_newline = true;
        if !self.at_eol() {
            loop {
                items.push(self.parse_expression()?);
                if self.match_kind(TokenType::Comma) {
                    append_newline = true;
                    if self.at_eol() {
                        break;
                    }
                    continue;
                }
                if self.match_kind(TokenType::Semi) {
                    append_newline = false;
                    if self.at_eol() {
                        break;
                    }
                    continue;
                }
                break;
            }
        }
        Some(PrintStatement {
            span: self.span_from(start),
            items,
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
        let exit_type = self.advance()?.lexeme.to_uppercase();
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

    fn parse_declare(&mut self) -> Option<DeclareStatement> {
        let start = self.pos;
        self.advance()?; // DECLARE
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
        let name = self.expect(TokenType::Identifier)?.lexeme.clone();
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

        if self.match_kind(TokenType::Eq) {
            let value = self.parse_expression()?;
            return Some(Statement::Assignment(AssignmentStatement {
                span: self.span_from(start),
                target: left,
                value,
            }));
        }

        if self.at_eol() {
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
                self.consume_eol();
                case_else = self.parse_body(&[
                    Terminator::Keyword("CASE"),
                    Terminator::EndPair("SELECT"),
                ]);
                continue;
            }

            // CASE value1, value2
            let mut values = vec![self.parse_expression()?];
            while self.match_kind(TokenType::Comma) {
                values.push(self.parse_expression()?);
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

    fn parse_sub(&mut self) -> Option<SubroutineStatement> {
        let start = self.pos;
        self.expect(TokenType::Sub)?;
        let name = self.expect(TokenType::Identifier)?.lexeme.clone();
        let params = if self.match_kind(TokenType::LParen) {
            let p = self.parse_parameter_list()?;
            self.expect(TokenType::RParen);
            p
        } else {
            Vec::new()
        };
        self.consume_eol();
        let body = self.parse_body(&[Terminator::EndPair("SUB")]);
        self.expect(TokenType::End);
        self.expect(TokenType::Sub);
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
        let name = self.expect(TokenType::Identifier)?.lexeme.clone();
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
        let body = self.parse_body(&[Terminator::EndPair("FUNCTION")]);
        self.expect(TokenType::End);
        self.expect(TokenType::Function);
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
        let extends = if self.match_kind(TokenType::Extends) {
            Some(self.expect(TokenType::Identifier)?.lexeme.clone())
        } else {
            None
        };
        self.consume_eol();

        let mut fields = Vec::new();
        let mut methods = Vec::new();
        let mut constructor = Vec::new();

        loop {
            self.skip_newlines();
            if self.peek_is_end_followed_by("TYPE") || self.at_end() {
                break;
            }
            // Skip PRIVATE: / PUBLIC: labels
            if self.peek_identifier_eq("PRIVATE") || self.peek_identifier_eq("PUBLIC") {
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
            // Property
            if self.peek_kind() == Some(TokenType::Property) {
                // skip PROPERTY SET / GET lines
                self.skip_to_eol();
                continue;
            }
            // Field: name[(dims)] AS Type
            if self.peek_kind() == Some(TokenType::Identifier) {
                let field_start = self.pos;
                let fname = self.advance()?.lexeme.clone();
                let arr = if self.match_kind(TokenType::LParen) {
                    let expr = self.parse_expression()?;
                    self.expect(TokenType::RParen);
                    Some(expr)
                } else {
                    None
                };
                self.expect(TokenType::As)?;
                let ftype = self.advance()?.lexeme.clone();
                // optional PROPERTY SET
                if self.peek_kind() == Some(TokenType::Property) {
                    self.skip_to_eol();
                }
                fields.push(TypeField {
                    span: self.span_from(field_start),
                    name: fname,
                    type_name: ftype,
                    array_size: arr,
                });
                self.consume_eol();
                continue;
            }
            // Unknown line inside TYPE — skip
            self.skip_to_eol();
            self.consume_eol();
        }

        self.expect(TokenType::End);
        self.expect(TokenType::Type);

        Some(TypeStatement {
            span: self.span_from(start),
            name,
            extends,
            fields,
            methods,
            constructor,
        })
    }

    fn parse_create(&mut self) -> Option<CreateStatement> {
        let start = self.pos;
        self.expect(TokenType::Create)?;
        let name = self.expect(TokenType::Identifier)?.lexeme.clone();
        self.expect(TokenType::As)?;
        let type_name = self.expect(TokenType::Identifier)?.lexeme.clone();
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
            let pname = self.expect(TokenType::Identifier)?.lexeme.clone();
            let ptype = if self.match_kind(TokenType::As) {
                self.advance()?.lexeme.clone()
            } else {
                "VARIANT".to_string()
            };
            params.push(Parameter {
                span: self.span_from(p_start),
                name: pname,
                type_name: ptype,
                by_ref,
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
        let mut expr = self.parse_equality()?;
        while self.match_kind(TokenType::And) {
            let right = self.parse_equality()?;
            expr = binary(expr, BinaryOperator::And, right);
        }
        Some(expr)
    }

    fn parse_equality(&mut self) -> Option<Expression> {
        let mut expr = self.parse_comparison()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenType::Eq) => BinaryOperator::Equal,
                Some(TokenType::Neq) => BinaryOperator::NotEqual,
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
            let op = match self.peek_kind() {
                Some(TokenType::Lt) => BinaryOperator::LessThan,
                Some(TokenType::Lte) => BinaryOperator::LessThanOrEqual,
                Some(TokenType::Gt) => BinaryOperator::GreaterThan,
                Some(TokenType::Gte) => BinaryOperator::GreaterThanOrEqual,
                _ => break,
            };
            self.advance();
            let right = self.parse_term()?;
            expr = binary(expr, op, right);
        }
        Some(expr)
    }

    fn parse_term(&mut self) -> Option<Expression> {
        let mut expr = self.parse_factor()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenType::Plus) => BinaryOperator::Add,
                Some(TokenType::Minus) => BinaryOperator::Subtract,
                Some(TokenType::Ampersand) => BinaryOperator::Concat,
                _ => break,
            };
            self.advance();
            let right = self.parse_factor()?;
            expr = binary(expr, op, right);
        }
        Some(expr)
    }

    fn parse_factor(&mut self) -> Option<Expression> {
        let mut expr = self.parse_power()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenType::Star) => BinaryOperator::Multiply,
                Some(TokenType::Slash) => BinaryOperator::Divide,
                Some(TokenType::Backslash) => BinaryOperator::IntegerDivide,
                Some(TokenType::Mod) => BinaryOperator::Modulo,
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
            if self.match_kind(TokenType::Dot) {
                let member = self.expect(TokenType::Identifier)?;
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
            args.push(self.parse_expression()?);
            if self.match_kind(TokenType::Comma) {
                continue;
            }
            self.expect(TokenType::RParen)?;
            break;
        }
        Some(args)
    }

    fn parse_argument_list_without_parens(&mut self) -> Option<Vec<Expression>> {
        let mut args = vec![self.parse_expression()?];
        while self.match_kind(TokenType::Comma) {
            args.push(self.parse_expression()?);
        }
        Some(args)
    }

    fn parse_primary(&mut self) -> Option<Expression> {
        match self.peek_kind()? {
            // WITH-dot access: `.Property`
            TokenType::Dot => {
                let dot_tok = self.advance()?;
                let member = self.expect(TokenType::Identifier)?;
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

#[cfg(test)]
mod tests {
    use rapidr_ast::*;
    use rapidr_lexer::Lexer;

    use super::parse_tokens;

    fn parse(code: &str) -> Vec<Statement> {
        let tokens = Lexer::new(code, None).tokenize().unwrap();
        parse_tokens(&tokens).statements
    }

    #[test]
    fn parses_directives_and_dim_statements() {
        let stmts = parse("$APPTYPE GUI\nDIM x, y AS INTEGER\n");
        assert!(matches!(stmts[0], Statement::Directive(_)));
        match &stmts[1] {
            Statement::Dim(dim) => {
                assert_eq!(dim.declarators.len(), 2);
                assert_eq!(dim.type_name, "INTEGER");
            }
            other => panic!("expected dim, got {other:?}"),
        }
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
}