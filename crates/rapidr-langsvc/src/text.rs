//! Positions in text: byte offsets, lines, and the UTF-16 columns editors
//! speak (LSP's default position encoding).

/// Where each line of a text starts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineIndex {
    starts: Vec<usize>,
    len: usize,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        let mut starts = vec![0];
        starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        LineIndex { starts, len: text.len() }
    }

    pub fn line_count(&self) -> usize {
        self.starts.len()
    }

    /// The 0-based line and byte column of `offset`.
    pub fn line_col(&self, offset: usize) -> (usize, usize) {
        let offset = offset.min(self.len);
        let line = self.starts.partition_point(|&s| s <= offset).saturating_sub(1);
        (line, offset - self.starts[line])
    }

    pub fn line_start(&self, line: usize) -> Option<usize> {
        self.starts.get(line).copied()
    }

    /// The end of a line, before its `\r\n` / `\n`.
    pub fn line_end(&self, text: &str, line: usize) -> usize {
        let next = self.starts.get(line + 1).map_or(text.len(), |&n| n - 1);
        let start = self.starts.get(line).copied().unwrap_or(text.len()).min(next);
        if text.as_bytes().get(next.wrapping_sub(1)) == Some(&b'\r') && next > start {
            next - 1
        } else {
            next
        }
    }

    /// A line's text, without its line ending.
    pub fn line_text<'t>(&self, text: &'t str, line: usize) -> &'t str {
        let Some(start) = self.line_start(line) else { return "" };
        text.get(start..self.line_end(text, line).max(start)).unwrap_or("")
    }

    /// The (line, UTF-16 column) of a byte offset.
    pub fn to_utf16(&self, text: &str, offset: usize) -> (u32, u32) {
        let (line, col) = self.line_col(offset);
        let start = self.starts[line];
        let col16 = text.get(start..start + col).map_or(col, |s| s.encode_utf16().count());
        (line as u32, col16 as u32)
    }

    /// The byte offset of a (line, UTF-16 column); past the end of a line
    /// it's the line's end, past the last line the text's end.
    pub fn from_utf16(&self, text: &str, line: u32, col16: u32) -> usize {
        let Some(start) = self.line_start(line as usize) else { return text.len() };
        let end = self.line_end(text, line as usize);
        let mut units = 0u32;
        for (i, c) in text[start..end].char_indices() {
            if units >= col16 {
                return start + i;
            }
            units += c.len_utf16() as u32;
        }
        end
    }
}

/// Whether `c` can be part of a BASIC name (type suffixes included).
pub fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// A name's type suffix characters.
pub fn is_suffix_char(c: char) -> bool {
    matches!(c, '$' | '%' | '&' | '!' | '#' | '?')
}

/// The name (with its suffix) around byte `offset` of `text`, as a range.
pub fn word_at(text: &str, offset: usize) -> Option<(usize, usize)> {
    let offset = offset.min(text.len());
    let bytes = text.as_bytes();
    let mut start = offset;
    while start > 0 && is_suffix_char(bytes[start - 1] as char) {
        start -= 1;
    }
    while start > 0 && is_name_char(bytes[start - 1] as char) {
        start -= 1;
    }
    let mut end = start;
    while end < bytes.len() && is_name_char(bytes[end] as char) {
        end += 1;
    }
    while end < bytes.len() && is_suffix_char(bytes[end] as char) {
        end += 1;
    }
    if start == end || end < offset || !(bytes[start] as char).is_ascii_alphabetic() && bytes[start] != b'_' {
        return None;
    }
    Some((start, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_columns_round_trip() {
        let text = "a = \"é😀\"\r\nb";
        let ix = LineIndex::new(text);
        assert_eq!(ix.line_text(text, 0), "a = \"é😀\"");
        let after = text.find('😀').unwrap() + 4;
        let (l, c) = ix.to_utf16(text, after);
        assert_eq!((l, c), (0, 8));
        assert_eq!(ix.from_utf16(text, l, c), after);
        assert_eq!(ix.from_utf16(text, 1, 0), text.len() - 1);
        assert_eq!(ix.line_col(text.len()), (1, 1));
    }

    #[test]
    fn words_carry_their_suffix() {
        let t = "PRINT name$ + x";
        assert_eq!(word_at(t, 7), Some((6, 11)));
        assert_eq!(word_at(t, 11), Some((6, 11)));
        assert_eq!(word_at(t, 13), None);
    }
}
