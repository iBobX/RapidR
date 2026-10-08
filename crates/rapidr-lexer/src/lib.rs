use std::error::Error;
use std::fmt;

use rapidr_diagnostics::{Diagnostic, SourceLocation, TextSpan};

pub mod lossless;
pub use lossless::{lex_lossless, LineClass, LosslessFile, Piece, Trivia, TriviaKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenType {
    Dim,
    As,
    Integer,
    String,
    Double,
    Single,
    Byte,
    Word,
    Dword,
    Long,
    Int64,
    Currency,
    RObject,
    Variant,
    If,
    Then,
    Else,
    ElseIf,
    End,
    For,
    To,
    Step,
    Next,
    While,
    Wend,
    Do,
    Loop,
    Until,
    Select,
    Case,
    Sub,
    Function,
    Call,
    Return,
    Exit,
    Print,
    Input,
    Goto,
    Gosub,
    Import,
    Create,
    Const,
    Type,
    Declare,
    Lib,
    Alias,
    With,
    Directive,
    Plus,
    Minus,
    Star,
    Slash,
    Backslash,
    Caret,
    Mod,
    Ampersand,
    DefStr,
    DefInt,
    DefByte,
    DefWord,
    DefDword,
    DefLong,
    DefShort,
    /// `DATA …`: the raw text after DATA (items are split by the parser)
    Data,
    /// `@var`: pass by reference (RapidQ manual 3.5)
    At,
    /// `[` / `]` for RapidQ string indexing (`s$[2]`)
    LBracket,
    RBracket,
    /// `{` / `}` around array initializers (`DEFINT a(2) = {1, 2, 3}`)
    LBrace,
    RBrace,
    DefSng,
    DefDbl,
    DefCur,
    Extends,
    Property,
    Set,
    ByVal,
    ByRef,
    Bind,
    Constructor,
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    And,
    Or,
    Not,
    Xor,
    Hash,
    Open,
    Close,
    Write,
    Seek,
    Kill,
    RustStart,
    RustEnd,
    /// Text the lexer can't read (an unexpected character, `&H` without
    /// digits): only from [`Lexer::tokenize_recovering`], never in the
    /// tokens of a program that compiles.
    Error,
    LParen,
    RParen,
    Comma,
    Colon,
    Semi,
    Dot,
    Identifier,
    Number,
    StringLit,
    Newline,
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenType,
    pub lexeme: String,
    pub span: TextSpan,
    pub line: usize,
    pub column: usize,
    pub trailing: Option<String>,
}

impl Token {
    fn new(kind: TokenType, lexeme: String, span: TextSpan, line: usize, column: usize) -> Self {
        Self {
            kind,
            lexeme,
            span,
            line,
            column,
            trailing: None,
        }
    }

    fn with_trailing(mut self, trailing: Option<String>) -> Self {
        self.trailing = trailing;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub diagnostic: Diagnostic,
}

impl LexError {
    fn new(
        message: impl Into<String>,
        span: TextSpan,
        line: usize,
        column: usize,
        file_path: Option<String>,
    ) -> Self {
        Self {
            diagnostic: Diagnostic::error(
                message,
                span,
                SourceLocation::new(line, column),
                file_path,
            ),
        }
    }
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.diagnostic.fmt(f)
    }
}

impl Error for LexError {}

pub fn lex_file(path: impl AsRef<std::path::Path>) -> Result<Vec<Token>, LexError> {
    let path = path.as_ref();
    let preprocessed = rapidr_preprocessor::preprocess_file(
        path,
        rapidr_preprocessor::PreprocessOptions::default(),
    )
    .map_err(|error| LexError {
        diagnostic: error.diagnostic,
    })?;

    Lexer::new(&preprocessed.source, Some(path.display().to_string())).tokenize()
}

pub struct Lexer<'src> {
    source: &'src str,
    file_path: Option<String>,
    index: usize,
    line: usize,
    column: usize,
    /// `$ESCAPECHARS ON`: `\n`, `\t`, `\"`, `\65`, `\x41`, … in strings.
    escape_chars: bool,
    /// Errors so far ([`Lexer::tokenize_recovering`]).
    errors: Vec<LexError>,
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str, file_path: Option<String>) -> Self {
        Self {
            source,
            file_path,
            index: 0,
            line: 1,
            column: 1,
            escape_chars: false,
            errors: Vec::new(),
        }
    }

    /// The tokens, or the first lexical error (what compilers use: a
    /// program with an error doesn't build).
    pub fn tokenize(self) -> Result<Vec<Token>, LexError> {
        let (tokens, mut errors) = self.tokenize_recovering();
        if errors.is_empty() {
            Ok(tokens)
        } else {
            Err(errors.remove(0))
        }
    }

    /// Every token and every lexical error: an error becomes a
    /// [`TokenType::Error`] token (or, for a string left open at the end of
    /// the file, the string as far as it goes) and lexing goes on — what
    /// tools (editors) use. Up to the first error the tokens are
    /// [`Lexer::tokenize`]'s.
    pub fn tokenize_recovering(mut self) -> (Vec<Token>, Vec<LexError>) {
        let mut tokens = Vec::new();

        while !self.is_at_end() {
            if self.try_consume_line_continuation() {
                continue;
            }

            let start = self.index;
            let line = self.line;
            let column = self.column;
            let current = self.current_char().unwrap();

            match current {
                ' ' | '\t' => {
                    self.advance_char();
                }
                '\r' | '\n' => {
                    self.consume_newline();
                    tokens.push(Token::new(
                        TokenType::Newline,
                        "\n".to_string(),
                        TextSpan::new(start, self.index),
                        line,
                        column,
                    ));
                }
                '\'' => {
                    self.consume_comment();
                }
                '$' => {
                    tokens.push(self.lex_directive(start, line, column));
                }
                '"' => {
                    tokens.push(self.lex_string(start, line, column));
                }
                '&' => {
                    if self.is_prefixed_number() {
                        tokens.push(self.lex_prefixed_number(start, line, column));
                    } else {
                        self.advance_char();
                        tokens.push(Token::new(
                            TokenType::Ampersand,
                            "&".to_string(),
                            TextSpan::new(start, self.index),
                            line,
                            column,
                        ));
                    }
                }
                '0'..='9' if self.digits_start_a_name() => {
                    tokens.push(self.lex_identifier(start, line, column));
                }
                '0'..='9' => {
                    tokens.push(self.lex_decimal_number(start, line, column));
                }
                '@' => {
                    self.advance_char();
                    tokens.push(Token::new(TokenType::At, "@".to_string(), TextSpan::new(start, self.index), line, column));
                }
                '{' | '}' | '[' | ']' => {
                    let brace = self.current_char().unwrap_or('{');
                    let kind = match brace {
                        '{' => TokenType::LBrace,
                        '}' => TokenType::RBrace,
                        '[' => TokenType::LBracket,
                        _ => TokenType::RBracket,
                    };
                    self.advance_char();
                    tokens.push(Token::new(
                        kind,
                        brace.to_string(),
                        TextSpan::new(start, self.index),
                        line,
                        column,
                    ));
                }
                '(' => {
                    self.advance_char();
                    tokens.push(Token::new(
                        TokenType::LParen,
                        "(".to_string(),
                        TextSpan::new(start, self.index),
                        line,
                        column,
                    ));
                }
                ')' => {
                    self.advance_char();
                    tokens.push(Token::new(
                        TokenType::RParen,
                        ")".to_string(),
                        TextSpan::new(start, self.index),
                        line,
                        column,
                    ));
                }
                ',' => {
                    self.advance_char();
                    tokens.push(Token::new(
                        TokenType::Comma,
                        ",".to_string(),
                        TextSpan::new(start, self.index),
                        line,
                        column,
                    ));
                }
                ':' => {
                    self.advance_char();
                    tokens.push(Token::new(
                        TokenType::Colon,
                        ":".to_string(),
                        TextSpan::new(start, self.index),
                        line,
                        column,
                    ));
                }
                // `?` is BASIC shorthand for PRINT.
                '?' => {
                    self.advance_char();
                    tokens.push(Token::new(
                        TokenType::Print,
                        "?".to_string(),
                        TextSpan::new(start, self.index),
                        line,
                        column,
                    ));
                }
                ';' => {
                    self.advance_char();
                    tokens.push(Token::new(
                        TokenType::Semi,
                        ";".to_string(),
                        TextSpan::new(start, self.index),
                        line,
                        column,
                    ));
                }
                // `.5` is a number (`SetRGBA(.1, 1, .1, .7)`), not a WITH
                // member, unless it follows a value (`a.5` stays a member).
                '.' if matches!(self.peek_char(1), Some('0'..='9'))
                    && !tokens.last().is_some_and(|t| {
                        matches!(t.kind, TokenType::Identifier | TokenType::RParen | TokenType::RBracket | TokenType::Number | TokenType::String)
                    }) =>
                {
                    tokens.push(self.lex_decimal_number(start, line, column));
                }
                '.' => {
                    self.advance_char();
                    tokens.push(Token::new(
                        TokenType::Dot,
                        ".".to_string(),
                        TextSpan::new(start, self.index),
                        line,
                        column,
                    ));
                }
                '+' => {
                    self.advance_char();
                    tokens.push(Token::new(
                        TokenType::Plus,
                        "+".to_string(),
                        TextSpan::new(start, self.index),
                        line,
                        column,
                    ));
                }
                '-' => {
                    self.advance_char();
                    tokens.push(Token::new(
                        TokenType::Minus,
                        "-".to_string(),
                        TextSpan::new(start, self.index),
                        line,
                        column,
                    ));
                }
                '*' => {
                    self.advance_char();
                    tokens.push(Token::new(
                        TokenType::Star,
                        "*".to_string(),
                        TextSpan::new(start, self.index),
                        line,
                        column,
                    ));
                }
                '/' => {
                    self.advance_char();
                    tokens.push(Token::new(
                        TokenType::Slash,
                        "/".to_string(),
                        TextSpan::new(start, self.index),
                        line,
                        column,
                    ));
                }
                '\\' => {
                    self.advance_char();
                    tokens.push(Token::new(
                        TokenType::Backslash,
                        "\\".to_string(),
                        TextSpan::new(start, self.index),
                        line,
                        column,
                    ));
                }
                '^' => {
                    self.advance_char();
                    tokens.push(Token::new(
                        TokenType::Caret,
                        "^".to_string(),
                        TextSpan::new(start, self.index),
                        line,
                        column,
                    ));
                }
                '<' => {
                    self.advance_char();
                    let token = if self.match_char('>') {
                        Token::new(
                            TokenType::Neq,
                            "<>".to_string(),
                            TextSpan::new(start, self.index),
                            line,
                            column,
                        )
                    } else if self.match_char('=') {
                        Token::new(
                            TokenType::Lte,
                            "<=".to_string(),
                            TextSpan::new(start, self.index),
                            line,
                            column,
                        )
                    } else {
                        Token::new(
                            TokenType::Lt,
                            "<".to_string(),
                            TextSpan::new(start, self.index),
                            line,
                            column,
                        )
                    };
                    tokens.push(token);
                }
                '>' => {
                    self.advance_char();
                    let token = if self.match_char('=') {
                        Token::new(
                            TokenType::Gte,
                            ">=".to_string(),
                            TextSpan::new(start, self.index),
                            line,
                            column,
                        )
                    } else {
                        Token::new(
                            TokenType::Gt,
                            ">".to_string(),
                            TextSpan::new(start, self.index),
                            line,
                            column,
                        )
                    };
                    tokens.push(token);
                }
                '=' => {
                    self.advance_char();
                    // `=>` and `=<` are `>=` and `<=` (RapidQ accepts both orders).
                    let (kind, text) = if self.match_char('>') {
                        (TokenType::Gte, ">=")
                    } else if self.match_char('<') {
                        (TokenType::Lte, "<=")
                    } else {
                        (TokenType::Eq, "=")
                    };
                    tokens.push(Token::new(kind, text.to_string(), TextSpan::new(start, self.index), line, column));
                }
                '#' => {
                    self.advance_char();
                    tokens.push(Token::new(
                        TokenType::Hash,
                        "#".to_string(),
                        TextSpan::new(start, self.index),
                        line,
                        column,
                    ));
                }
                c if Self::is_identifier_start(c) => {
                    if self.starts_with_rem_comment() {
                        self.consume_comment();
                    } else if self.starts_data_statement(&tokens) {
                        tokens.push(self.lex_data_line(start, line, column));
                    } else {
                        tokens.push(self.lex_identifier(start, line, column));
                    }
                }
                _ => {
                    self.advance_char();
                    let span = TextSpan::new(start, self.index);
                    self.error(format!("Unexpected character: {current}"), span, line, column);
                    tokens.push(Token::new(TokenType::Error, current.to_string(), span, line, column));
                }
            }
        }

        tokens.push(Token::new(
            TokenType::Eof,
            String::new(),
            TextSpan::new(self.index, self.index),
            self.line,
            self.column,
        ));

        demote_keyword_names(&mut tokens);
        (tokens, self.errors)
    }

    fn error(&mut self, message: impl Into<String>, span: TextSpan, line: usize, column: usize) {
        self.errors.push(LexError::new(message, span, line, column, self.file_path.clone()));
    }

    fn is_at_end(&self) -> bool {
        self.index >= self.source.len()
    }

    fn current_char(&self) -> Option<char> {
        self.source[self.index..].chars().next()
    }

    fn peek_char(&self, offset: usize) -> Option<char> {
        self.source[self.index..].chars().nth(offset)
    }

    fn advance_char(&mut self) -> Option<char> {
        let current = self.current_char()?;
        self.index += current.len_utf8();
        self.column += 1;
        Some(current)
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.current_char() == Some(expected) {
            self.advance_char();
            return true;
        }
        false
    }

    fn consume_newline(&mut self) {
        if self.current_char() == Some('\r') {
            self.advance_char();
            if self.current_char() == Some('\n') {
                self.advance_char();
            }
        } else {
            self.advance_char();
        }

        self.line += 1;
        self.column = 1;
    }

    fn consume_comment(&mut self) {
        while let Some(current) = self.current_char() {
            if current == '\r' || current == '\n' {
                break;
            }
            self.advance_char();
        }
    }

    /// Whether the `_` here is the last thing on its line (spaces or a
    /// `' comment` may follow).
    fn underscore_ends_line(&self) -> bool {
        let rest = &self.source[self.index + 1..];
        let rest = rest.trim_start_matches([' ', '\t']);
        rest.is_empty() || rest.starts_with(['\r', '\n', '\''])
    }

    /// After a continuation: lines holding only a comment are skipped (the
    /// statement goes on after them).
    fn skip_comment_lines(&mut self) {
        loop {
            let rest = &self.source[self.index..];
            let trimmed = rest.trim_start_matches([' ', '\t']);
            if !trimmed.starts_with('\'') {
                return;
            }
            while !matches!(self.current_char(), None | Some('\r' | '\n')) {
                self.advance_char();
            }
            if self.current_char().is_none() {
                return;
            }
            self.consume_newline();
        }
    }

    fn try_consume_line_continuation(&mut self) -> bool {
        if self.current_char() != Some('_') {
            return false;
        }

        let mut lookahead = self.index + 1;
        while let Some(ch) = self.source[lookahead..].chars().next() {
            match ch {
                ' ' | '\t' => lookahead += ch.len_utf8(),
                '\r' | '\n' => {
                    self.advance_char();
                    while matches!(self.current_char(), Some(' ' | '\t')) {
                        self.advance_char();
                    }
                    self.consume_newline();
                    self.skip_comment_lines();
                    return true;
                }
                // `_   ' comment` continues the line too.
                '\'' => {
                    while !matches!(self.current_char(), None | Some('\r' | '\n')) {
                        self.advance_char();
                    }
                    if self.current_char().is_some() {
                        self.consume_newline();
                        self.skip_comment_lines();
                    }
                    return true;
                }
                _ => return false,
            }
        }

        false
    }

    /// A string under `$ESCAPECHARS ON` (RapidQ manual, chapter 3): `\a \b
    /// \f \n \r \t \v \\ \"`, `\###` (decimal 0..255) and `\xHH`. Ends at
    /// its closing quote or, like any string, at the end of the line.
    fn lex_escaped_string(&mut self, start: usize, line: usize, column: usize) -> Token {
        self.advance_char();
        let mut text = String::new();
        while let Some(ch) = self.current_char() {
            match ch {
                '"' => {
                    self.advance_char();
                    break;
                }
                '\r' | '\n' => break,
                // (a `_` continuation joins the next line, as in any string)
                '_' if self.string_continuation_follows() => {
                    self.advance_char();
                    self.try_consume_line_continuation_tail();
                }
                '\\' => {
                    self.advance_char();
                    let Some(e) = self.current_char() else { text.push('\\'); break };
                    let simple = match e {
                        'a' => Some('\u{7}'),
                        'b' => Some('\u{8}'),
                        'f' => Some('\u{c}'),
                        'n' => Some('\n'),
                        'r' => Some('\r'),
                        't' => Some('\t'),
                        'v' => Some('\u{b}'),
                        '\\' => Some('\\'),
                        '"' => Some('"'),
                        _ => None,
                    };
                    if let Some(c) = simple {
                        self.advance_char();
                        text.push(c);
                    } else if e == 'x' && self.peek_char(1).is_some_and(|c| c.is_ascii_hexdigit()) {
                        self.advance_char();
                        let mut n = 0u32;
                        for _ in 0..2 {
                            match self.current_char().and_then(|c| c.to_digit(16)) {
                                Some(d) => {
                                    n = n * 16 + d;
                                    self.advance_char();
                                }
                                None => break,
                            }
                        }
                        text.push(char::from_u32(n).unwrap_or('?'));
                    } else if e.is_ascii_digit() {
                        let mut n = 0u32;
                        for _ in 0..3 {
                            match self.current_char().and_then(|c| c.to_digit(10)) {
                                Some(d) if n * 10 + d <= 255 => {
                                    n = n * 10 + d;
                                    self.advance_char();
                                }
                                _ => break,
                            }
                        }
                        text.push(char::from_u32(n).unwrap_or('?'));
                    } else {
                        // Unknown escape: keep it as written.
                        text.push('\\');
                    }
                }
                _ => {
                    text.push(ch);
                    self.advance_char();
                }
            }
        }
        Token::new(TokenType::StringLit, text, TextSpan::new(start, self.index), line, column)
    }

    /// `DATA` beginning a statement (not a variable named Data: `Data = 1`,
    /// `Data(2)`, `Data.x`).
    fn starts_data_statement(&self, tokens: &[Token]) -> bool {
        let rest = &self.source[self.index..];
        if !rest.as_bytes().get(..4).is_some_and(|b| b.eq_ignore_ascii_case(b"DATA")) {
            return false;
        }
        let after = &rest[4..];
        if after.chars().next().is_some_and(Self::is_identifier_part) {
            return false;
        }
        // (a line number in front is a label: `130 DATA 1, 2`)
        let after_line_number = tokens.last().is_some_and(|t| t.kind == TokenType::Number && t.lexeme.chars().all(|c| c.is_ascii_digit()))
            && matches!(tokens.len().checked_sub(2).and_then(|i| tokens.get(i)).map(|t| t.kind), None | Some(TokenType::Newline));
        let at_statement_start = after_line_number
            || matches!(
                tokens.last().map(|t| t.kind),
                None | Some(TokenType::Newline | TokenType::Colon)
            );
        let next_text = after.trim_start_matches([' ', '\t']);
        let next = next_text.chars().next();
        // (`Data AS QStringGrid`: a TYPE's field named Data)
        let field = next_text.get(..2).is_some_and(|w| w.eq_ignore_ascii_case("as")) && !next_text[2..].chars().next().is_some_and(Self::is_identifier_part);
        at_statement_start && !field && !matches!(next, Some('=' | '(' | '.'))
    }

    /// The rest of a DATA line, without a trailing `' comment` (a `'` inside
    /// quotes is data).
    fn lex_data_line(&mut self, start: usize, line: usize, column: usize) -> Token {
        for _ in 0..4 {
            self.advance_char();
        }
        let body_start = self.index;
        let mut in_quotes = false;
        let mut body_end = None;
        while let Some(ch) = self.current_char() {
            if ch == '\r' || ch == '\n' {
                break;
            }
            if ch == '"' {
                in_quotes = !in_quotes;
            }
            if ch == '\'' && !in_quotes && body_end.is_none() {
                body_end = Some(self.index);
            }
            self.advance_char();
        }
        let raw = self.source[body_start..body_end.unwrap_or(self.index)].to_string();
        Token::new(TokenType::Data, raw, TextSpan::new(start, self.index), line, column)
    }

    /// After a `_` inside a string: only spaces/tabs, then a line break.
    fn string_continuation_follows(&self) -> bool {
        let rest = &self.source[self.index + 1..];
        let trimmed = rest.trim_start_matches([' ', '\t']);
        trimmed.starts_with('\n') || trimmed.starts_with("\r\n")
    }

    /// Skips the line break after a string's `_` and the next line's indentation.
    fn try_consume_line_continuation_tail(&mut self) {
        while matches!(self.current_char(), Some(' ' | '\t')) {
            self.advance_char();
        }
        if matches!(self.current_char(), Some('\r' | '\n')) {
            self.consume_newline();
        }
        while matches!(self.current_char(), Some(' ' | '\t')) {
            self.advance_char();
        }
    }

    /// The rest of a continued string, up to its closing quote or the end
    /// of the line (lenient, like an unterminated string).
    fn lex_string_tail(&mut self) -> String {
        let content_start = self.index;
        while let Some(ch) = self.current_char() {
            if ch == '"' {
                let text = self.source[content_start..self.index].to_string();
                self.advance_char();
                return text;
            }
            if ch == '_' && self.string_continuation_follows() {
                let mut text = self.source[content_start..self.index].to_string();
                self.advance_char();
                self.try_consume_line_continuation_tail();
                text.push_str(&self.lex_string_tail());
                return text;
            }
            if ch == '\r' || ch == '\n' {
                break;
            }
            self.advance_char();
        }
        self.source[content_start..self.index].to_string()
    }

    /// RapidQ accepts names that start with digits (`SUB 01click`): digits
    /// running straight into letters form one name, unless the letters are
    /// a keyword (`1TO 5`), an exponent (`1E5`) or a type suffix (`5&`).
    fn digits_start_a_name(&self) -> bool {
        let rest = &self.source[self.index..];
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        let tail = &rest[digits..];
        let word: String = tail.chars().take_while(|c| Self::is_identifier_part(*c)).collect();
        let Some(first) = word.chars().next() else { return false };
        if !first.is_alphabetic() || matches!(first, 'e' | 'E') && word[1..].chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
        keyword_token(&word.to_ascii_uppercase()).is_none()
    }

    fn starts_with_rem_comment(&self) -> bool {
        let slice = &self.source[self.index..];
        if !slice.as_bytes().get(..3).is_some_and(|b| b.eq_ignore_ascii_case(b"REM")) {
            return false;
        }

        match slice[3..].chars().next() {
            None => true,
            Some(ch) => !Self::is_identifier_part(ch),
        }
    }

    fn lex_directive(&mut self, start: usize, line: usize, column: usize) -> Token {
        self.advance_char();
        while let Some(ch) = self.current_char() {
            if ch.is_ascii_alphabetic() || ch == '_' {
                self.advance_char();
            } else {
                break;
            }
        }

        let lexeme = self.source[start..self.index].to_string();
        let trailing_start = self.index;
        while let Some(ch) = self.current_char() {
            if ch == '\r' || ch == '\n' {
                break;
            }
            self.advance_char();
        }

        let trailing = self.source[trailing_start..self.index].trim().to_string();
        if lexeme.eq_ignore_ascii_case("$ESCAPECHARS") {
            let setting = trailing.split('\'').next().unwrap_or("").trim();
            self.escape_chars = setting.eq_ignore_ascii_case("ON");
        }
        Token::new(
            TokenType::Directive,
            lexeme,
            TextSpan::new(start, self.index),
            line,
            column,
        )
        .with_trailing((!trailing.is_empty()).then_some(trailing))
    }

    fn lex_string(
        &mut self,
        start: usize,
        line: usize,
        column: usize,
    ) -> Token {
        if self.escape_chars {
            return self.lex_escaped_string(start, line, column);
        }
        self.advance_char();
        let content_start = self.index;

        while let Some(ch) = self.current_char() {
            // (`""` inside a string is no quote in RapidQ: RC.EXE reads
            // `"[:"":>"` as two strings side by side, whose value is the
            // first — `[:`, LEN 2 — the parser's juxtaposed operands)
            if ch == '"' {
                let lexeme = self.source[content_start..self.index].to_string();
                self.advance_char();
                return Token::new(TokenType::StringLit, lexeme, TextSpan::new(start, self.index), line, column);
            }

            // `"first part _` + newline continues the string on the next
            // line (RapidQ joins `_` continuations before reading strings).
            if ch == '_' && self.string_continuation_follows() {
                let mut text = self.source[content_start..self.index].to_string();
                self.advance_char();
                self.try_consume_line_continuation_tail();
                let rest = self.lex_string_tail();
                text.push_str(&rest);
                return Token::new(TokenType::StringLit, text, TextSpan::new(start, self.index), line, column);
            }
            if ch == '\r' || ch == '\n' {
                // RapidQ ends an unterminated string at the end of the line
                // (and real programs rely on it: `x = "error'`).
                let lexeme = self.source[content_start..self.index].to_string();
                return Token::new(TokenType::StringLit, lexeme, TextSpan::new(start, self.index), line, column);
            }

            self.advance_char();
        }

        // (at the end of the file: an error, the string as far as it goes)
        let span = TextSpan::new(start, self.index);
        self.error("Unterminated string literal", span, line, column);
        Token::new(TokenType::StringLit, self.source[content_start..self.index].to_string(), span, line, column)
    }

    fn is_prefixed_number(&self) -> bool {
        matches!(self.peek_char(1), Some('H' | 'h' | 'O' | 'o' | 'B' | 'b'))
    }

    fn lex_prefixed_number(
        &mut self,
        start: usize,
        line: usize,
        column: usize,
    ) -> Token {
        self.advance_char();
        let prefix = self.advance_char().unwrap();
        let digit_start = self.index;

        // RapidQ's `&H` takes the whole alphanumeric run and keeps its hex
        // digits — `&hHE` is 14, `&hG1` 1, a bare `&h` 0 (RC.EXE; the
        // corpus' SBLIB.BAS). `&O` / `&B` are RapidR's own.
        let mut hex_digits = String::new();
        while let Some(ch) = self.current_char() {
            let valid = match prefix {
                'H' | 'h' => ch.is_ascii_alphanumeric(),
                'O' | 'o' => matches!(ch, '0'..='7'),
                'B' | 'b' => matches!(ch, '0' | '1'),
                _ => false,
            };

            if valid {
                if ch.is_ascii_hexdigit() {
                    hex_digits.push(ch);
                }
                self.advance_char();
            } else {
                break;
            }
        }

        if digit_start == self.index && !matches!(prefix, 'H' | 'h') {
            let span = TextSpan::new(start, self.index);
            self.error("Invalid prefixed number literal", span, line, column);
            return Token::new(TokenType::Error, self.source[start..self.index].to_string(), span, line, column);
        }

        // A long-integer suffix (`&H1&`, `&HFFFF&` in the Windows includes):
        // the `&` right after the digits, not a concatenation.
        let digits_end = self.index;
        if self.current_char() == Some('&')
            && !matches!(self.peek_char(1), Some(c) if c.is_alphanumeric() || matches!(c, '_' | '"' | '(' | '&'))
        {
            self.advance_char();
        }
        let digits = &self.source[digit_start..digits_end];
        let normalized = match prefix {
            'H' | 'h' => format!("0x{}", if hex_digits.is_empty() { "0" } else { hex_digits.as_str() }),
            'O' | 'o' => format!("0o{digits}"),
            'B' | 'b' => format!("0b{digits}"),
            _ => unreachable!(),
        };

        Token::new(TokenType::Number, normalized, TextSpan::new(start, self.index), line, column)
    }

    fn lex_decimal_number(&mut self, start: usize, line: usize, column: usize) -> Token {
        while matches!(self.current_char(), Some('0'..='9')) {
            self.advance_char();
        }

        if self.current_char() == Some('.') && matches!(self.peek_char(1), Some('0'..='9')) {
            self.advance_char();
            while matches!(self.current_char(), Some('0'..='9')) {
                self.advance_char();
            }
        }

        if matches!(self.current_char(), Some('e' | 'E')) {
            let checkpoint = self.index;
            self.advance_char();
            if matches!(self.current_char(), Some('+' | '-')) {
                self.advance_char();
            }
            if matches!(self.current_char(), Some('0'..='9')) {
                while matches!(self.current_char(), Some('0'..='9')) {
                    self.advance_char();
                }
            } else {
                self.index = checkpoint;
                self.column = column + (checkpoint - start);
            }
        }

        let end = self.index;
        // Type suffix on a literal: `0&` (LONG), `1.5!` (SINGLE), `2#`, `7%`,
        // and BYTE / WORD / DWORD's `?`, `??`, `???` (`xFinish=0??`).
        if matches!(self.current_char(), Some('&' | '!' | '#' | '%'))
            && !self.peek_char(1).is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            self.advance_char();
        } else if self.current_char() == Some('?') {
            let n = (0..3).take_while(|&k| self.peek_char(k) == Some('?')).count();
            if !self.peek_char(n).is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '"') {
                for _ in 0..n {
                    self.advance_char();
                }
            }
        }

        Token::new(
            TokenType::Number,
            self.source[start..end].to_string(),
            TextSpan::new(start, self.index),
            line,
            column,
        )
    }

    fn lex_identifier(&mut self, start: usize, line: usize, column: usize) -> Token {
        self.advance_char();
        while let Some(ch) = self.current_char() {
            // `Getreditformhndle_` at the end of a line: RapidQ's line
            // continuation, stuck to the name.
            if ch == '_' && self.underscore_ends_line() {
                break;
            }
            if Self::is_identifier_part(ch) {
                self.advance_char();
            } else {
                break;
            }
        }

        if matches!(self.current_char(), Some('$' | '%' | '#' | '&' | '!')) {
            self.advance_char();
        } else if self.current_char() == Some('?') && keyword_token(&self.source[start..self.index]).is_none() {
            // `b?` BYTE, `w??` WORD, `d???` DWORD (a `?` after a keyword is PRINT).
            for _ in 0..3 {
                if self.current_char() != Some('?') {
                    break;
                }
                self.advance_char();
            }
        }

        let lexeme = self.source[start..self.index].to_string();
        if let Some(keyword) = keyword_token(&lexeme) {
            if keyword == TokenType::RustStart {
                // Scan forward for RUSTEND, collecting everything as the body
                // Skip to end of this line first
                while let Some(ch) = self.current_char() {
                    if ch == '\n' { self.advance_char(); self.line += 1; self.column = 1; break; }
                    self.advance_char();
                }
                let body_start = self.index;
                let mut body_end = self.index;
                loop {
                    if self.index >= self.source.len() { break; }
                    // Check if current line starts with RUSTEND
                    let remaining = &self.source[self.index..];
                    let trimmed = remaining.trim_start();
                    if trimmed.to_ascii_uppercase().starts_with("RUSTEND") {
                        body_end = self.index;
                        // Skip the RUSTEND line
                        while let Some(ch) = self.current_char() {
                            if ch == '\n' { self.advance_char(); self.line += 1; self.column = 1; break; }
                            self.advance_char();
                        }
                        break;
                    }
                    // Advance one line
                    while let Some(ch) = self.current_char() {
                        if ch == '\n' { self.advance_char(); self.line += 1; self.column = 1; break; }
                        self.advance_char();
                    }
                }
                let body = self.source[body_start..body_end].to_string();
                return Token {
                    kind: TokenType::RustStart,
                    lexeme: body,
                    span: TextSpan::new(start, self.index),
                    line,
                    column,
                    trailing: None,
                };
            }
            Token::new(
                keyword,
                lexeme.to_ascii_uppercase(),
                TextSpan::new(start, self.index),
                line,
                column,
            )
        } else {
            Token::new(
                TokenType::Identifier,
                lexeme,
                TextSpan::new(start, self.index),
                line,
                column,
            )
        }
    }

    // Accented letters are allowed (RapidQ read ANSI source, and programs
    // use names like `Précédent`).
    fn is_identifier_start(ch: char) -> bool {
        ch.is_alphabetic() || ch == '_'
    }

    fn is_identifier_part(ch: char) -> bool {
        ch.is_alphanumeric() || ch == '_'
    }
}

