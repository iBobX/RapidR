//! QEDIT / QRICHEDIT (and RapidR's QMEMO): the text and the selection,
//! shared by the desktop and web runtimes, so a program reads and changes
//! them the same before its form shows, and the same way on every runtime
//! (RapidQ manual, Appendix A). The runtimes copy what the user typed and
//! selected in the widget here ([`TextEdit::user_edit`]) before the program
//! reads, and show the model again after the program changed it
//! ([`TextEdit::revision`]).
//!
//! A multi-line control's Text has its lines joined by CR LF, as RapidQ's on
//! Windows, and its positions (SelStart, SelLength) are positions in that
//! text: a line break counts two characters, as RC.EXE's QRICHEDIT counts
//! it (RapidQ's rich edit is RichEdit 1.0's: in "ab\r\ncd" SelStart 4 is
//! the `c`, and SelStart 3, inside the break, reads 2). So a position INSTR
//! finds in Text selects that text (RapidQ's richedit/syntax example). The
//! model keeps '\n' breaks; [`TextEdit::sel_start`] / `sel_len` are its own
//! positions, which the widgets use.

use crate::{v_int, v_str, Value};

/// Most characters kept (beyond it, text is cut).
const MAX_CHARS: usize = 64 << 20;

#[derive(Clone, Debug, Default)]
pub struct TextEdit {
    pub multi: bool,
    /// The text, lines separated by '\n'.
    chars: Vec<char>,
    pub sel_start: usize,
    pub sel_len: usize,
    pub modified: bool,
    pub read_only: bool,
    pub max_length: i64,
    /// QEDIT CharCase: 0 ecNormal, 1 ecUpperCase, 2 ecLowerCase.
    pub char_case: i64,
    /// Goes up when the program changes the text or the selection (the
    /// widget shows the model again).
    pub revision: u64,
}

impl TextEdit {
    pub fn new(multi: bool) -> Self {
        TextEdit { multi, ..Default::default() }
    }

