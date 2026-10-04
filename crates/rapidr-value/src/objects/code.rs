//! RCODEEDITOR, RapidR's code editor (the IDE's): what it adds to the
//! shared text model (`textedit::TextEdit` in code mode) — BASIC's syntax
//! colours, and the SUBs and FUNCTIONs it finds (GetSubList, GotoSub) —
//! the same on every runtime. RapidQ has no code editor, so these rules
//! are RapidR's own.
//!
//! Highlighting is per line: no token runs past a line break (a comment
//! ends with its line, and so does a string left open), so an edit only
//! colours again the lines it touched. [`spans`] takes and returns a
//! state for the hosts that carry one from line to line, ready for a
//! syntax with block comments; BASIC's is always 0.

use std::ops::Range;

/// What a piece of code is, for its colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Token {
    Keyword,
    String,
    /// `'` to the line's end.
    Comment,
    Number,
    Normal,
}

/// How a token is drawn: its colour (0xRRGGBB), bold, italic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style {
    pub color: u32,
    pub bold: bool,
    pub italic: bool,
}

impl Token {
    /// The editor's style table: keywords dark blue and bold, strings red,
    /// comments green and italic, numbers maroon, the rest black.
    pub fn style(self) -> Style {
        let (color, bold, italic) = match self {
            Token::Keyword => (0x0000B4, true, false),
            Token::String => (0xA31515, false, false),
            Token::Comment => (0x008000, false, true),
            Token::Number => (0x800000, false, false),
            Token::Normal => (0x000000, false, false),
        };
        Style { color, bold, italic }
    }
}

/// Which syntax an editor colours its text by.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Syntax {
    /// Plain text.
    #[default]
    None,
    /// RapidQ / RapidR BASIC.
    Basic,
}

/// The words shown as keywords (any case).
pub const KEYWORDS: &[&str] = &[
    "SUB", "END", "FUNCTION", "DIM", "AS", "IF", "THEN", "ELSE", "ELSEIF", "FOR", "TO", "STEP", "NEXT", "WHILE", "WEND", "DO", "LOOP", "UNTIL", "SELECT", "CASE", "EXIT", "CREATE", "INTEGER", "STRING", "DOUBLE", "BOOLEAN", "AND", "OR",
    "NOT", "MOD", "TRUE", "FALSE", "CONST", "RETURN", "PRINT", "MSGBOX", "SHELL", "SHELLWAIT", "CALL", "RFORM", "RBUTTON", "RLABEL", "REDIT", "RCHECKBOX", "RRADIOBUTTON", "RCOMBOBOX", "RLISTBOX", "RPANEL", "RGROUPBOX",
    "RDESIGNSURFACE", "RCODEEDITOR", "RSTRINGGRID", "RTREEVIEW", "RCANVAS", "RTIMER", "RIMAGE", "RRICHEDIT", "RPROGRESSBAR", "RTRACKBAR", "RSCROLLBAR", "RSPLITTER", "RMAINMENU", "RMENUITEM", "RMYSQL", "RSQLITE", "RCOOLBTN",
    "ROVALBTN", "ROPENDIALOG", "RSAVEDIALOG", "RFILEDIALOG", "RCOLORDIALOG", "RFONTDIALOG", "RFILESTREAM", "RJSON", "RHTTP", "RSOCKET", "$THEME", "LEFT", "RIGHT", "MID", "LEN", "INSTR", "UCASE", "LCASE", "VAL", "STR",
    "CHR", "ASC", "TRIM",
];

fn is_keyword(word: &str) -> bool {
    KEYWORDS.iter().any(|k| k.eq_ignore_ascii_case(word))
}

/// One line's tokens other than [`Token::Normal`], as byte ranges in order.
pub fn line_tokens(line: &str) -> Vec<(Range<usize>, Token)> {
    let b = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let digit = |i: usize| b.get(i).is_some_and(u8::is_ascii_digit);
    while i < b.len() {
        let start = i;
        let token = match b[i] {
            // a comment runs to the line's end
            b'\'' => {
                i = b.len();
                Token::Comment
            }
            // a string to its closing quote (or the line's end)
            b'"' => {
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    i += 1;
                }
                i = (i + 1).min(b.len());
                Token::String
            }
            c if c.is_ascii_digit() || (c == b'.' && digit(i + 1)) => {
                while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.') {
                    i += 1;
                }
                Token::Number
            }
            c if c.is_ascii_alphabetic() || c == b'_' || c == b'$' => {
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_' || b[i] == b'$') {
                    i += 1;
                }
                if !is_keyword(&line[start..i]) {
                    continue;
                }
                Token::Keyword
            }
            // anything else (a multi-byte character as a whole)
            _ => {
                i += line[i..].chars().next().map_or(1, char::len_utf8);
                continue;
            }
        };
        out.push((start..i, token));
    }
    out
}

