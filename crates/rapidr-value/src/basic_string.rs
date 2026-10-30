//! [`BasicString`]: the text a `Value::String` holds.
//!
//! A BASIC string is shared, not copied: cloning a value (reading a
//! variable, passing it to a SUB, putting it in an array) only counts one
//! more reference, and the text is copied only when a shared string is
//! changed (copy on write). Positions in it count characters (MID$, LEFT$,
//! RIGHT$, LEN, INSTR — `strings`), while the text is UTF-8, where a
//! character takes 1 to 4 bytes; so a long string gets an index from
//! character positions to byte offsets the first time a position far into
//! it is asked for — once, then every MID$ on it costs what it returns, not
//! the string's length (a loop that reads a 64 KB file's string a character
//! at a time was quadratic before: 65,536 copies of the whole text).
//!
//! The index is part of the text it describes: any change to the text
//! (every `&mut` access goes through [`DerefMut`], which forgets it) drops
//! it, and a copy starts without one.

use std::borrow::{Borrow, Cow};
use std::cell::OnceCell;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

/// A BASIC string's text: shared, copied on write, with a character index
/// built on demand (module docs).
#[derive(Clone)]
pub struct BasicString(Rc<Body>);

thread_local! {
    /// The empty string, shared: `""` costs no allocation, as an empty
    /// `String` doesn't.
    static EMPTY: BasicString = BasicString(Rc::new(Body::default()));
}

impl Default for BasicString {
    fn default() -> Self {
        // (a thread ending, its locals going: one of its own)
        EMPTY.try_with(BasicString::clone).unwrap_or_else(|_| BasicString(Rc::new(Body::default())))
    }
}

#[derive(Default)]
struct Body {
    text: String,
    /// Where characters start, once a lookup needed it (long texts only).
    index: OnceCell<Box<CharIndex>>,
}

impl Clone for Body {
    /// (a copy made to be changed: it starts without an index)
    fn clone(&self) -> Self {
        Body { text: self.text.clone(), index: OnceCell::new() }
    }
}

/// Every how many characters the index notes a byte offset: a lookup walks
/// at most `STRIDE - 1` characters from the nearest note.
const STRIDE: usize = 32;

/// Positions before this many characters (and every position of a text of
/// at most this many bytes) are found by walking from the start: cheaper
/// than an index for the short strings most programs use.
const DIRECT: usize = 64;

/// The index of a long text.
struct CharIndex {
    /// How many characters the text has.
    chars: usize,
    /// `None`: every character is one byte (ASCII), a position is its
    /// offset. `Some(marks)`: `marks[j]` is the byte offset of character
    /// `j * STRIDE`.
    marks: Option<Box<[u32]>>,
}

impl CharIndex {
    fn build(text: &str) -> CharIndex {
        if text.is_ascii() {
            return CharIndex { chars: text.len(), marks: None };
        }
        let mut marks = Vec::with_capacity(text.len() / STRIDE + 1);
        let mut chars = 0usize;
        for (at, &b) in text.as_bytes().iter().enumerate() {
            if is_char_start(b) {
                if chars.is_multiple_of(STRIDE) {
                    marks.push(at as u32);
                }
                chars += 1;
            }
        }
        CharIndex { chars, marks: Some(marks.into_boxed_slice()) }
    }
}

/// Whether `b` begins a character (isn't a UTF-8 continuation byte).
#[inline]
fn is_char_start(b: u8) -> bool {
    (b as i8) >= -0x40
}

/// How many bytes the character beginning with byte `b` takes.
#[inline]
fn width(b: u8) -> usize {
    match b {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        _ => 4,
    }
}

/// The byte offset `n` characters after byte offset `from` (a character's
/// start) in `text`, or the text's length when it has fewer.
#[inline]
fn walk(text: &str, from: usize, n: usize) -> usize {
    let bytes = text.as_bytes();
    let mut at = from;
    for _ in 0..n {
        match bytes.get(at) {
            Some(&b) => at += width(b),
            None => return text.len(),
        }
    }
    at.min(text.len())
}

