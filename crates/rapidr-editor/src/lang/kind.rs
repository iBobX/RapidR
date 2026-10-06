//! Token kinds: a fixed vocabulary that colour schemes map to colours, plus
//! custom names under it (`keyword.tag`, `string.url` …) that fall back to
//! their base kind when a scheme doesn't name them.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};

/// At most this many kinds (fixed and custom).
const MAX_KINDS: usize = 4096;

/// Each kind's base, readable without a lock (`base()` runs per token).
static BASES: [AtomicU8; MAX_KINDS] = [const { AtomicU8::new(0) }; MAX_KINDS];

/// A token's kind (an interned name).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TokenKind(u16);

/// The fixed vocabulary, in [`TokenKind`]'s constants' order.
pub const FIXED_KINDS: &[&str] = &[
    "text",
    "keyword",
    "keyword.control",
    "type",
    "function",
    "variable",
    "constant",
    "number",
    "string",
    "string.escape",
    "comment",
    "operator",
    "punctuation",
    "directive",
    "label",
    "invalid",
];

struct Interner {
    names: Vec<&'static str>,
    index: HashMap<&'static str, u16>,
}

fn interner() -> &'static Mutex<Interner> {
    static I: OnceLock<Mutex<Interner>> = OnceLock::new();
    I.get_or_init(|| {
        let names: Vec<&'static str> = FIXED_KINDS.to_vec();
        for (i, n) in names.iter().enumerate() {
            let base = n.split('.').next().unwrap_or(n);
            BASES[i].store(FIXED_KINDS.iter().position(|k| *k == base).unwrap_or(0) as u8, Ordering::Relaxed);
        }
        let index = names.iter().enumerate().map(|(i, n)| (*n, i as u16)).collect();
        Mutex::new(Interner { names, index })
    })
}

fn valid_segment(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}

impl TokenKind {
    /// Plain text (not coloured; tokenizers don't emit it).
    pub const TEXT: TokenKind = TokenKind(0);
    pub const KEYWORD: TokenKind = TokenKind(1);
    pub const KEYWORD_CONTROL: TokenKind = TokenKind(2);
    pub const TYPE: TokenKind = TokenKind(3);
    pub const FUNCTION: TokenKind = TokenKind(4);
    pub const VARIABLE: TokenKind = TokenKind(5);
    pub const CONSTANT: TokenKind = TokenKind(6);
    pub const NUMBER: TokenKind = TokenKind(7);
    pub const STRING: TokenKind = TokenKind(8);
    pub const STRING_ESCAPE: TokenKind = TokenKind(9);
    pub const COMMENT: TokenKind = TokenKind(10);
    pub const OPERATOR: TokenKind = TokenKind(11);
    pub const PUNCTUATION: TokenKind = TokenKind(12);
    pub const DIRECTIVE: TokenKind = TokenKind(13);
    pub const LABEL: TokenKind = TokenKind(14);
    pub const INVALID: TokenKind = TokenKind(15);

    /// The kind called `name`: one of [`FIXED_KINDS`] or a custom name under
    /// one (`keyword.tag`, `string.url.x`). `None` for anything else.
    pub fn named(name: &str) -> Option<TokenKind> {
        let mut segments = name.split('.');
        let base = segments.next()?;
        if !FIXED_KINDS.contains(&base) || !name.split('.').all(valid_segment) {
            return None;
        }
        let mut i = interner().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(&k) = i.index.get(name) {
            return Some(TokenKind(k));
        }
        if i.names.len() >= MAX_KINDS {
            return None;
        }
        let leaked: &'static str = Box::leak(name.to_string().into_boxed_str());
        let k = i.names.len() as u16;
        BASES[k as usize].store(FIXED_KINDS.iter().position(|f| *f == base).unwrap_or(0) as u8, Ordering::Relaxed);
        i.names.push(leaked);
        i.index.insert(leaked, k);
        Some(TokenKind(k))
    }

    pub fn name(self) -> &'static str {
        interner().lock().unwrap_or_else(|e| e.into_inner()).names[self.0 as usize]
    }

    /// The kind's names from the most specific to its base (`keyword.tag`,
    /// `keyword`): what a colour scheme looks up, in order.
    pub fn fallbacks(self) -> Vec<&'static str> {
        let name = self.name();
        let mut out = vec![name];
        let mut rest = name;
        while let Some(i) = rest.rfind('.') {
            rest = &name[..i];
            out.push(rest);
        }
        out
    }

    /// The fixed kind this one is under (`string.escape` → `string`).
    pub fn base(self) -> TokenKind {
        // (the fixed kinds' bases are stored once the interner exists)
        let _ = interner();
        TokenKind(BASES[self.0 as usize].load(Ordering::Relaxed) as u16)
    }

    pub fn is_comment(self) -> bool {
        self.base() == TokenKind::COMMENT
    }

    pub fn is_string(self) -> bool {
        self.base() == TokenKind::STRING
    }

    /// Inside a comment or a string (where brackets and markers don't count).
    pub fn is_literal(self) -> bool {
        matches!(self.base(), TokenKind::COMMENT | TokenKind::STRING)
    }

    pub fn index(self) -> u16 {
        self.0
    }
}

/// A coloured piece of a line: bytes `start..end` of the line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Token {
    pub start: u32,
    pub end: u32,
    pub kind: TokenKind,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds() {
        assert_eq!(TokenKind::named("keyword.control"), Some(TokenKind::KEYWORD_CONTROL));
        assert_eq!(TokenKind::KEYWORD_CONTROL.base(), TokenKind::KEYWORD);
        assert_eq!(TokenKind::STRING_ESCAPE.base(), TokenKind::STRING);
        let tag = TokenKind::named("keyword.tag").unwrap();
        assert_eq!(TokenKind::named("keyword.tag"), Some(tag));
        assert_eq!(tag.base(), TokenKind::KEYWORD);
        assert_eq!(tag.fallbacks(), ["keyword.tag", "keyword"]);
        assert_eq!(TokenKind::named("heading"), None);
        assert_eq!(TokenKind::named("string.Url"), None);
        assert_eq!(TokenKind::named("string..x"), None);
        assert!(TokenKind::named("comment.quote").unwrap().is_literal());
    }
}
