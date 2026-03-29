use std::error::Error;
use std::fmt;

use rapidr_diagnostics::{Diagnostic, SourceLocation, TextSpan};

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
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str, file_path: Option<String>) -> Self {
        Self {
            source,
            file_path,
            index: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, LexError> {
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
                    tokens.push(self.lex_string(start, line, column)?);
                }
                '&' => {
                    if self.is_prefixed_number() {
                        tokens.push(self.lex_prefixed_number(start, line, column)?);
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
                '0'..='9' => {
                    tokens.push(self.lex_decimal_number(start, line, column));
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
                    tokens.push(Token::new(
                        TokenType::Eq,
                        "=".to_string(),
                        TextSpan::new(start, self.index),
                        line,
                        column,
                    ));
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
                    } else {
                        tokens.push(self.lex_identifier(start, line, column));
                    }
                }
                _ => {
                    return Err(LexError::new(
                        format!("Unexpected character: {current}"),
                        TextSpan::new(start, start + current.len_utf8()),
                        line,
                        column,
                        self.file_path.clone(),
                    ));
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

        Ok(tokens)
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
                    return true;
                }
                _ => return false,
            }
        }

        false
    }

    fn starts_with_rem_comment(&self) -> bool {
        let slice = &self.source[self.index..];
        if slice.len() < 3 || !slice[..3].eq_ignore_ascii_case("REM") {
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
    ) -> Result<Token, LexError> {
        self.advance_char();
        let content_start = self.index;

        while let Some(ch) = self.current_char() {
            if ch == '"' {
                let lexeme = self.source[content_start..self.index].to_string();
                self.advance_char();
                return Ok(Token::new(
                    TokenType::StringLit,
                    lexeme,
                    TextSpan::new(start, self.index),
                    line,
                    column,
                ));
            }

            if ch == '\r' || ch == '\n' {
                return Err(LexError::new(
                    "Unterminated string literal",
                    TextSpan::new(start, self.index),
                    line,
                    column,
                    self.file_path.clone(),
                ));
            }

            self.advance_char();
        }

        Err(LexError::new(
            "Unterminated string literal",
            TextSpan::new(start, self.index),
            line,
            column,
            self.file_path.clone(),
        ))
    }

    fn is_prefixed_number(&self) -> bool {
        matches!(self.peek_char(1), Some('H' | 'h' | 'O' | 'o' | 'B' | 'b'))
    }

    fn lex_prefixed_number(
        &mut self,
        start: usize,
        line: usize,
        column: usize,
    ) -> Result<Token, LexError> {
        self.advance_char();
        let prefix = self.advance_char().unwrap();
        let digit_start = self.index;

        while let Some(ch) = self.current_char() {
            let valid = match prefix {
                'H' | 'h' => ch.is_ascii_hexdigit(),
                'O' | 'o' => matches!(ch, '0'..='7'),
                'B' | 'b' => matches!(ch, '0' | '1'),
                _ => false,
            };

            if valid {
                self.advance_char();
            } else {
                break;
            }
        }

        if digit_start == self.index {
            return Err(LexError::new(
                "Invalid prefixed number literal",
                TextSpan::new(start, self.index),
                line,
                column,
                self.file_path.clone(),
            ));
        }

        let digits = &self.source[digit_start..self.index];
        let normalized = match prefix {
            'H' | 'h' => format!("0x{digits}"),
            'O' | 'o' => format!("0o{digits}"),
            'B' | 'b' => format!("0b{digits}"),
            _ => unreachable!(),
        };

        Ok(Token::new(
            TokenType::Number,
            normalized,
            TextSpan::new(start, self.index),
            line,
            column,
        ))
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

        Token::new(
            TokenType::Number,
            self.source[start..self.index].to_string(),
            TextSpan::new(start, self.index),
            line,
            column,
        )
    }

    fn lex_identifier(&mut self, start: usize, line: usize, column: usize) -> Token {
        self.advance_char();
        while let Some(ch) = self.current_char() {
            if Self::is_identifier_part(ch) {
                self.advance_char();
            } else {
                break;
            }
        }

        if matches!(self.current_char(), Some('$' | '%' | '#' | '&' | '!')) {
            self.advance_char();
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

    fn is_identifier_start(ch: char) -> bool {
        ch.is_ascii_alphabetic() || ch == '_'
    }

    fn is_identifier_part(ch: char) -> bool {
        ch.is_ascii_alphanumeric() || ch == '_'
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
        "TYPE" => Some(TokenType::Type),
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
        "DEFLONG" => Some(TokenType::DefLong),
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