/// Byte offset where character `n` (0-based) of `text` starts — the text's
/// length when it has `n` characters or fewer — walking from the start
/// (for texts without an index: short ones, other values' text).
pub fn byte_of_char_in(text: &str, n: usize) -> usize {
    if n >= text.len() {
        return text.len();
    }
    walk(text, 0, n)
}

/// The byte offset `n` characters back from the end of `text`.
pub fn byte_of_char_from_end(text: &str, n: usize) -> usize {
    if n == 0 {
        return text.len();
    }
    text.char_indices().rev().nth(n - 1).map_or(0, |(i, _)| i)
}

impl BasicString {
    pub fn new() -> BasicString {
        BasicString::default()
    }

    pub fn as_str(&self) -> &str {
        &self.0.text
    }

    /// The text as an owned `String` (no copy when this was its only
    /// holder).
    pub fn into_string(self) -> String {
        match Rc::try_unwrap(self.0) {
            Ok(body) => body.text,
            Err(shared) => shared.text.clone(),
        }
    }

    /// Whether `a` and `b` are the same text in memory (one shared string,
    /// not two equal ones).
    pub fn ptr_eq(a: &BasicString, b: &BasicString) -> bool {
        Rc::ptr_eq(&a.0, &b.0)
    }

    /// Whether this is the text's only holder (changing it then copies
    /// nothing).
    pub fn is_unique(&self) -> bool {
        Rc::strong_count(&self.0) == 1
    }

    /// The index, built the first time (`None` for a text too short to
    /// need one, or too long for 32-bit offsets).
    fn index(&self) -> Option<&CharIndex> {
        let text = &self.0.text;
        if text.len() <= DIRECT || text.len() > u32::MAX as usize {
            return None;
        }
        Some(self.0.index.get_or_init(|| Box::new(CharIndex::build(text))))
    }

    /// How many characters the text has (`LEN`): counted once for a long
    /// text, then kept.
    pub fn char_count(&self) -> usize {
        match self.index() {
            Some(ix) => ix.chars,
            None => self.0.text.chars().count(),
        }
    }

    /// Byte offset where character `n` (0-based) starts; the text's length
    /// when it has `n` characters or fewer.
    pub fn byte_of_char(&self, n: usize) -> usize {
        let text = &self.0.text;
        if n < DIRECT {
            return byte_of_char_in(text, n);
        }
        match self.index() {
            None => byte_of_char_in(text, n),
            Some(ix) if n >= ix.chars => text.len(),
            Some(CharIndex { marks: None, .. }) => n,
            Some(CharIndex { marks: Some(marks), .. }) => walk(text, marks[n / STRIDE] as usize, n % STRIDE),
        }
    }

    /// Which character (0-based) begins at byte offset `at` (a character's
    /// start, or the text's length: then the number of characters).
    pub fn char_of_byte(&self, at: usize) -> usize {
        let text = &self.0.text;
        let at = at.min(text.len());
        match self.index() {
            None => text[..at].chars().count(),
            Some(CharIndex { marks: None, .. }) => at,
            Some(CharIndex { marks: Some(marks), .. }) => {
                // (the last note at or before `at`, then characters from it)
                let j = marks.partition_point(|&m| m as usize <= at).saturating_sub(1);
                j * STRIDE + text[marks[j] as usize..at].chars().count()
            }
        }
    }

    /// Characters `from .. from + n` (0-based, clamped to the text) as byte
    /// offsets: the start through the index, the end walked (no further
    /// than the characters a copy of the range goes through anyway).
    pub fn byte_range(&self, from: usize, n: usize) -> std::ops::Range<usize> {
        let start = self.byte_of_char(from);
        start..walk(&self.0.text, start, n)
    }

    /// Appends `s` in place: no copy of the text when this is its only
    /// holder (`s = s + x` in a loop).
    pub fn push_str(&mut self, s: &str) {
        (**self).push_str(s);
    }
}

