//! The text: our own [`Buffer`] trait over a rope (decision D6: ropey 1.6),
//! so the rope can change (ropey 2, crop) without touching the editor.
//!
//! Positions are byte offsets into the whole text. Lines break at LF, CR LF
//! and a lone CR; a line's text never includes its break. Columns for the
//! language server protocol and the browser are UTF-16 code units, computed
//! on demand.

use std::borrow::Cow;
use std::ops::Range;

use ropey::Rope;

/// The line break a file is written with. A file keeps its own: loading
/// detects it, new lines are typed with it, the text is never normalized.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LineEnding {
    #[default]
    Lf,
    /// Windows' (and the `.bas` files RapidQ's tools write).
    CrLf,
    /// Classic Mac OS.
    Cr,
}

impl LineEnding {
    pub fn as_str(self) -> &'static str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::CrLf => "\r\n",
            LineEnding::Cr => "\r",
        }
    }

    /// The most common break in `text` (LF when there is none or a tie).
    pub fn detect(text: &str) -> LineEnding {
        let (mut lf, mut crlf, mut cr) = (0usize, 0usize, 0usize);
        let b = text.as_bytes();
        let mut i = 0;
        while i < b.len() {
            match b[i] {
                b'\n' => lf += 1,
                b'\r' if b.get(i + 1) == Some(&b'\n') => {
                    crlf += 1;
                    i += 1;
                }
                b'\r' => cr += 1,
                _ => {}
            }
            i += 1;
        }
        if crlf > lf && crlf >= cr {
            LineEnding::CrLf
        } else if cr > lf && cr > crlf {
            LineEnding::Cr
        } else {
            LineEnding::Lf
        }
    }
}

/// A line / column position. `column` is in bytes from the line's start
/// unless a function says otherwise.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

/// The editor's text storage. Every offset is a byte offset on a character
/// boundary; out-of-range offsets are clamped by the provided helpers, never
/// by the required methods (which may panic, as slices do).
pub trait Buffer {
    fn len_bytes(&self) -> usize;
    /// At least 1: an empty text is one empty line, and a text ending in a
    /// line break ends with an empty line.
    fn len_lines(&self) -> usize;
    /// Where line `line` starts (`line == len_lines()` gives the text's end).
    fn line_start(&self, line: usize) -> usize;
    /// The line `byte` is on (a line break belongs to the line it ends).
    fn line_of(&self, byte: usize) -> usize;
    /// Line `line`'s text without its break.
    fn line_text(&self, line: usize) -> Cow<'_, str>;
    fn slice(&self, range: Range<usize>) -> Cow<'_, str>;
    /// The whole text.
    fn text(&self) -> Cow<'_, str>;
    /// Replaces `range` with `text`.
    fn replace(&mut self, range: Range<usize>, text: &str);
    fn char_before(&self, byte: usize) -> Option<char>;
    fn char_after(&self, byte: usize) -> Option<char>;
    fn is_char_boundary(&self, byte: usize) -> bool;
    fn byte_to_char(&self, byte: usize) -> usize;
    fn char_to_byte(&self, char_index: usize) -> usize;
    /// UTF-16 code units before `byte`.
    fn byte_to_utf16(&self, byte: usize) -> usize;
    /// The byte offset of UTF-16 code unit `units` (rounded down to a
    /// character).
    fn utf16_to_byte(&self, units: usize) -> usize;

    // ---- provided ----

    /// Where line `line`'s text ends (before its break).
    fn line_end(&self, line: usize) -> usize {
        let start = self.line_start(line);
        start + self.line_text(line).len()
    }

    /// `byte` as a line and a byte column.
    fn position(&self, byte: usize) -> Position {
        let line = self.line_of(byte);
        Position { line, column: byte - self.line_start(line) }
    }

    /// The byte offset of a line and byte column, clamped to the line.
    fn offset(&self, pos: Position) -> usize {
        let line = pos.line.min(self.len_lines() - 1);
        let start = self.line_start(line);
        self.clamp(start + pos.column.min(self.line_end(line) - start))
    }

