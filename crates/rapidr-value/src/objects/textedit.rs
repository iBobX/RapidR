//! QEDIT / QRICHEDIT (and RapidR's QMEMO): the text and the selection,
//! shared by the desktop and web runtimes, so a program reads and changes
//! them the same before its form shows, and the same way on every runtime
//! (RapidQ manual, Appendix A). The runtimes copy what the user typed and
//! selected in the widget here ([`TextEdit::user_edit`]) before the program
//! reads, and show the model again after the program changed it
//! ([`TextEdit::revision`]).
//!
//! A multi-line control's Text has its lines joined by CR LF, as RapidQ's on
//! Windows; positions (SelStart, SelLength) count a line break as one
//! character, as a rich edit does.
//!
//! RapidR's RCODEEDITOR is a multi-line one in code mode ([`TextEdit::code`]):
//! its Text keeps '\n' line breaks, and it has GetSubList, GotoSub and
//! GotoLine (`objects::code`).

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
    /// An RCODEEDITOR's (see the module's doc).
    pub code: bool,
    /// Goes up when the program asks for the caret to be scrolled into
    /// view (GotoLine, GotoSub).
    pub reveal: u64,
}

impl TextEdit {
    pub fn new(multi: bool) -> Self {
        TextEdit { multi, ..Default::default() }
    }

    /// An RCODEEDITOR's: multi-line, its Text with '\n' line breaks.
    pub fn code() -> Self {
        TextEdit { multi: true, code: true, ..Default::default() }
    }

    /// The line break the program reads (CR LF; a code editor's '\n').
    fn line_break(&self) -> &'static str {
        if self.multi && !self.code { "\r\n" } else { "\n" }
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

    /// The caret at line `n`'s start (from 0; past the last, the end),
    /// scrolled into view.
    fn goto_line(&mut self, n: usize) {
        self.sel_start = super::code::line_start(&self.raw(), n);
        self.sel_len = 0;
        self.reveal += 1;
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
        let flag = |b: bool| v_int(if b { -1 } else { 0 });
        Some(match prop {
            "text" => v_str(&self.text()),
            "seltext" => v_str(&self.selected().replace('\n', self.line_break())),
            "selstart" => v_int(self.sel_start as i64),
            "sellength" => v_int(self.sel_len as i64),
            "modified" => flag(self.modified),
            "readonly" => flag(self.read_only),
            "maxlength" => v_int(self.max_length),
            "charcase" => v_int(self.char_case),
            "linecount" if self.multi => v_int(self.lines().len() as i64),
            "wherex" if self.multi => v_int(self.caret().1 as i64),
            "wherey" if self.multi => v_int(self.caret().0 as i64),
            // (a function read without parentheses: `S$ = Ed.GetSubList`)
            "getsublist" if self.code => v_str(&super::code::sub_list(&self.raw()).join("\n")),
            _ => return None,
        })
    }

    pub fn set(&mut self, prop: &str, val: &Value) -> bool {
        match prop {
            "text" => self.set_text(&val.to_string_val()),
            "seltext" => self.replace_selection(&val.to_string_val()),
            // (setting SelStart drops the selection, as Delphi)
            "selstart" => {
                self.sel_start = val.to_i64().max(0) as usize;
                self.sel_len = 0;
                self.clamp_selection();
                self.changed();
            }
            "sellength" => {
                self.sel_len = val.to_i64().max(0) as usize;
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
            // (an RCODEEDITOR's: objects::code)
            "getsublist" if self.code => return Some(v_str(&super::code::sub_list(&self.raw()).join("\n"))),
            "gotosub" if self.code => {
                if let Some(line) = super::code::sub_line(&self.raw(), &arg(0)) {
                    self.goto_line(line);
                }
            }
            "gotoline" if self.code => {
                let line = args.first().map_or(0, Value::to_i64).max(0) as usize;
                self.goto_line(line);
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
        r.set("selstart", &v_int(4));
        r.set("sellength", &v_int(3));
        assert_eq!(r.get("seltext").unwrap().to_string_val(), "two");
        assert_eq!((r.get("wherey").unwrap().to_i64(), r.get("wherex").unwrap().to_i64()), (1, 0));
        r.set("seltext", &v_str("2"));
        assert_eq!(r.get("text").unwrap().to_string_val(), "ONE\r\n2\r\n");
        assert_eq!(r.get("modified").unwrap().to_i64(), -1);
        r.set("text", &v_str("a\r\nb"));
        assert_eq!((r.get("linecount").unwrap().to_i64(), r.get("modified").unwrap().to_i64()), (2, 0));
        r.user_edit("ab\nc", 1, 1);
        assert_eq!(r.get("seltext").unwrap().to_string_val(), "b");
        assert_eq!(r.get("modified").unwrap().to_i64(), -1);

        // a code editor: '\n' line breaks, its SUBs, GotoLine / GotoSub
        let mut c = TextEdit::code();
        c.set("text", &v_str("DIM a\r\nSUB Foo\r\nEND SUB\r\nFUNCTION Bar(x)"));
        assert_eq!(c.get("text").unwrap().to_string_val(), "DIM a\nSUB Foo\nEND SUB\nFUNCTION Bar(x)");
        assert_eq!(c.call("getsublist", &[]).unwrap().to_string_val(), "Foo\nBar");
        assert_eq!(c.get("getsublist"), c.call("getsublist", &[]));
        let rev = c.revision;
        c.call("gotosub", &[v_str("bar")]);
        assert_eq!((c.sel_start, c.sel_len, c.reveal), (22, 0, 1));
        assert!(c.revision > rev && !c.modified);
        c.call("gotoline", &[v_int(1)]);
        assert_eq!((c.get("wherey").unwrap().to_i64(), c.sel_start), (1, 6));
        c.set("sellength", &v_int(9));
        assert_eq!(c.get("seltext").unwrap().to_string_val(), "SUB Foo\nE");
        assert!(r.call("gotoline", &[v_int(1)]).is_none(), "a memo has none");

        let mut e = TextEdit::new(false);
        e.set("charcase", &v_int(1));
        e.set("text", &v_str("hello"));
        e.set("selstart", &v_int(1));
        e.set("sellength", &v_int(3));
        assert_eq!(e.get("seltext").unwrap().to_string_val(), "ELL");
        e.call("selectall", &[]);
        assert_eq!(e.get("sellength").unwrap().to_i64(), 5);
    }
}