/// A line's coloured pieces in `syntax`, from `state` (what the line
/// before left), and the state it leaves for the next.
pub fn spans(syntax: Syntax, line: &str, state: u32) -> (Vec<(Range<usize>, Token)>, u32) {
    match syntax {
        Syntax::None => (Vec::new(), state),
        Syntax::Basic => (line_tokens(line), 0),
    }
}

/// The SUB or FUNCTION a line declares (its name), if it does.
fn declared(line: &str) -> Option<&str> {
    let t = line.trim();
    let upper = t.to_ascii_uppercase();
    if !(upper.starts_with("SUB ") || upper.starts_with("FUNCTION ")) {
        return None;
    }
    t.split('(').next()?.split_whitespace().nth(1)
}

/// GetSubList: the SUBs' and FUNCTIONs' names, in order.
pub fn sub_list(text: &str) -> Vec<String> {
    text.lines().filter_map(declared).map(str::to_string).collect()
}

/// GotoSub's line: the first SUB / FUNCTION line whose text contains
/// `name` (any case).
pub fn sub_line(text: &str, name: &str) -> Option<usize> {
    let target = name.to_uppercase();
    text.lines().position(|l| declared(l).is_some() && l.trim().to_uppercase().contains(&target))
}

/// Where line `n` (from 0) starts, in characters ('\n' counts one); past
/// the last line, the text's end.
pub fn line_start(text: &str, n: usize) -> usize {
    let mut at = 0;
    for (i, line) in text.split('\n').enumerate() {
        if i == n {
            return at;
        }
        at += line.chars().count() + 1;
    }
    text.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_tokens_and_their_styles() {
        let line = "IF x1 = 10 THEN PRINT \"hi\" ' done";
        let t: Vec<(&str, Token)> = line_tokens(line).into_iter().map(|(r, t)| (&line[r], t)).collect();
        assert_eq!(t, [("IF", Token::Keyword), ("10", Token::Number), ("THEN", Token::Keyword), ("PRINT", Token::Keyword), ("\"hi\"", Token::String), ("' done", Token::Comment)]);
        // `LEFT$` is one word, not the keyword LEFT; a string left open
        // ends with the line; `.5` is a number
        assert_eq!(line_tokens("a$ = LEFT$(b$, 2)").len(), 1);
        assert_eq!(line_tokens("s = \"open").last().map(|(r, t)| (r.clone(), *t)), Some((4..9, Token::String)));
        assert_eq!(line_tokens("x=.5")[0], (2..4, Token::Number));
        assert_eq!(line_tokens("dim é AS integer").iter().map(|(_, t)| *t).collect::<Vec<_>>(), [Token::Keyword, Token::Keyword, Token::Keyword]);
        assert_eq!(spans(Syntax::Basic, "' x", 0).1, 0);
        assert!(spans(Syntax::None, "SUB", 0).0.is_empty());
        assert_eq!(Token::Comment.style(), Style { color: 0x008000, bold: false, italic: true });
    }

    #[test]
    fn subs_and_lines() {
        let src = "DIM a\nSUB Foo(x AS INTEGER)\nEND SUB\n  function Bar\nsubroutine\nSUB Foo2";
        assert_eq!(sub_list(src), ["Foo", "Bar", "Foo2"]);
        assert_eq!(sub_line(src, "bar"), Some(3));
        assert_eq!(sub_line(src, "foo2"), Some(5));
        assert_eq!(sub_line(src, "Baz"), None);
        assert_eq!(line_start(src, 0), 0);
        assert_eq!(line_start(src, 2), 28);
        assert_eq!(line_start(src, 99), src.chars().count());
    }
}