    /// `byte` as a line and a UTF-16 column (the language server protocol's
    /// and JavaScript's).
    fn utf16_position(&self, byte: usize) -> Position {
        let line = self.line_of(byte);
        let start = self.line_start(line);
        Position { line, column: self.byte_to_utf16(byte) - self.byte_to_utf16(start) }
    }

    /// The byte offset of a line and UTF-16 column, clamped to the line.
    fn offset_from_utf16(&self, pos: Position) -> usize {
        let line = pos.line.min(self.len_lines() - 1);
        let start = self.line_start(line);
        let end = self.line_end(line);
        let at = self.utf16_to_byte(self.byte_to_utf16(start) + pos.column);
        at.clamp(start, end)
    }

    /// `byte` moved to a valid caret position: inside the text, on a
    /// character boundary (down), never between a CR and its LF.
    fn clamp(&self, byte: usize) -> usize {
        let mut b = byte.min(self.len_bytes());
        while !self.is_char_boundary(b) {
            b -= 1;
        }
        if b > 0 && self.char_before(b) == Some('\r') && self.char_after(b) == Some('\n') {
            b -= 1;
        }
        b
    }

    /// The previous caret position (a CR LF is one step).
    fn prev_boundary(&self, byte: usize) -> usize {
        match self.char_before(byte) {
            None => byte,
            Some('\n') if byte >= 2 && self.char_before(byte - 1) == Some('\r') => byte - 2,
            Some(c) => byte - c.len_utf8(),
        }
    }

    /// The next caret position (a CR LF is one step).
    fn next_boundary(&self, byte: usize) -> usize {
        match self.char_after(byte) {
            None => byte,
            Some('\r') if self.char_after(byte + 1) == Some('\n') => byte + 2,
            Some(c) => byte + c.len_utf8(),
        }
    }
}

/// The rope buffer (ropey 1.6).
#[derive(Clone, Debug, Default)]
pub struct RopeBuffer {
    rope: Rope,
}

impl RopeBuffer {
    pub fn new(text: &str) -> Self {
        RopeBuffer { rope: Rope::from_str(text) }
    }

    /// Heap bytes the rope holds (for the memory budget, docs/ide-plan.md
    /// §6.2): its text plus its tree, approximately.
    pub fn approx_heap_bytes(&self) -> usize {
        // ropey's leaves hold up to ~1 KB of text; nodes are small next to
        // them. `capacity` counts the leaves' allocated bytes.
        self.rope.capacity() + self.rope.capacity() / 16
    }

    /// The rope's text in its chunks, in order (to write a file without a
    /// copy).
    pub fn chunks(&self) -> impl Iterator<Item = &str> {
        self.rope.chunks()
    }
}

fn strip_break(s: &str) -> &str {
    s.strip_suffix("\r\n").or_else(|| s.strip_suffix('\n')).or_else(|| s.strip_suffix('\r')).unwrap_or(s)
}

impl Buffer for RopeBuffer {
    fn len_bytes(&self) -> usize {
        self.rope.len_bytes()
    }

    fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    fn line_start(&self, line: usize) -> usize {
        self.rope.line_to_byte(line)
    }

    fn line_of(&self, byte: usize) -> usize {
        self.rope.byte_to_line(byte.min(self.rope.len_bytes()))
    }