fn keyword_token(identifier: &str) -> Option<TokenType> {
    match identifier.to_ascii_uppercase().as_str() {
        "DIM" => Some(TokenType::Dim),
        "AS" => Some(TokenType::As),
        "INTEGER" => Some(TokenType::Integer),
        "STRING" => Some(TokenType::String),
        "DOUBLE" => Some(TokenType::Double),
        "SINGLE" => Some(TokenType::Single),
        "BYTE" => Some(TokenType::Byte),
        "WORD" => Some(TokenType::Word),
        "DWORD" => Some(TokenType::Dword),
        "LONG" => Some(TokenType::Long),
        "INT64" => Some(TokenType::Int64),
        "CURRENCY" => Some(TokenType::Currency),
        "ROBJECT" => Some(TokenType::RObject),
        "VARIANT" => Some(TokenType::Variant),
        "IF" => Some(TokenType::If),
        "THEN" => Some(TokenType::Then),
        "ELSE" => Some(TokenType::Else),
        "ELSEIF" => Some(TokenType::ElseIf),
        "END" => Some(TokenType::End),
        "FOR" => Some(TokenType::For),
        "TO" => Some(TokenType::To),
        "STEP" => Some(TokenType::Step),
        "NEXT" => Some(TokenType::Next),
        "WHILE" => Some(TokenType::While),
        "WEND" => Some(TokenType::Wend),
        "DO" => Some(TokenType::Do),
        "LOOP" => Some(TokenType::Loop),
        "UNTIL" => Some(TokenType::Until),
        "SELECT" => Some(TokenType::Select),
        "CASE" => Some(TokenType::Case),
        "SUB" => Some(TokenType::Sub),
        "FUNCTION" => Some(TokenType::Function),
        "CALL" => Some(TokenType::Call),
        "RETURN" => Some(TokenType::Return),
        "EXIT" => Some(TokenType::Exit),
        "PRINT" => Some(TokenType::Print),
        "INPUT" => Some(TokenType::Input),
        "GOTO" => Some(TokenType::Goto),
        "GOSUB" => Some(TokenType::Gosub),
        "IMPORT" => Some(TokenType::Import),
        "CREATE" => Some(TokenType::Create),
        "CONST" => Some(TokenType::Const),
        // (RapidQ's `STRUCT … END STRUCT` is a TYPE)
        "TYPE" | "STRUCT" => Some(TokenType::Type),
        "DECLARE" => Some(TokenType::Declare),
        "LIB" => Some(TokenType::Lib),
        "ALIAS" => Some(TokenType::Alias),
        "WITH" => Some(TokenType::With),
        "MOD" => Some(TokenType::Mod),
        "DEFSTR" => Some(TokenType::DefStr),
        "DEFINT" => Some(TokenType::DefInt),
        "DEFBYTE" => Some(TokenType::DefByte),
        "DEFWORD" => Some(TokenType::DefWord),
        "DEFDWORD" => Some(TokenType::DefDword),
        "DEFLONG" | "DEFLNG" => Some(TokenType::DefLong),
        "DEFSHORT" => Some(TokenType::DefShort),
        "DEFSNG" => Some(TokenType::DefSng),
        "DEFDBL" => Some(TokenType::DefDbl),
        "DEFCUR" => Some(TokenType::DefCur),
        "EXTENDS" => Some(TokenType::Extends),
        "PROPERTY" => Some(TokenType::Property),
        "SET" => Some(TokenType::Set),
        "BYVAL" => Some(TokenType::ByVal),
        "BYREF" => Some(TokenType::ByRef),
        "BIND" => Some(TokenType::Bind),
        "CONSTRUCTOR" => Some(TokenType::Constructor),
        "AND" => Some(TokenType::And),
        "OR" => Some(TokenType::Or),
        "NOT" => Some(TokenType::Not),
        "XOR" => Some(TokenType::Xor),
        "OPEN" => Some(TokenType::Open),
        "CLOSE" => Some(TokenType::Close),
        "WRITE" => Some(TokenType::Write),
        "SEEK" => Some(TokenType::Seek),
        "KILL" => Some(TokenType::Kill),
        "RUSTSTART" => Some(TokenType::RustStart),
        "RUSTEND" => Some(TokenType::RustEnd),
        _ => None,
    }
}