/// A text read through a `BasicString` (its index), or any other text
/// (walked from the start): what the string functions work on.
pub enum Chars<'a> {
    Indexed(&'a BasicString),
    Plain(Cow<'a, str>),
}

impl Chars<'_> {
    pub fn as_str(&self) -> &str {
        match self {
            Chars::Indexed(s) => s.as_str(),
            Chars::Plain(s) => s,
        }
    }

    pub fn char_count(&self) -> usize {
        match self {
            Chars::Indexed(s) => s.char_count(),
            Chars::Plain(s) => s.chars().count(),
        }
    }

    pub fn byte_of_char(&self, n: usize) -> usize {
        match self {
            Chars::Indexed(s) => s.byte_of_char(n),
            Chars::Plain(s) => byte_of_char_in(s, n),
        }
    }

    pub fn char_of_byte(&self, at: usize) -> usize {
        match self {
            Chars::Indexed(s) => s.char_of_byte(at),
            Chars::Plain(s) => s[..at.min(s.len())].chars().count(),
        }
    }

    pub fn byte_range(&self, from: usize, n: usize) -> std::ops::Range<usize> {
        match self {
            Chars::Indexed(s) => s.byte_range(from, n),
            Chars::Plain(s) => {
                let start = byte_of_char_in(s, from);
                start..walk(s, start, n)
            }
        }
    }
}

impl Deref for BasicString {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0.text
    }
}

impl DerefMut for BasicString {
    /// Changing the text: copied first if shared, its index forgotten.
    fn deref_mut(&mut self) -> &mut String {
        let body = Rc::make_mut(&mut self.0);
        body.index.take();
        &mut body.text
    }
}

impl fmt::Debug for BasicString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl fmt::Display for BasicString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl PartialEq for BasicString {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0) || self.as_str() == other.as_str()
    }
}

impl Eq for BasicString {}

impl PartialOrd for BasicString {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BasicString {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl Hash for BasicString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}

impl PartialEq<str> for BasicString {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for BasicString {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<String> for BasicString {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<BasicString> for str {
    fn eq(&self, other: &BasicString) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<BasicString> for &str {
    fn eq(&self, other: &BasicString) -> bool {
        *self == other.as_str()
    }
}

impl PartialEq<BasicString> for String {
    fn eq(&self, other: &BasicString) -> bool {
        self.as_str() == other.as_str()
    }
}

impl AsRef<str> for BasicString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<std::path::Path> for BasicString {
    fn as_ref(&self) -> &std::path::Path {
        self.as_str().as_ref()
    }
}

impl AsRef<std::ffi::OsStr> for BasicString {
    fn as_ref(&self) -> &std::ffi::OsStr {
        self.as_str().as_ref()
    }
}

impl AsRef<[u8]> for BasicString {
    fn as_ref(&self) -> &[u8] {
        self.as_str().as_bytes()
    }
}

impl Borrow<str> for BasicString {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl From<String> for BasicString {
    fn from(text: String) -> Self {
        if text.capacity() == 0 {
            return BasicString::default();
        }
        BasicString(Rc::new(Body { text, index: OnceCell::new() }))
    }
}

impl From<&str> for BasicString {
    fn from(text: &str) -> Self {
        BasicString::from(text.to_string())
    }
}

impl From<&String> for BasicString {
    fn from(text: &String) -> Self {
        BasicString::from(text.clone())
    }
}

impl From<&mut str> for BasicString {
    fn from(text: &mut str) -> Self {
        BasicString::from(text.to_string())
    }
}

impl From<char> for BasicString {
    fn from(c: char) -> Self {
        BasicString::from(String::from(c))
    }
}

impl From<Box<str>> for BasicString {
    fn from(text: Box<str>) -> Self {
        BasicString::from(String::from(text))
    }
}

impl From<Cow<'_, str>> for BasicString {
    fn from(text: Cow<'_, str>) -> Self {
        BasicString::from(text.into_owned())
    }
}

impl From<BasicString> for String {
    fn from(s: BasicString) -> Self {
        s.into_string()
    }
}

impl From<&BasicString> for String {
    fn from(s: &BasicString) -> Self {
        s.as_str().to_string()
    }
}

impl FromIterator<char> for BasicString {
    fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Self {
        BasicString::from(String::from_iter(iter))
    }
}

impl<'a> FromIterator<&'a char> for BasicString {
    fn from_iter<I: IntoIterator<Item = &'a char>>(iter: I) -> Self {
        BasicString::from(String::from_iter(iter))
    }
}

impl<'a> FromIterator<&'a str> for BasicString {
    fn from_iter<I: IntoIterator<Item = &'a str>>(iter: I) -> Self {
        BasicString::from(String::from_iter(iter))
    }
}

impl FromIterator<String> for BasicString {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        BasicString::from(String::from_iter(iter))
    }
}

impl<'a> FromIterator<Cow<'a, str>> for BasicString {
    fn from_iter<I: IntoIterator<Item = Cow<'a, str>>>(iter: I) -> Self {
        BasicString::from(String::from_iter(iter))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every position of `text` through the index agrees with walking the
    /// characters one by one.
    fn check_positions(text: &str) {
        let s = BasicString::from(text);
        let starts: Vec<usize> = text.char_indices().map(|(i, _)| i).chain([text.len()]).collect();
        let n = starts.len() - 1;
        assert_eq!(s.char_count(), n, "LEN of a {}-byte text", text.len());
        for (c, &b) in starts.iter().enumerate() {
            assert_eq!(s.byte_of_char(c), b, "character {c} of {n}");
            assert_eq!(s.char_of_byte(b), c, "byte {b}");
        }
        assert_eq!(s.byte_of_char(n + 5), text.len());
        for from in [0, 1, 31, 32, 33, 63, 64, 65, n / 2, n.saturating_sub(1), n] {
            for k in [0, 1, 2, 31, 32, 64, 65, 200, n] {
                let want = starts[from.min(n)]..starts[(from + k).min(n)];
                assert_eq!(s.byte_range(from, k), want, "range {from}+{k}");
            }
        }
    }

