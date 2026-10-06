//! Property values as a CREATE block writes them (`Left = 10`, `Anchors =
//! akLeft + akRight`, `Caption = "OK"`, `Width = ClientWidth / 2`): what the
//! designer can read of them without running the program, and how it writes
//! the values it sets.
//!
//! Reading covers numbers (`10`, `-3`, `75.5`, `&HFF00`), the registry's
//! constants (`alClient`, `akRight`, `clRed`, `True` / `False`) and
//! arithmetic on them (`+ - * / \ MOD AND OR XOR`, parentheses). Anything
//! else (`Screen.Width / 2`, a variable) is "set in code": the designer
//! keeps the text and reads no number from it.

use crate::layout::{AK_BOTTOM, AK_LEFT, AK_RIGHT, AK_TOP};

/// A value read from its text.
#[derive(Clone, Debug, PartialEq)]
pub enum PropValue {
    Number(f64),
    Str(String),
    /// A bare name the designer doesn't know the value of (an event
    /// handler: `OnClick = Button1Click`).
    Name(String),
    /// An expression the designer can't evaluate.
    Code,
}

impl PropValue {
    /// The value as an integer property stores it (rounded half to even,
    /// as the runtimes store Left / Top / Width / Height).
    pub fn int(&self) -> Option<i64> {
        match self {
            PropValue::Number(f) => Some(crate::numeric::round_to_int(*f)),
            _ => None,
        }
    }
}

/// Reads a value's text.
pub fn read(text: &str) -> PropValue {
    let t = text.trim();
    if let Some(s) = string_literal(t) {
        return PropValue::Str(s);
    }
    let tokens = match tokenize(t) {
        Some(tokens) if !tokens.is_empty() => tokens,
        _ => return PropValue::Code,
    };
    let mut p = Parser { tokens: &tokens, at: 0 };
    match p.expr() {
        Some(v) if p.at == tokens.len() => PropValue::Number(v),
        _ => match tokens.as_slice() {
            [Tok::Name(n)] => PropValue::Name(n.clone()),
            _ => PropValue::Code,
        },
    }
}

/// An integer value's text, or `None` when it isn't one the designer can
/// read.
pub fn int(text: &str) -> Option<i64> {
    read(text).int()
}

/// A whole string literal (`"OK"`, `"say ""hi"""`): its text.
fn string_literal(t: &str) -> Option<String> {
    let inner = t.strip_prefix('"')?.strip_suffix('"')?;
    // (`"a" + "b"` isn't one literal)
    let mut out = String::new();
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' {
            if chars.next() != Some('"') {
                return None;
            }
        }
        out.push(c);
    }
    Some(out)
}

