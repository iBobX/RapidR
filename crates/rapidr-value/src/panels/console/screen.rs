//! A console's screen: the lines a program printed, with their colours, as
//! a terminal keeps them. RapidQ's CLS, COLOR and LOCATE compile to ANSI /
//! VT escape sequences on every runtime (`crate::console`); this is the
//! same screen model the web IDE's output panel had (`web-ide/
//! ansi_screen.js`), moved into Rust so the UI kernel draws a program's
//! output the same on the desktop and the web:
//!
//! - `ESC[r;cH` / `ESC[r;cf` (LOCATE) moves the cursor; moving never makes
//!   lines, writing does (a cleared screen keeps no empty lines down to
//!   where the cursor went);
//! - `ESC[2J` / `ESC[3J` (CLS) empties the screen, the cursor and colours
//!   kept (CLS's own `ESC[H` homes it);
//! - `ESC[K` erases from the cursor to the line's end;
//! - `ESC[…m` sets the colours: 30–37 / 90–97 the text's, 40–47 / 100–107
//!   the background's, 39 / 49 / 0 back to the theme's;
//! - `\r` goes to the line's start, `\n` down a line, a tab to the next
//!   column of 8, a backspace one back; an escape split across writes waits
//!   for its end;
//! - past [`Screen::max`] lines the oldest go ([`Screen::first`] counts
//!   them, so a line keeps its number while the screen scrolls).

use std::collections::VecDeque;

/// The 16 colours a program can ask for, in ANSI's order (black, red,
/// green, yellow, blue, magenta, cyan, white, then the bright ones): the
/// VGA text-mode palette RapidQ's consoles show, as 0xRRGGBB.
pub const PALETTE: [u32; 16] = [
    0x000000, 0xAA0000, 0x00AA00, 0xAA5500, 0x0000AA, 0xAA00AA, 0x00AAAA, 0xAAAAAA, //
    0x555555, 0xFF5555, 0x55FF55, 0xFFFF55, 0x5555FF, 0xFF55FF, 0x55FFFF, 0xFFFFFF,
];

/// A character's colours: palette indexes (`None`: the theme's text or
/// background).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Attr {
    pub fg: Option<u8>,
    pub bg: Option<u8>,
}

/// One line: its text and where its colours change (character index,
/// colours from there; the first at 0 when the line has text).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Line {
    text: String,
    /// Its length in characters.
    len: usize,
    runs: Vec<(usize, Attr)>,
}