    #[test]
    fn positions_in_ascii_texts() {
        for len in [0, 1, 63, 64, 65, 300, 5000] {
            check_positions(&"abcdefghij".repeat(len / 10 + 1)[..len]);
        }
    }

    #[test]
    fn positions_in_mixed_texts() {
        // (bytes as CHR$ makes them — 0..255, half of them two bytes in
        // UTF-8 — and wider characters: accents, CJK, emoji)
        let bytes: String = (0..3000u32).map(|i| char::from_u32((i * 7919) % 256).unwrap()).collect();
        check_positions(&bytes);
        let mixed: String = (0..2000).map(|i| ['a', 'é', '€', '😀', 'Z', '\u{0}', 'ñ'][i % 7]).collect();
        check_positions(&mixed);
        check_positions("ñandú");
        check_positions(&format!("{}é", "a".repeat(255)));
        check_positions(&format!("é{}", "a".repeat(300)));
        check_positions(&"😀".repeat(100));
    }

    #[test]
    fn a_change_forgets_the_index() {
        let mut s = BasicString::from("é".repeat(200));
        assert_eq!(s.byte_of_char(150), 300);
        s.push_str(&"a".repeat(10));
        s.insert(0, 'x');
        assert_eq!(s.char_count(), 211);
        assert_eq!(s.byte_of_char(150), 1 + 298);
        s.clear();
        s.push_str(&"b".repeat(100));
        assert_eq!(s.char_count(), 100);
        assert_eq!(s.byte_of_char(80), 80);
    }

    #[test]
    fn shared_until_changed() {
        let a = BasicString::from("x".repeat(1000));
        let mut b = a.clone();
        assert!(BasicString::ptr_eq(&a, &b));
        assert!(!a.is_unique());
        b.push_str("y");
        assert!(!BasicString::ptr_eq(&a, &b));
        assert_eq!(a.len(), 1000);
        assert_eq!(b.len(), 1001);
        assert!(a.is_unique() && b.is_unique());
        assert_eq!(b.into_string().len(), 1001);
    }

    #[test]
    fn compares_and_prints_as_its_text() {
        let s = BasicString::from("héllo");
        assert_eq!(s, "héllo");
        assert_eq!("héllo", s);
        assert_eq!(s, String::from("héllo"));
        assert_eq!(format!("{s}|{s:?}"), "héllo|\"héllo\"");
        let (a, b) = (BasicString::from("a"), BasicString::from("b"));
        assert!(a < b);
    }
}