    fn line_text(&self, line: usize) -> Cow<'_, str> {
        let slice = self.rope.line(line);
        match slice.as_str() {
            Some(s) => Cow::Borrowed(strip_break(s)),
            None => {
                let mut s: String = slice.into();
                let keep = strip_break(&s).len();
                s.truncate(keep);
                Cow::Owned(s)
            }
        }
    }

    fn slice(&self, range: Range<usize>) -> Cow<'_, str> {
        self.rope.byte_slice(range).into()
    }

    fn text(&self) -> Cow<'_, str> {
        (&self.rope).into()
    }

    fn replace(&mut self, range: Range<usize>, text: &str) {
        let start = self.rope.byte_to_char(range.start);
        if range.end > range.start {
            let end = self.rope.byte_to_char(range.end);
            self.rope.remove(start..end);
        }
        if !text.is_empty() {
            self.rope.insert(start, text);
        }
    }

    fn char_before(&self, byte: usize) -> Option<char> {
        if byte == 0 || byte > self.rope.len_bytes() {
            return None;
        }
        let c = self.rope.byte_to_char(byte);
        // (a byte inside a character belongs to that character)
        let c = if self.rope.char_to_byte(c) == byte { c.checked_sub(1)? } else { c };
        Some(self.rope.char(c))
    }

    fn char_after(&self, byte: usize) -> Option<char> {
        if byte >= self.rope.len_bytes() {
            return None;
        }
        self.rope.get_char(self.rope.byte_to_char(byte))
    }

    fn is_char_boundary(&self, byte: usize) -> bool {
        if byte >= self.rope.len_bytes() {
            return byte == self.rope.len_bytes();
        }
        // UTF-8 continuation bytes are 0b10xx_xxxx
        self.rope.byte(byte) & 0xC0 != 0x80
    }

    fn byte_to_char(&self, byte: usize) -> usize {
        self.rope.byte_to_char(byte)
    }

    fn char_to_byte(&self, char_index: usize) -> usize {
        self.rope.char_to_byte(char_index)
    }

    fn byte_to_utf16(&self, byte: usize) -> usize {
        self.rope.char_to_utf16_cu(self.rope.byte_to_char(byte))
    }

    fn utf16_to_byte(&self, units: usize) -> usize {
        let units = units.min(self.rope.len_utf16_cu());
        self.rope.char_to_byte(self.rope.utf16_cu_to_char(units))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_and_breaks() {
        let b = RopeBuffer::new("ab\r\ncd\nef\rgh");
        assert_eq!(b.len_lines(), 4);
        assert_eq!(b.line_text(0), "ab");
        assert_eq!(b.line_text(1), "cd");
        assert_eq!(b.line_text(2), "ef");
        assert_eq!(b.line_text(3), "gh");
        assert_eq!(b.line_start(1), 4);
        assert_eq!(b.line_end(0), 2);
        assert_eq!(b.line_of(3), 0); // the LF of CR LF is on line 0
        assert_eq!(RopeBuffer::new("").len_lines(), 1);
        assert_eq!(RopeBuffer::new("a\n").len_lines(), 2);
        // a caret never lands between CR and LF
        assert_eq!(b.clamp(3), 2);
        assert_eq!(b.next_boundary(2), 4);
        assert_eq!(b.prev_boundary(4), 2);
    }

    #[test]
    fn detect_line_endings() {
        assert_eq!(LineEnding::detect("a\r\nb\r\nc\n"), LineEnding::CrLf);
        assert_eq!(LineEnding::detect("a\nb"), LineEnding::Lf);
        assert_eq!(LineEnding::detect("a\rb\r"), LineEnding::Cr);
        assert_eq!(LineEnding::detect(""), LineEnding::Lf);
    }

    #[test]
    fn utf16_columns() {
        // é is 2 bytes / 1 unit, 😀 is 4 bytes / 2 units
        let b = RopeBuffer::new("x\né😀z");
        let z = b.len_bytes() - 1;
        assert_eq!(b.utf16_position(z), Position { line: 1, column: 3 });
        assert_eq!(b.offset_from_utf16(Position { line: 1, column: 3 }), z);
        assert_eq!(b.offset_from_utf16(Position { line: 1, column: 99 }), b.len_bytes());
        // inside a surrogate pair: the character's start
        assert_eq!(b.offset_from_utf16(Position { line: 1, column: 2 }), 4);
        assert_eq!(b.char_before(z), Some('😀'));
        assert_eq!(b.char_after(2), Some('é'));
        assert!(!b.is_char_boundary(3));
        assert_eq!(b.clamp(3), 2);
    }

    #[test]
    fn replace_and_slice() {
        let mut b = RopeBuffer::new("hello world");
        b.replace(0..5, "goodbye");
        assert_eq!(b.text(), "goodbye world");
        b.replace(7..7, ",");
        assert_eq!(b.slice(0..8), "goodbye,");
        b.replace(0..9, "");
        assert_eq!(b.text(), "world");
    }
}