    /// The line break the program reads (a multi-line control's CR LF).
    fn line_break(&self) -> &'static str {
        if self.multi { "\r\n" } else { "\n" }
    }

    /// The text as the widget holds it (lines separated by '\n').
    pub fn raw(&self) -> String {
        self.chars.iter().collect()
    }

    /// Text as the program reads it.
    pub fn text(&self) -> String {
        let s = self.raw();
        if self.line_break() != "\n" { s.replace('\n', self.line_break()) } else { s }
    }

    fn normalize(&self, s: &str) -> Vec<char> {
        let s = if self.multi { s.replace("\r\n", "\n").replace('\r', "\n") } else { s.to_string() };
        let s = match self.char_case {
            1 => s.to_uppercase(),
            2 => s.to_lowercase(),
            _ => s,
        };
        s.chars().take(MAX_CHARS).collect()
    }

    fn changed(&mut self) {
        self.revision += 1;
    }

    /// The program's position (in [`Self::text`]) of position `i` of the
    /// model: each line break before it counts two in a multi-line control.
    fn outer(&self, i: usize) -> usize {
        let i = i.min(self.chars.len());
        if self.line_break() == "\n" {
            return i;
        }
        i + self.chars[..i].iter().filter(|c| **c == '\n').count()
    }

    /// The model's position for the program's position `x`: one inside a
    /// CR LF is the break's start, past the end is the end.
    fn inner(&self, x: usize) -> usize {
        if self.line_break() == "\n" {
            return x.min(self.chars.len());
        }
        let mut at = 0;
        for (i, c) in self.chars.iter().enumerate() {
            let w = if *c == '\n' { 2 } else { 1 };
            if at + w > x {
                return i;
            }
            at += w;
        }
        self.chars.len()
    }

    fn clamp_selection(&mut self) {
        self.sel_start = self.sel_start.min(self.chars.len());
        self.sel_len = self.sel_len.min(self.chars.len() - self.sel_start);
    }

    /// The program sets the text: the caret at its start, not Modified
    /// (Windows clears the flag on WM_SETTEXT).
    pub fn set_text(&mut self, s: &str) {
        self.chars = self.normalize(s);
        self.sel_start = 0;
        self.sel_len = 0;
        self.modified = false;
        self.changed();
    }

    /// What the user typed and selected in the widget (`text` as the widget
    /// holds it, positions in characters).
    pub fn user_edit(&mut self, text: &str, sel_start: usize, sel_len: usize) {
        let chars: Vec<char> = text.replace("\r\n", "\n").chars().take(MAX_CHARS).collect();
        if chars != self.chars {
            self.chars = chars;
            self.modified = true;
        }
        self.sel_start = sel_start;
        self.sel_len = sel_len;
        self.clamp_selection();
    }

    pub fn selected(&self) -> String {
        self.chars[self.sel_start..self.sel_start + self.sel_len].iter().collect()
    }

    /// Replaces the selection (SelText = s, a paste): the caret after it.
    pub fn replace_selection(&mut self, s: &str) {
        self.clamp_selection();
        let new = self.normalize(s);
        let at = self.sel_start;
        self.chars.splice(at..at + self.sel_len, new.iter().copied());
        self.sel_start = at + new.len();
        self.sel_len = 0;
        self.modified = true;
        self.changed();
    }

    fn lines(&self) -> Vec<String> {
        if self.chars.is_empty() {
            return Vec::new();
        }
        let s = self.raw();
        let s = s.strip_suffix('\n').unwrap_or(&s);
        s.split('\n').map(str::to_string).collect()
    }

    fn set_lines(&mut self, lines: &[String]) {
        let mut s = lines.join("\n");
        if !s.is_empty() {
            s.push('\n');
        }
        self.chars = s.chars().collect();
        self.clamp_selection();
        self.changed();
    }

    /// The caret's line and column (WhereY, WhereX), from 0.
    fn caret(&self) -> (usize, usize) {
        let before = &self.chars[..self.sel_start.min(self.chars.len())];
        let line = before.iter().filter(|c| **c == '\n').count();
        let col = before.iter().rev().take_while(|c| **c != '\n').count();
        (line, col)
    }

    pub fn get(&self, prop: &str) -> Option<Value> {
        let flag = |b: bool| v_int(b as i64);
        Some(match prop {
            "text" => v_str(&self.text()),
            "seltext" => v_str(&self.selected().replace('\n', self.line_break())),
            "selstart" => v_int(self.outer(self.sel_start) as i64),
            "sellength" => v_int((self.outer(self.sel_start + self.sel_len) - self.outer(self.sel_start)) as i64),
            "modified" => flag(self.modified),
            "readonly" => flag(self.read_only),
            "maxlength" => v_int(self.max_length),
            "charcase" => v_int(self.char_case),
            "linecount" if self.multi => v_int(self.lines().len() as i64),
            "wherex" if self.multi => v_int(self.caret().1 as i64),
            "wherey" if self.multi => v_int(self.caret().0 as i64),
            _ => return None,
        })
    }

    pub fn set(&mut self, prop: &str, val: &Value) -> bool {
        match prop {
            "text" => self.set_text(&val.to_string_val()),
            "seltext" => self.replace_selection(&val.to_string_val()),
            // (setting SelStart drops the selection, as Delphi)
            "selstart" => {
                self.sel_start = self.inner(val.to_i64().max(0) as usize);
                self.sel_len = 0;
                self.clamp_selection();
                self.changed();
            }
            "sellength" => {
                let end = self.inner(self.outer(self.sel_start).saturating_add(val.to_i64().max(0) as usize));
                self.sel_len = end.saturating_sub(self.sel_start);
                self.clamp_selection();
                self.changed();
            }
            "modified" => self.modified = val.to_bool(),
            "readonly" => {
                self.read_only = val.to_bool();
                self.changed();
            }
            "maxlength" => self.max_length = val.to_i64().max(0),
            "charcase" => {
                self.char_case = val.to_i64().clamp(0, 2);
                let s = self.raw();
                self.chars = self.normalize(&s);
                self.changed();
            }
            _ => return false,
        }
        true
    }

    /// Line(i), AddStrings, Clear, SelectAll; `None` for other methods.
    pub fn call(&mut self, method: &str, args: &[Value]) -> Option<Value> {
        let arg = |i: usize| args.get(i).map(Value::to_string_val).unwrap_or_default();
        match method {
            // Line(i) reads a line; Line(i) = s (here with s) changes it.
            "line" | "lines" | "line=" if self.multi => {
                let i = usize::try_from(args.first().map_or(-1, Value::to_i64)).ok();
                let mut lines = self.lines();
                if args.len() >= 2 {
                    if let Some(l) = i.and_then(|i| lines.get_mut(i)) {
                        *l = arg(1);
                        self.set_lines(&lines);
                    }
                    return Some(Value::Null);
                }
                return Some(v_str(i.and_then(|i| lines.get(i)).map_or("", |s| s.as_str())));
            }
            "addstrings" | "addstring" | "addlines" | "add" | "additems" if self.multi => {
                let mut lines = self.lines();
                lines.extend(args.iter().map(|a| a.to_string_val().replace("\r\n", "\n").replace('\r', "\n")));
                self.set_lines(&lines);
            }
            "clear" => {
                self.chars.clear();
                self.sel_start = 0;
                self.sel_len = 0;
                self.changed();
            }
            "selectall" => {
                self.sel_start = 0;
                self.sel_len = self.chars.len();
                self.changed();
            }
            "clearselection" => {
                if !self.read_only {
                    self.replace_selection("");
                }
            }
            _ => return None,
        }
        Some(Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_selection_and_crlf() {
        let mut r = TextEdit::new(true);
        r.call("addstrings", &[v_str("one"), v_str("two")]);
        assert_eq!(r.get("text").unwrap().to_string_val(), "one\r\ntwo\r\n");
        assert_eq!(r.get("linecount").unwrap().to_i64(), 2);
        assert_eq!(r.call("line", &[v_int(1)]).unwrap().to_string_val(), "two");
        r.call("line", &[v_int(0), v_str("ONE")]);
        // ("ONE\r\n" is five characters: "two" starts at 5)
        r.set("selstart", &v_int(5));
        r.set("sellength", &v_int(3));
        assert_eq!(r.get("seltext").unwrap().to_string_val(), "two");
        assert_eq!((r.get("selstart").unwrap().to_i64(), r.get("sellength").unwrap().to_i64()), (5, 3));
        assert_eq!((r.get("wherey").unwrap().to_i64(), r.get("wherex").unwrap().to_i64()), (1, 0));
        r.set("seltext", &v_str("2"));
        assert_eq!(r.get("text").unwrap().to_string_val(), "ONE\r\n2\r\n");
        assert_eq!(r.get("modified").unwrap().to_i64(), 1);
        r.set("text", &v_str("a\r\nb"));
        assert_eq!((r.get("linecount").unwrap().to_i64(), r.get("modified").unwrap().to_i64()), (2, 0));
        r.user_edit("ab\nc", 1, 1);
        assert_eq!(r.get("seltext").unwrap().to_string_val(), "b");
        assert_eq!(r.get("modified").unwrap().to_i64(), 1);

        let mut e = TextEdit::new(false);
        e.set("charcase", &v_int(1));
        // (a single-line edit has no CR LF: positions are its characters)
        e.set("text", &v_str("hello"));
        e.set("selstart", &v_int(1));
        e.set("sellength", &v_int(3));
        assert_eq!(e.get("seltext").unwrap().to_string_val(), "ELL");
        e.call("selectall", &[]);
        assert_eq!(e.get("sellength").unwrap().to_i64(), 5);
    }

    /// RC.EXE's QRICHEDIT (tests/results probe w_selcrlf, 2026-10-15): with
    /// "ab", "cd", "ef" added, Text is "ab\r\ncd\r\nef\r\n" (12); SelStart 4,
    /// SelLength 2 select "cd"; SelStart 3 (inside the break) reads 2, and
    /// SelLength 3 from there selects CR LF and `c`; SelStart 1000 reads 12.
    #[test]
    fn positions_count_cr_lf_as_rapidq() {
        let mut r = TextEdit::new(true);
        r.call("addstrings", &[v_str("ab"), v_str("cd"), v_str("ef")]);
        assert_eq!(r.get("text").unwrap().to_string_val().len(), 12);
        r.set("selstart", &v_int(4));
        r.set("sellength", &v_int(2));
        assert_eq!(r.get("seltext").unwrap().to_string_val(), "cd");
        assert_eq!((r.get("selstart").unwrap().to_i64(), r.get("sellength").unwrap().to_i64()), (4, 2));
        r.set("selstart", &v_int(3));
        r.set("sellength", &v_int(3));
        assert_eq!(r.get("seltext").unwrap().to_string_val(), "\r\nc");
        assert_eq!((r.get("selstart").unwrap().to_i64(), r.get("sellength").unwrap().to_i64()), (2, 3));
        r.set("selstart", &v_int(1000));
        assert_eq!(r.get("selstart").unwrap().to_i64(), 12);
        // (the widget's own positions stay the model's: '\n' one character)
        r.set("selstart", &v_int(4));
        assert_eq!(r.sel_start, 3);
        // (INSTR's position in Text selects what it found)
        let text = r.get("text").unwrap().to_string_val();
        let at = text.find("ef").unwrap();
        r.set("selstart", &v_int(at as i64));
        r.set("sellength", &v_int(2));
        assert_eq!(r.get("seltext").unwrap().to_string_val(), "ef");
    }
}