impl Line {
    /// A line of plain text (no colours).
    pub fn plain(text: &str) -> Line {
        let mut l = Line::default();
        for c in text.chars() {
            l.push(c, Attr::default());
        }
        l
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// Its length in characters.
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Its runs of one colour: (first character, past the last, colours).
    pub fn spans(&self) -> Vec<(usize, usize, Attr)> {
        let mut out = Vec::with_capacity(self.runs.len());
        for (k, &(start, attr)) in self.runs.iter().enumerate() {
            let end = self.runs.get(k + 1).map_or(self.len, |r| r.0);
            if end > start {
                out.push((start, end, attr));
            }
        }
        out
    }

    fn push(&mut self, c: char, attr: Attr) {
        if self.runs.last().is_none_or(|r| r.1 != attr) {
            self.runs.push((self.len, attr));
        }
        self.text.push(c);
        self.len += 1;
    }

    /// Character `c` with `attr` at column `col` (spaces before it when the
    /// line is shorter).
    fn put(&mut self, col: usize, c: char, attr: Attr) {
        while self.len < col {
            self.push(' ', Attr::default());
        }
        if col == self.len {
            self.push(c, attr);
            return;
        }
        // (over what's there: the line's characters again, one changed)
        let mut cells: Vec<(char, Attr)> = self.cells();
        cells[col] = (c, attr);
        self.set_cells(&cells);
    }

    /// The characters from column `col` on erased.
    fn truncate(&mut self, col: usize) {
        if col >= self.len {
            return;
        }
        let cells = self.cells();
        self.set_cells(&cells[..col]);
    }

    fn cells(&self) -> Vec<(char, Attr)> {
        let mut out = Vec::with_capacity(self.len);
        for (start, end, attr) in self.spans() {
            out.extend(self.text.chars().skip(start).take(end - start).map(|c| (c, attr)));
        }
        out
    }

    fn set_cells(&mut self, cells: &[(char, Attr)]) {
        *self = Line::default();
        for &(c, a) in cells {
            self.push(c, a);
        }
    }
}

/// A screen of lines, its cursor and colours.
#[derive(Clone, Debug, PartialEq)]
pub struct Screen {
    lines: VecDeque<Line>,
    /// The cursor: a line (counted in `lines`) and a column.
    row: usize,
    col: usize,
    attr: Attr,
    /// An escape sequence split across writes.
    pending: String,
    /// The most lines kept (MaxLines).
    pub max: usize,
    /// How many lines went (past `max`, or cleared): line `i` of the
    /// screen is line `first + i` of everything printed.
    pub first: u64,
    /// Goes up at every change (what's derived from the text — the search's
    /// matches — is made again).
    pub rev: u64,
}

impl Default for Screen {
    fn default() -> Self {
        Screen::new(5000)
    }
}

impl Screen {
    pub fn new(max: usize) -> Screen {
        Screen { lines: VecDeque::new(), row: 0, col: 0, attr: Attr::default(), pending: String::new(), max: max.max(1), first: 0, rev: 0 }
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn line(&self, i: usize) -> Option<&Line> {
        self.lines.get(i)
    }

    /// Every line's text, one to a line.
    pub fn text(&self) -> String {
        let mut s = String::new();
        for (i, l) in self.lines.iter().enumerate() {
            if i > 0 {
                s.push('\n');
            }
            s.push_str(l.text());
        }
        s
    }

    /// The cursor (line, column), from 0.
    pub fn cursor(&self) -> (usize, usize) {
        (self.row, self.col)
    }

    /// Empties it (the cursor home, the colours the theme's); the lines
    /// that were there count as gone.
    pub fn clear(&mut self) {
        self.first += self.lines.len() as u64;
        self.lines.clear();
        self.row = 0;
        self.col = 0;
        self.attr = Attr::default();
        self.pending.clear();
        self.rev += 1;
    }

    /// MaxLines changed: the oldest lines past it go now.
    pub fn set_max(&mut self, max: usize) {
        self.max = max.max(1);
        self.trim();
        self.rev += 1;
    }

    /// Writes `text`, its escape sequences done.
    pub fn write(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.rev += 1;
        let s: Vec<char> = if self.pending.is_empty() { text.chars().collect() } else { std::mem::take(&mut self.pending).chars().chain(text.chars()).collect() };
        let mut i = 0;
        while i < s.len() {
            let c = s[i];
            match c {
                '\x1b' => {
                    let Some(&next) = s.get(i + 1) else {
                        self.pending = s[i..].iter().collect();
                        break;
                    };
                    if next != '[' {
                        // (not a CSI: the escape alone is dropped)
                        i += 1;
                        continue;
                    }
                    let mut j = i + 2;
                    while j < s.len() && !('\x40'..='\x7e').contains(&s[j]) {
                        j += 1;
                    }
                    if j >= s.len() {
                        self.pending = s[i..].iter().collect();
                        break;
                    }
                    let params: String = s[i + 2..j].iter().collect();
                    self.csi(&params, s[j]);
                    i = j;
                }
                '\n' => {
                    self.row += 1;
                    self.col = 0;
                }
                '\r' => self.col = 0,
                '\t' => {
                    let to = (self.col / 8 + 1) * 8;
                    while self.col < to {
                        self.put(' ');
                    }
                }
                '\x08' => self.col = self.col.saturating_sub(1),
                c if (c as u32) < 0x20 || c == '\x7f' => {}
                c => self.put(c),
            }
            i += 1;
        }
        self.trim();
    }

    /// The cursor's line made (and the ones before it).
    fn ensure_row(&mut self) {
        while self.lines.len() <= self.row {
            self.lines.push_back(Line::default());
        }
    }

    fn put(&mut self, c: char) {
        self.ensure_row();
        let (col, attr) = (self.col, self.attr);
        self.lines[self.row].put(col, c, attr);
        self.col += 1;
    }

    fn csi(&mut self, params: &str, fin: char) {
        let nums: Vec<Option<i64>> = params.split(';').map(|p| p.trim().parse::<i64>().ok()).collect();
        let n = |k: usize| nums.get(k).copied().flatten();
        match fin {
            'H' | 'f' => {
                self.row = (n(0).unwrap_or(1).max(1) - 1) as usize;
                self.col = (n(1).unwrap_or(1).max(1) - 1) as usize;
            }
            'J' if matches!(n(0), Some(2) | Some(3)) => {
                let (row, col, attr) = (self.row, self.col, self.attr);
                self.clear();
                (self.row, self.col, self.attr) = (row, col, attr);
            }
            'K' => {
                self.ensure_row();
                let col = self.col;
                self.lines[self.row].truncate(col);
            }
            'm' => {
                let codes: Vec<Option<i64>> = if params.is_empty() { vec![Some(0)] } else { nums };
                for code in codes {
                    match code.unwrap_or(0) {
                        0 => self.attr = Attr::default(),
                        c @ 30..=37 => self.attr.fg = Some((c - 30) as u8),
                        c @ 90..=97 => self.attr.fg = Some((c - 90 + 8) as u8),
                        39 => self.attr.fg = None,
                        c @ 40..=47 => self.attr.bg = Some((c - 40) as u8),
                        c @ 100..=107 => self.attr.bg = Some((c - 100 + 8) as u8),
                        49 => self.attr.bg = None,
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    /// The oldest lines past `max` go (the cursor stays on its line).
    fn trim(&mut self) {
        if self.lines.len() <= self.max {
            return;
        }
        let drop = self.lines.len() - self.max;
        self.lines.drain(..drop);
        self.first += drop as u64;
        self.row = self.row.saturating_sub(drop);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{console, Value};

    fn screen(writes: &[&str]) -> Screen {
        let mut s = Screen::new(5000);
        for w in writes {
            s.write(w);
        }
        s
    }

    fn colours(s: &Screen, i: usize) -> Vec<(String, Option<u8>, Option<u8>)> {
        let l = s.line(i).unwrap();
        l.spans().into_iter().map(|(a, b, at)| (l.text().chars().skip(a).take(b - a).collect(), at.fg, at.bg)).collect()
    }

    #[test]
    fn plain_text_and_new_lines() {
        let s = screen(&["hello\nworld", "!\n"]);
        assert_eq!(s.text(), "hello\nworld!");
        assert_eq!(s.len(), 2, "a new line makes no line until something is written on it");
        assert_eq!(s.cursor(), (2, 0));
        let s = screen(&["a\n\nb"]);
        assert_eq!(s.text(), "a\n\nb");
        let s = screen(&["abc\rX"]);
        assert_eq!(s.text(), "Xbc");
        let s = screen(&["a\tb", "\x08\x08c"]);
        assert_eq!(s.text(), "a      cb");
    }

    #[test]
    fn colours_as_rapidq_sets_them() {
        // COLOR 4 (red), COLOR 14, 1 (yellow on blue), COLOR (reset)
        let red = console::color(&Value::Integer(4), &Value::Null);
        let yellow_on_blue = console::color(&Value::Integer(14), &Value::Integer(1));
        let reset = console::color(&Value::Null, &Value::Null);
        let s = screen(&[&format!("{red}err{yellow_on_blue}warn{reset}ok")]);
        assert_eq!(colours(&s, 0), vec![("err".into(), Some(1), None), ("warn".into(), Some(11), Some(4)), ("ok".into(), None, None)]);
        // (39 / 49: the theme's again, one at a time; bright backgrounds)
        let s = screen(&["\x1b[31;104mA\x1b[39mB\x1b[49mC"]);
        assert_eq!(colours(&s, 0), vec![("A".into(), Some(1), Some(12)), ("B".into(), None, Some(12)), ("C".into(), None, None)]);
        assert_eq!(PALETTE[1], 0xAA0000);
    }

    #[test]
    fn cls_and_locate() {
        let mut s = screen(&["one\ntwo\n"]);
        let first = s.first;
        s.write(&console::cls());
        assert!(s.is_empty());
        assert_eq!(s.first, first + 2, "the cleared lines count as gone");
        // LOCATE 3, 5: moving makes no lines; writing there does
        s.write("\x1b[3;5H");
        assert!(s.is_empty());
        s.write("X");
        assert_eq!(s.text(), "\n\n    X");
        // a write over what's there
        s.write("\x1b[3;2H\x1b[32mY");
        assert_eq!(s.line(2).unwrap().text(), " Y  X");
        assert_eq!(colours(&s, 2)[1], ("Y".into(), Some(2), None));
        // ESC[J keeps the colours and the cursor (CLS's own ESC[H homes it)
        s.write("\x1b[2Jz");
        assert_eq!(s.text(), "\n\n  z");
        assert_eq!(colours(&s, 2)[1], ("z".into(), Some(2), None));
    }

    #[test]
    fn erase_to_the_line_end() {
        let s = screen(&["abcdef\x1b[1;3H\x1b[K"]);
        assert_eq!(s.text(), "ab");
        let s = screen(&["\x1b[2;1H\x1b[K"]);
        assert_eq!(s.text(), "\n", "K makes the cursor's line");
    }

    #[test]
    fn an_escape_split_across_writes() {
        let s = screen(&["a\x1b", "[3", "1mb\x1b[", "0mc"]);
        assert_eq!(s.text(), "abc");
        assert_eq!(colours(&s, 0), vec![("a".into(), None, None), ("b".into(), Some(1), None), ("c".into(), None, None)]);
        // (an escape that isn't a CSI is dropped alone)
        assert_eq!(screen(&["x\x1bQy"]).text(), "xQy");
    }

    #[test]
    fn max_lines_drops_the_oldest() {
        let mut s = Screen::new(3);
        for i in 0..5 {
            s.write(&format!("line {i}\n"));
        }
        assert_eq!(s.text(), "line 2\nline 3\nline 4");
        assert_eq!(s.first, 2);
        s.write("more");
        assert_eq!(s.text(), "line 3\nline 4\nmore");
        s.set_max(1);
        assert_eq!(s.text(), "more");
        assert_eq!(s.first, 5);
    }

    #[test]
    fn a_hundred_thousand_lines() {
        let mut s = Screen::new(200_000);
        let t = std::time::Instant::now();
        for i in 0..100_000 {
            s.write(&format!("\x1b[3{}mline {i}\x1b[0m\n", i % 8));
        }
        assert_eq!(s.len(), 100_000);
        assert!(t.elapsed().as_secs_f64() < 5.0, "{:?}", t.elapsed());
    }
}