/// Keywords of statements that RapidQ programs also use as names of their
/// components and variables (`DIM Open AS QMENUITEM`, `Open.Caption = …`,
/// `File.AddItems New, Open, Save`): where one is followed by `.`, is a
/// declared name (`DIM … Open AS`), or is an argument, it's an identifier.
fn demote_keyword_names(tokens: &mut [Token]) {
    use TokenType::*;
    let demotable = |k: TokenType| matches!(k, Open | Close | Write | Seek | Kill | Input | Set | Bind | Alias | Property | Extends);
    for i in 0..tokens.len() {
        if !demotable(tokens[i].kind) {
            continue;
        }
        let prev = i.checked_sub(1).map(|j| tokens[j].kind);
        let next = tokens.get(i + 1).map(|t| t.kind);
        let member = next == Some(Dot);
        let declared = matches!(prev, Some(Dim | Comma)) && next == Some(As);
        let argument = matches!(prev, Some(Comma | LParen)) && matches!(next, Some(Comma | RParen));
        if member || declared || argument {
            tokens[i].kind = Identifier;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Lexer, TokenType};

    fn lex(code: &str) -> Vec<super::Token> {
        Lexer::new(code, None).tokenize().unwrap()
    }

    #[test]
    fn lexes_keywords_case_insensitively() {
        let tokens = lex("DIM x AS INTEGER\nIf y = 5 tHeN Print y");
        let kinds: Vec<TokenType> = tokens.into_iter().map(|token| token.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenType::Dim,
                TokenType::Identifier,
                TokenType::As,
                TokenType::Integer,
                TokenType::Newline,
                TokenType::If,
                TokenType::Identifier,
                TokenType::Eq,
                TokenType::Number,
                TokenType::Then,
                TokenType::Print,
                TokenType::Identifier,
                TokenType::Eof,
            ]
        );
    }

    #[test]
    fn lexes_strings_and_numbers() {
        let tokens = lex("val = \"Hello World!\"\nnum = 123.45e-2");
        assert_eq!(tokens[0].kind, TokenType::Identifier);
        assert_eq!(tokens[2].kind, TokenType::StringLit);
        assert_eq!(tokens[2].lexeme, "Hello World!");
        assert_eq!(tokens[6].kind, TokenType::Number);
        assert_eq!(tokens[6].lexeme, "123.45e-2");
    }

    #[test]
    fn skips_comments() {
        let tokens = lex("DIM ' This is a comment\nREM this is also a comment");
        let kinds: Vec<TokenType> = tokens.into_iter().map(|token| token.kind).collect();
        assert_eq!(kinds, vec![TokenType::Dim, TokenType::Newline, TokenType::Eof]);
    }

    #[test]
    fn lexes_operators() {
        let tokens = lex("a + b - c * d / e \\ f ^ g < > <= >= = <> AND OR NOT");
        let kinds: Vec<TokenType> = tokens.into_iter().map(|token| token.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenType::Identifier,
                TokenType::Plus,
                TokenType::Identifier,
                TokenType::Minus,
                TokenType::Identifier,
                TokenType::Star,
                TokenType::Identifier,
                TokenType::Slash,
                TokenType::Identifier,
                TokenType::Backslash,
                TokenType::Identifier,
                TokenType::Caret,
                TokenType::Identifier,
                TokenType::Lt,
                TokenType::Gt,
                TokenType::Lte,
                TokenType::Gte,
                TokenType::Eq,
                TokenType::Neq,
                TokenType::And,
                TokenType::Or,
                TokenType::Not,
                TokenType::Eof,
            ]
        );
    }

    #[test]
    fn preserves_suffixes_on_identifiers() {
        let tokens = lex("name$ value% x# ptr&");
        assert_eq!(tokens[0].lexeme, "name$");
        assert_eq!(tokens[1].lexeme, "value%");
        assert_eq!(tokens[2].lexeme, "x#");
        assert_eq!(tokens[3].lexeme, "ptr&");
        let tokens = lex("b? w?? d??? IF x THEN? 1");
        assert_eq!([tokens[0].lexeme.as_str(), &tokens[1].lexeme, &tokens[2].lexeme], ["b?", "w??", "d???"]);
        assert_eq!(tokens[6].kind, TokenType::Print);
    }

    #[test]
    fn lexes_directives_with_trailing_text() {
        let tokens = lex("$TYPECHECK ON\n$INCLUDE <PForms.inc>");
        assert_eq!(tokens[0].kind, TokenType::Directive);
        assert_eq!(tokens[0].lexeme, "$TYPECHECK");
        assert_eq!(tokens[0].trailing.as_deref(), Some("ON"));
        assert_eq!(tokens[2].kind, TokenType::Directive);
        assert_eq!(tokens[2].lexeme, "$INCLUDE");
        assert_eq!(tokens[2].trailing.as_deref(), Some("<PForms.inc>"));
    }

    #[test]
    fn recovers_from_errors() {
        let (tokens, errors) = Lexer::new("a = 1 ` b\nc = &HZ\nd = \"open", None).tokenize_recovering();
        let kinds: Vec<TokenType> = tokens.iter().map(|t| t.kind).collect();
        use TokenType::*;
        assert_eq!(kinds, [Identifier, Eq, Number, Error, Identifier, Newline, Identifier, Eq, Error, Identifier, Newline, Identifier, Eq, StringLit, Eof]);
        let messages: Vec<&str> = errors.iter().map(|e| e.diagnostic.message.as_str()).collect();
        assert_eq!(messages, ["Unexpected character: `", "Invalid prefixed number literal", "Unterminated string literal"]);
        // `tokenize` stops at the first, as before.
        assert_eq!(Lexer::new("a = 1 ` b", None).tokenize().unwrap_err().diagnostic.message, "Unexpected character: `");
    }

    #[test]
    fn lossless_round_trip_with_trivia() {
        use super::lossless::{LineClass, LosslessFile, TriviaKind};
        let text = "#!/usr/bin/env rapidr\r\n' header\r\n\r\n$INCLUDE \"x.inc\"\r\n$IFDEF NOPE\r\nPRINT \"no\"\r\n$ENDIF\r\nDIM a AS INTEGER ' the count\r\nREM old style\r\nx = a + _  ' more\r\n    1\r\n\t\r\n$TYPECHECK ON\r\nPRINT x ` oops\r\n";
        let file = LosslessFile::lex(text, None);
        assert_eq!(file.print(), text);
        let kinds: Vec<TriviaKind> = file.trivia.iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&TriviaKind::Shebang) && kinds.contains(&TriviaKind::Inactive) && kinds.contains(&TriviaKind::LineContinuation) && kinds.contains(&TriviaKind::LineBreak));
        let directives: Vec<&str> = file.trivia.iter().filter(|t| t.kind == TriviaKind::Directive).map(|t| &text[t.span.start..t.span.end]).collect();
        assert_eq!(directives, ["$INCLUDE \"x.inc\"", "$IFDEF NOPE", "$ENDIF"]);
        let comments: Vec<&str> = file.trivia.iter().filter(|t| t.kind == TriviaKind::Comment).map(|t| &text[t.span.start..t.span.end]).collect();
        assert_eq!(comments, ["' header", "' the count", "REM old style", "' more"]);
        use LineClass::*;
        assert_eq!(file.lines(), [Shebang, Comment, Blank, Directive, Directive, Inactive, Directive, Code, Comment, Code, Code, Blank, Code, Code, Blank]);
        assert_eq!(file.errors.len(), 1);
        // The tokens are the compiler's (the `$TYPECHECK` directive among them).
        assert!(file.tokens.iter().any(|t| t.kind == TokenType::Directive && t.lexeme == "$TYPECHECK"));
        // Trivia before `DIM`: the blank and directive lines.
        let dim = file.tokens.iter().position(|t| t.kind == TokenType::Dim).unwrap();
        assert!(file.leading_trivia(dim).is_empty(), "the line break before DIM is a token");
    }

    #[test]
    fn swallows_line_continuations() {
        let tokens = lex("DIM value _\n    AS INTEGER\n");
        let kinds: Vec<TokenType> = tokens.into_iter().map(|token| token.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenType::Dim,
                TokenType::Identifier,
                TokenType::As,
                TokenType::Integer,
                TokenType::Newline,
                TokenType::Eof,
            ]
        );
    }
}