/// How the designer writes a string: in quotes, a quote doubled.
pub fn write_str(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

/// How the designer writes Anchors: RapidR's constants added, in Delphi's
/// order (`akLeft + akTop + akRight`); `0` for none.
pub fn write_anchors(bits: i64) -> String {
    let names: Vec<&str> = [(AK_LEFT, "akLeft"), (AK_TOP, "akTop"), (AK_RIGHT, "akRight"), (AK_BOTTOM, "akBottom")].iter().filter(|(b, _)| bits & b != 0).map(|(_, n)| *n).collect();
    if names.is_empty() {
        "0".into()
    } else {
        names.join(" + ")
    }
}

/// How the designer writes an Align (RAPIDQ.INC's constants).
pub fn write_align(align: crate::layout::Align) -> String {
    use crate::layout::Align::*;
    match align {
        None => "alNone",
        Top => "alTop",
        Bottom => "alBottom",
        Left => "alLeft",
        Right => "alRight",
        Client => "alClient",
    }
    .into()
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Name(String),
    Op(char),
    /// MOD, AND, OR, XOR (upper case), `\`.
    Word(&'static str),
    Open,
    Close,
}

fn tokenize(t: &str) -> Option<Vec<Tok>> {
    let b = t.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < b.len() {
        let c = b[i] as char;
        if c.is_ascii_whitespace() {
            i += 1;
        } else if c == '&' && matches!(b.get(i + 1), Some(b'H' | b'h')) {
            let start = i + 2;
            let mut j = start;
            while j < b.len() && (b[j] as char).is_ascii_hexdigit() {
                j += 1;
            }
            let v = i64::from_str_radix(&t[start..j], 16).ok()?;
            // (`&HFF&`: a type suffix)
            if j < b.len() && b[j] == b'&' {
                j += 1;
            }
            out.push(Tok::Num(v as f64));
            i = j;
        } else if c.is_ascii_digit() || c == '.' {
            let mut j = i;
            while j < b.len() && ((b[j] as char).is_ascii_digit() || b[j] == b'.') {
                j += 1;
            }
            out.push(Tok::Num(t[i..j].parse().ok()?));
            // (a type suffix: 10&, 2.5#)
            if j < b.len() && matches!(b[j], b'%' | b'&' | b'!' | b'#') {
                j += 1;
            }
            i = j;
        } else if c.is_ascii_alphabetic() || c == '_' {
            let mut j = i;
            while j < b.len() && ((b[j] as char).is_ascii_alphanumeric() || b[j] == b'_' || b[j] == b'.') {
                j += 1;
            }
            let word = &t[i..j];
            let tok = match word.to_ascii_uppercase().as_str() {
                "MOD" => Tok::Word("MOD"),
                "AND" => Tok::Word("AND"),
                "OR" => Tok::Word("OR"),
                "XOR" => Tok::Word("XOR"),
                _ => Tok::Name(word.to_string()),
            };
            out.push(tok);
            i = j;
        } else {
            out.push(match c {
                '+' | '-' | '*' | '/' => Tok::Op(c),
                '\\' => Tok::Word("\\"),
                '(' => Tok::Open,
                ')' => Tok::Close,
                _ => return None,
            });
            i += 1;
        }
    }
    Some(out)
}

struct Parser<'a> {
    tokens: &'a [Tok],
    at: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.tokens.get(self.at)
    }

    /// OR / XOR, then AND, then + -, then * / \ MOD, then unary.
    fn expr(&mut self) -> Option<f64> {
        let mut v = self.and()?;
        while let Some(Tok::Word(w @ ("OR" | "XOR"))) = self.peek().cloned() {
            self.at += 1;
            let r = self.and()?;
            v = if w == "OR" { (v as i64 | r as i64) as f64 } else { (v as i64 ^ r as i64) as f64 };
        }
        Some(v)
    }

    fn and(&mut self) -> Option<f64> {
        let mut v = self.sum()?;
        while let Some(Tok::Word("AND")) = self.peek() {
            self.at += 1;
            let r = self.sum()?;
            v = (v as i64 & r as i64) as f64;
        }
        Some(v)
    }

    fn sum(&mut self) -> Option<f64> {
        let mut v = self.product()?;
        while let Some(Tok::Op(op @ ('+' | '-'))) = self.peek().cloned() {
            self.at += 1;
            let r = self.product()?;
            v = if op == '+' { v + r } else { v - r };
        }
        Some(v)
    }

    fn product(&mut self) -> Option<f64> {
        let mut v = self.unary()?;
        loop {
            match self.peek().cloned() {
                Some(Tok::Op('*')) => {
                    self.at += 1;
                    v *= self.unary()?;
                }
                Some(Tok::Op('/')) => {
                    self.at += 1;
                    let r = self.unary()?;
                    if r == 0.0 {
                        return None;
                    }
                    v /= r;
                }
                Some(Tok::Word(w @ ("\\" | "MOD"))) => {
                    self.at += 1;
                    let r = self.unary()? as i64;
                    if r == 0 {
                        return None;
                    }
                    v = if w == "MOD" { (v as i64 % r) as f64 } else { (v as i64 / r) as f64 };
                }
                _ => return Some(v),
            }
        }
    }

    fn unary(&mut self) -> Option<f64> {
        match self.peek().cloned()? {
            Tok::Op('-') => {
                self.at += 1;
                Some(-self.unary()?)
            }
            Tok::Op('+') => {
                self.at += 1;
                self.unary()
            }
            Tok::Num(n) => {
                self.at += 1;
                Some(n)
            }
            Tok::Open => {
                self.at += 1;
                let v = self.expr()?;
                (self.peek() == Some(&Tok::Close)).then(|| self.at += 1)?;
                Some(v)
            }
            Tok::Name(n) => {
                let v = match n.to_ascii_uppercase().as_str() {
                    "TRUE" => 1.0,
                    "FALSE" => 0.0,
                    _ => rapidr_lang::constant(&n)?.0 as f64,
                };
                self.at += 1;
                Some(v)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_what_create_blocks_write() {
        assert_eq!(int("10"), Some(10));
        assert_eq!(int(" -3 "), Some(-3));
        assert_eq!(int("75.5"), Some(76), "half to even: 75.5 → 76");
        assert_eq!(int("74.5"), Some(74));
        assert_eq!(int("&HFF"), Some(255));
        assert_eq!(int("akLeft + akTop + akRight"), Some(7));
        assert_eq!(int("akLeft OR akBottom"), Some(9));
        assert_eq!(int("alClient"), Some(5));
        assert_eq!(int("(10 + 2) * 3 - 6 / 2"), Some(33));
        assert_eq!(int("7 \\ 2 + 7 MOD 4"), Some(6));
        assert_eq!(int("True"), Some(1));
        assert_eq!(read("\"say \"\"hi\"\"\""), PropValue::Str("say \"hi\"".into()));
        assert_eq!(read("Button1Click"), PropValue::Name("Button1Click".into()));
        assert_eq!(read("Screen.Width / 2"), PropValue::Code);
        assert_eq!(read("\"a\" + \"b\""), PropValue::Code);
        assert_eq!(read("1 / 0"), PropValue::Code);
        assert_eq!(write_str("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(write_anchors(AK_LEFT | AK_TOP | AK_RIGHT), "akLeft + akTop + akRight");
        assert_eq!(write_anchors(0), "0");
        assert_eq!(int(&write_anchors(15)), Some(15));
        assert_eq!(write_align(crate::layout::Align::Client), "alClient");
    }
}
