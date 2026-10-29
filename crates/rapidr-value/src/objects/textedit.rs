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

//!
//! A QRICHEDIT's text is formatted too ([`TextEdit::rich`],
//! `richtext.rs`): every change of the text here goes through its formats,
//! so they follow the text whoever changed it.
//!
//! AddStrings and Line(i) = s change the text as RapidQ's Lines do (RapidQ's
//! own: RC.EXE, probes w_r1 / w_s1): a line is added at the end — after
//! a line break, or before one when the text doesn't end with one — and
//! the caret is left after what was added or changed.

use super::codeedit::Encoding;
use super::richtext::{CharFmt, ParaFmt, Rich, StyledPara, TWIPS_PER_PIXEL};
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
    /// A QRICHEDIT's formats.
    pub rich: Option<Box<Rich>>,
    /// A rich edit's WordWrap as last set (`None`: never; RapidQ's default
    /// is True).
    wraps: Option<bool>,
    /// How the last plain text file loaded was stored, which SaveToFile
    /// writes back: UTF-8 (with or without a BOM) when that file was
    /// UTF-8 text; `None` otherwise — Windows-1252, RapidQ's text files.
    /// Text with a character Windows-1252 lacks is saved as UTF-8 either way
    /// (never a `?` in its place).
    file_encoding: Option<Encoding>,
}

impl TextEdit {
    pub fn new(multi: bool) -> Self {
        TextEdit { multi, ..Default::default() }
    }

    /// A QRICHEDIT's: multi-line, formatted.
    pub fn rich_edit() -> Self {
        TextEdit { multi: true, rich: Some(Box::default()), wraps: Some(true), ..Default::default() }
    }

    /// The text's characters (paragraphs separated by '\n').
    pub fn chars(&self) -> &[char] {
        &self.chars
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
        let new = self.normalize(s);
        if let Some(r) = self.rich.as_mut() {
            r.set_text(self.chars.len(), &new);
        }
        self.chars = new;
        self.sel_start = 0;
        self.sel_len = 0;
        self.modified = false;
        self.changed();
    }

    /// What the user typed and selected in the widget (`text` as the widget
    /// holds it, positions in characters). A rich edit's typed text takes
    /// its formats by RapidQ's rules: the edit is found from the selection
    /// it replaced.
    pub fn user_edit(&mut self, text: &str, sel_start: usize, sel_len: usize) {
        let chars: Vec<char> = text.replace("\r\n", "\n").chars().take(MAX_CHARS).collect();
        if chars != self.chars {
            if let Some(r) = self.rich.as_mut() {
                let (a, b, ins) = edit_between(&self.chars, &chars, (self.sel_start, self.sel_start + self.sel_len));
                r.replace(&self.chars, a, b, &chars[ins]);
            }
            self.chars = chars;
            self.modified = true;
        }
        self.sel_start = sel_start;
        self.sel_len = sel_len;
        self.clamp_selection();
        self.selection_moved();
    }

    /// The selection moved: a rich edit's insertion format stays only at
    /// its caret.
    fn selection_moved(&mut self) {
        if let Some(r) = self.rich.as_mut() {
            r.select(self.sel_start, self.sel_len);
        }
    }

    /// Characters `a .. b` replaced by `ins` (formatted by the rich edit's
    /// rules); the caret after them.
    fn splice(&mut self, a: usize, b: usize, ins: &[char]) {
        let (a, b) = (a.min(self.chars.len()), b.min(self.chars.len()).max(a.min(self.chars.len())));
        if let Some(r) = self.rich.as_mut() {
            r.replace(&self.chars, a, b, ins);
        }
        self.chars.splice(a..b, ins.iter().copied());
        self.sel_start = a + ins.len();
        self.sel_len = 0;
    }

    pub fn selected(&self) -> String {
        self.chars[self.sel_start..self.sel_start + self.sel_len].iter().collect()
    }

    /// Replaces the selection (SelText = s, a paste): the caret after it.
    pub fn replace_selection(&mut self, s: &str) {
        self.clamp_selection();
        let new = self.normalize(s);
        let at = self.sel_start;
        self.splice(at, at + self.sel_len, &new);
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

    /// Where line `i` (from 0) is: its first character and its end (before
    /// its break).
    fn line_range(&self, i: usize) -> Option<(usize, usize)> {
        let mut start = 0;
        for _ in 0..i {
            start += self.chars[start..].iter().position(|c| *c == '\n')? + 1;
        }
        if start >= self.chars.len() {
            return None;
        }
        let end = self.chars[start..].iter().position(|c| *c == '\n').map_or(self.chars.len(), |p| start + p);
        Some((start, end))
    }

    /// AddStrings' line `s` at the end (RapidQ's Lines.Add): after the
    /// text's last break, or a break before it when the text doesn't end
    /// with one; the caret after it.
    fn add_line(&mut self, s: &str) {
        let s: Vec<char> = s.replace("\r\n", "\n").replace('\r', "\n").chars().collect();
        let at = self.chars.len();
        let ins: Vec<char> = if self.chars.is_empty() || self.chars.last() == Some(&'\n') {
            s.into_iter().chain(std::iter::once('\n')).collect()
        } else {
            std::iter::once('\n').chain(s).collect()
        };
        // (RapidQ selects where it adds first: an insertion format at
        // another caret goes)
        self.sel_start = at;
        self.sel_len = 0;
        self.selection_moved();
        self.splice(at, at, &ins);
    }

    /// Line(i) = s (RapidQ's Lines.Put): the line's text replaced, the
    /// caret after it.
    fn set_line(&mut self, i: usize, s: &str) {
        let Some((a, b)) = self.line_range(i) else { return };
        let s: Vec<char> = s.replace("\r\n", "\n").replace('\r', "\n").chars().collect();
        self.sel_start = a;
        self.sel_len = b - a;
        self.selection_moved();
        self.splice(a, b, &s);
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
            _ => return self.rich_get(prop),
        })
    }

    /// A rich edit's own members: SelAttributes.*, Alignment, (RapidR's)
    /// Paragraph.* and TextRTF.
    fn rich_get(&self, prop: &str) -> Option<Value> {
        let r = self.rich.as_ref()?;
        if let Some(p) = prop.strip_prefix("selattributes.") {
            return r.read(&self.chars, self.sel_start, self.sel_start + self.sel_len).get(p);
        }
        if let Some(p) = prop.strip_prefix("paragraph.") {
            let pf = r.para_at(&self.chars, self.sel_start);
            let px = |t: i64| v_int((t as f64 / TWIPS_PER_PIXEL as f64).round() as i64);
            return Some(match p {
                "alignment" => v_int(i64::from(pf.align.unwrap_or(r.alignment()))),
                "leftindent" => px(pf.left),
                "firstindent" => px(pf.first),
                "rightindent" => px(pf.right),
                "numbering" => v_int(i64::from(pf.bullet)),
                _ => return None,
            });
        }
        Some(match prop {
            "alignment" => v_int(i64::from(r.alignment())),
            "textrtf" => v_str(&super::rtf::write(&self.chars, r)),
            _ => return None,
        })
    }

    /// The control made again (its form's Center, its WordWrap, its
    /// Alignment): a rich edit's caret goes to the start (RC.EXE).
    pub fn recreated(&mut self) {
        if self.rich.is_some() && (self.sel_start, self.sel_len) != (0, 0) {
            self.sel_start = 0;
            self.sel_len = 0;
            self.selection_moved();
            self.changed();
        }
    }

    /// A rich edit's members set; whether it was one (WordWrap only
    /// remakes the control: the component keeps it).
    fn rich_set(&mut self, prop: &str, val: &Value) -> bool {
        if prop == "wordwrap" {
            // (only a change remakes it: RapidQ's SetWordWrap)
            if self.rich.is_some() && self.wraps != Some(val.to_bool()) {
                if self.wraps.is_some() {
                    self.recreated();
                }
                self.wraps = Some(val.to_bool());
            }
            return false;
        }
        let Some(r) = self.rich.as_mut() else { return false };
        let (a, b) = (self.sel_start, self.sel_start + self.sel_len);
        if let Some(p) = prop.strip_prefix("selattributes.") {
            let mut probe = CharFmt::default();
            if !probe.set(p, val) {
                return false;
            }
            r.apply(&self.chars, a, b, &|f| {
                f.set(p, val);
            });
            return true;
        }
        if let Some(p) = prop.strip_prefix("paragraph.") {
            let twips = val.to_i64().saturating_mul(TWIPS_PER_PIXEL);
            let v = val.to_i64();
            let f: Box<dyn Fn(&mut ParaFmt)> = match p {
                "alignment" => Box::new(move |pf| pf.align = Some(v.clamp(0, 3) as u8)),
                "leftindent" => Box::new(move |pf| pf.left = twips.max(0)),
                "firstindent" => Box::new(move |pf| pf.first = twips),
                "rightindent" => Box::new(move |pf| pf.right = twips.max(0)),
                "numbering" => Box::new(move |pf| {
                    pf.bullet = v != 0;
                    // (a bullet stands in a quarter inch, as WordPad's)
                    if pf.bullet && pf.left == 0 && pf.first == 0 {
                        pf.left = 360;
                        pf.first = -360;
                    }
                }),
                _ => return false,
            };
            r.apply_para(&self.chars, a, b, &*f);
            return true;
        }
        match prop {
            // (the control is made again: the caret at the start, RC.EXE)
            "alignment" => {
                let a = val.to_i64().clamp(0, 3) as u8;
                if a != r.alignment() {
                    r.set_alignment(a);
                    self.recreated();
                    self.changed();
                }
            }
            "textrtf" => {
                let s = val.to_string_val();
                let bytes: Vec<u8> = s.chars().map(|c| cp_byte(c).unwrap_or(b'?')).collect();
                self.load_bytes(&bytes, false);
            }
            _ => return false,
        }
        true
    }

    /// `SelAttributes = Font` / `SelFont = Font` (a QFONT's every setting;
    /// `color` its colour, `None` automatic).
    pub fn set_sel_font(&mut self, font: &super::font::Font, color: Option<i64>) {
        let (a, b) = (self.sel_start, self.sel_start + self.sel_len);
        if let Some(r) = self.rich.as_mut() {
            r.apply(&self.chars, a, b, &|f| f.assign(font, color));
        }
    }

    /// The control's Font is `f` (a rich edit's default format).
    pub fn set_default_format(&mut self, f: CharFmt) {
        if let Some(r) = self.rich.as_mut() {
            r.set_font(f);
        }
    }

    /// A rich edit's paragraphs as the views draw them, and the formats'
    /// revision; `None` for a plain edit.
    pub fn styled(&self) -> Option<(u64, Vec<StyledPara>)> {
        let r = self.rich.as_ref()?;
        Some((r.revision, r.styled(&self.chars)))
    }

    /// A rich edit's formats' revision (0 for a plain edit).
    pub fn format_revision(&self) -> u64 {
        self.rich.as_ref().map_or(0, |r| r.revision)
    }

    /// The format text typed at the caret takes (a rich edit's).
    pub fn caret_format(&self) -> Option<CharFmt> {
        let r = self.rich.as_ref()?;
        Some(r.read(&self.chars, self.sel_start, self.sel_start + self.sel_len))
    }

    /// The rich edit's formats as they are (an undo step keeps them).
    pub fn rich_snapshot(&self) -> Option<Rich> {
        self.rich.as_deref().cloned()
    }

    /// Formats `snap` put back (an undo: the text is the one they were
    /// taken with).
    pub fn rich_restore(&mut self, snap: Rich) {
        if let Some(r) = self.rich.as_mut() {
            let rev = r.revision;
            **r = snap;
            r.revision = rev + 1;
        }
    }

    /// A rich edit's selection as RTF (what a copy keeps beside the plain
    /// text: [`rich_clip_set`]); `None` for a plain edit or no selection.
    pub fn selection_rtf(&self) -> Option<String> {
        let r = self.rich.as_ref()?;
        let (a, b) = (self.sel_start.min(self.chars.len()), (self.sel_start + self.sel_len).min(self.chars.len()));
        if a >= b {
            return None;
        }
        let part = r.slice(&self.chars, a, b);
        Some(super::rtf::write(&self.chars[a..b], &part))
    }

    /// RTF pasted in place of the selection (a rich edit's paste of what
    /// another rich edit copied): its text with its formats, the caret
    /// after it. Whether it was rich text this could paste.
    pub fn paste_rtf(&mut self, rtf: &str) -> bool {
        if self.read_only {
            return false;
        }
        let Some(r) = self.rich.as_mut() else { return false };
        let bytes: Vec<u8> = rtf.chars().map(|c| cp_byte(c).unwrap_or(b'?')).collect();
        let Some(doc) = super::rtf::read(&bytes, r.font()) else { return false };
        let (a, b) = (self.sel_start.min(self.chars.len()), (self.sel_start + self.sel_len).min(self.chars.len()));
        r.replace_formatted(&self.chars, a, b, &doc.text, &doc.runs, &doc.paras);
        self.chars.splice(a..b, doc.text.iter().copied());
        self.sel_start = a + doc.text.len();
        self.sel_len = 0;
        self.modified = true;
        self.changed();
        true
    }

    /// LoadFromFile / LoadFromStream / TextRTF = …: a rich edit reads RTF
    /// (unless `plain`, its PlainText); anything else is the text.
    pub fn load_bytes(&mut self, bytes: &[u8], plain: bool) {
        if let Some(r) = self.rich.as_mut().filter(|_| !plain) {
            if let Some(doc) = super::rtf::read(bytes, r.font()) {
                let text: Vec<char> = doc.text.into_iter().take(MAX_CHARS).collect();
                r.load(doc.runs, doc.paras, &text);
                self.chars = text;
                self.sel_start = 0;
                self.sel_len = 0;
                self.modified = false;
                self.changed();
                return;
            }
        }
        // (a NUL ends a text control's text: a rich edit's own RTF file
        // ends with one, read as plain text — RC.EXE)
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        let (text, encoding) = decode_plain(&bytes[..end]);
        self.file_encoding = encoding;
        if self.rich.is_some() {
            let new = self.normalize(&text);
            if let Some(r) = self.rich.as_mut() {
                r.load_plain(self.chars.len(), &new);
            }
            self.chars = new;
            self.sel_start = 0;
            self.sel_len = 0;
            self.modified = false;
            self.changed();
            return;
        }
        self.set_text(&text);
    }

    /// SaveToFile / SaveToStream: a rich edit's RTF (unless `plain`) —
    /// ending with a NUL byte, as RapidQ's rich edit writes it (RC.EXE) —
    /// else the text, lines ending CR LF.
    pub fn save_bytes(&self, plain: bool) -> Vec<u8> {
        match self.rich.as_ref().filter(|_| !plain) {
            Some(r) => {
                let mut b = super::rtf::write(&self.chars, r).into_bytes();
                b.push(0);
                b
            }
            None => encode_plain(&self.text(), self.file_encoding),
        }
    }

    pub fn set(&mut self, prop: &str, val: &Value) -> bool {
        match prop {
            "text" => self.set_text(&val.to_string_val()),
            "seltext" => self.replace_selection(&val.to_string_val()),
            // (setting SelStart drops the selection, as RapidQ does)
            "selstart" => {
                self.sel_start = self.inner(val.to_i64().max(0) as usize);
                self.sel_len = 0;
                self.clamp_selection();
                self.selection_moved();
                self.changed();
            }
            "sellength" => {
                let end = self.inner(self.outer(self.sel_start).saturating_add(val.to_i64().max(0) as usize));
                self.sel_len = end.saturating_sub(self.sel_start);
                self.clamp_selection();
                self.selection_moved();
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
            _ => return self.rich_set(prop, val),
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
                if args.len() >= 2 {
                    if let Some(i) = i {
                        self.set_line(i, &arg(1));
                        self.changed();
                    }
                    return Some(Value::Null);
                }
                let lines = self.lines();
                return Some(v_str(i.and_then(|i| lines.get(i)).map_or("", |s| s.as_str())));
            }
            "addstrings" | "addstring" | "addlines" | "add" | "additems" if self.multi => {
                for a in args {
                    self.add_line(&a.to_string_val());
                }
                self.changed();
            }
            "clear" => {
                if let Some(r) = self.rich.as_mut() {
                    r.clear(&self.chars);
                }
                self.chars.clear();
                self.sel_start = 0;
                self.sel_len = 0;
                self.changed();
            }
            "selectall" => {
                self.sel_start = 0;
                self.sel_len = self.chars.len();
                self.selection_moved();
                self.changed();
            }
            // SelAttributes.AddStyles(fsBold, …) / DelStyles(…)
            "selattributes.addstyles" | "selattributes.delstyles" if self.rich.is_some() => {
                let on = method.ends_with("addstyles");
                let styles: Vec<i64> = args.iter().map(Value::to_i64).collect();
                let (a, b) = (self.sel_start, self.sel_start + self.sel_len);
                if let Some(r) = self.rich.as_mut() {
                    r.apply(&self.chars, a, b, &|f| {
                        for s in &styles {
                            f.style(*s, on);
                        }
                    });
                }
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

thread_local! {
    /// What a rich edit last copied: the plain text that went to the
    /// clipboard and the same as RTF.
    static RICH_CLIP: std::cell::RefCell<Option<(String, String)>> = const { std::cell::RefCell::new(None) };
}

/// A rich edit copied `plain` (to the clipboard), which is `rtf` with its
/// formats: a paste of the same text into a rich edit keeps them.
pub fn rich_clip_set(plain: &str, rtf: Option<String>) {
    RICH_CLIP.with(|c| *c.borrow_mut() = rtf.map(|r| (plain.replace("\r\n", "\n"), r)));
}

/// The RTF a rich edit copied, if the clipboard still holds its text
/// (`clip`: the clipboard's text now).
pub fn rich_clip_for(clip: &str) -> Option<String> {
    let clip = clip.replace("\r\n", "\n");
    RICH_CLIP.with(|c| c.borrow().as_ref().filter(|(p, _)| *p == clip).map(|(_, r)| r.clone()))
}

/// Character `c` as the byte a Windows-1252 file gets (a Latin-1 byte read
/// in is kept as it was); `None` past them.
fn cp_byte(c: char) -> Option<u8> {
    super::rtf::cp1252_byte(c).or(if (c as u32) < 256 { Some(c as u32 as u8) } else { None })
}

/// A plain text file's text, and how it was stored (`None`: Windows-1252,
/// as RapidQ's text files are): UTF-8 when the bytes are UTF-8 with
/// anything beyond ASCII in them (or start with a BOM), else Windows-1252.
fn decode_plain(bytes: &[u8]) -> (String, Option<Encoding>) {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return (super::codec::decode_text(bytes), Some(Encoding::Utf8Bom));
    }
    if super::codec::is_utf8_text(bytes) {
        return (super::codec::decode_text(bytes), Some(Encoding::Utf8));
    }
    (bytes.iter().map(|&b| super::rtf::cp1252_char(b)).collect(), None)
}

/// A plain text file's bytes: as stored when it was loaded, Windows-1252
/// otherwise — and as UTF-8 when the text has a character Windows-1252
/// lacks (Chinese, `€`'s cousins, emoji), never with a `?` in its place.
fn encode_plain(text: &str, encoding: Option<Encoding>) -> Vec<u8> {
    match encoding {
        Some(Encoding::Utf8Bom) => [&[0xEF, 0xBB, 0xBF][..], text.as_bytes()].concat(),
        Some(_) => text.as_bytes().to_vec(),
        None => text.chars().map(cp_byte).collect::<Option<Vec<u8>>>().unwrap_or_else(|| text.as_bytes().to_vec()),
    }
}

/// The edit that made `new` of `old` (characters `a .. b` of `old`
/// replaced by `new[range]`), found from the selection `sel` it was made
/// at: typed, pasted or deleted over it, a Backspace or Delete at its
/// caret; anything else (an undo) the part that differs.
fn edit_between(old: &[char], new: &[char], sel: (usize, usize)) -> (usize, usize, std::ops::Range<usize>) {
    let (a, b) = (sel.0.min(old.len()), sel.1.min(old.len()).max(sel.0.min(old.len())));
    if new.len() + (b - a) >= old.len() {
        let k = new.len() + (b - a) - old.len();
        if (k > 0 || b > a) && a + k <= new.len() && new[..a] == old[..a] && new[a + k..] == old[b..] {
            return (a, b, a..a + k);
        }
    }
    if a == b && new.len() < old.len() {
        let k = old.len() - new.len();
        if a >= k && new[..a - k] == old[..a - k] && new[a - k..] == old[a..] {
            return (a - k, a, a - k..a - k);
        }
        if a + k <= old.len() && new[..a.min(new.len())] == old[..a.min(new.len())] && a <= new.len() && new[a..] == old[a + k..] {
            return (a, a + k, a..a);
        }
    }
    let mut p = 0;
    while p < old.len() && p < new.len() && old[p] == new[p] {
        p += 1;
    }
    let mut s = 0;
    while s < old.len() - p && s < new.len() - p && old[old.len() - 1 - s] == new[new.len() - 1 - s] {
        s += 1;
    }
    (p, old.len() - s, p..new.len() - s)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A text box's SaveToFile wrote `?` for every character Windows-1252
    /// lacks. It keeps how the file it loaded was stored (UTF-8, or the
    /// ANSI of RapidQ's files), and what that can't hold goes to UTF-8.
    #[test]
    fn plain_text_files_lose_no_character() {
        // (a UTF-8 file: read as text, saved as it was)
        let utf8 = "太阳系 · café\r\n中文";
        let mut t = TextEdit::new(true);
        t.load_bytes(utf8.as_bytes(), true);
        assert_eq!(t.get("text").unwrap().to_string_val(), utf8);
        assert_eq!(t.save_bytes(true), utf8.as_bytes());
        // (with a BOM: kept)
        t.load_bytes(b"\xEF\xBB\xBFna\xC3\xAFve", true);
        assert_eq!(t.get("text").unwrap().to_string_val(), "naïve");
        assert_eq!(t.save_bytes(true), b"\xEF\xBB\xBFna\xC3\xAFve");
        // (an ANSI file, RapidQ's: read as Windows-1252, saved as it was)
        t.load_bytes(b"caf\xE9 \x80 \x93x\x94", true);
        assert_eq!(t.get("text").unwrap().to_string_val(), "café € “x”");
        assert_eq!(t.save_bytes(true), b"caf\xE9 \x80 \x93x\x94");
        // (Chinese typed into it: the file becomes UTF-8, nothing a `?`)
        t.set("text", &v_str("café 中文"));
        let saved = t.save_bytes(true);
        assert_eq!(saved, "café 中文".as_bytes());
        assert!(!saved.contains(&b'?'));
        // (a text never loaded: ANSI as RapidQ writes it while it fits)
        let mut n = TextEdit::new(true);
        n.set("text", &v_str("Español"));
        assert_eq!(n.save_bytes(true), b"Espa\xF1ol");
        n.set("text", &v_str("日本語 Español 😀"));
        assert_eq!(n.save_bytes(true), "日本語 Español 😀".as_bytes());
    }

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

    /// RC.EXE's AddStrings / Line(i) = (probe w_r1, w_s1): a line goes
    /// after the last break, or a break before it; the caret after it.
    #[test]
    fn lines_are_added_and_put_as_rapidq() {
        let mut r = TextEdit::rich_edit();
        r.set("text", &v_str("Hello world"));
        r.call("addstrings", &[v_str("added")]);
        assert_eq!(r.get("text").unwrap().to_string_val(), "Hello world\r\nadded");
        assert_eq!(r.get("selstart").unwrap().to_i64(), 18);
        r.call("clear", &[]);
        r.call("addstrings", &[v_str("a"), v_str("b")]);
        assert_eq!((r.get("text").unwrap().to_string_val().as_str(), r.get("selstart").unwrap().to_i64()), ("a\r\nb\r\n", 6));
        r.set("text", &v_str("AB\r\nCD\r\nEF"));
        r.call("line", &[v_int(1), v_str("XX")]);
        assert_eq!((r.get("text").unwrap().to_string_val().as_str(), r.get("selstart").unwrap().to_i64()), ("AB\r\nXX\r\nEF", 6));
    }

    fn font(name: &str, size: i64, color: i64, styles: u8) -> super::super::font::Font {
        super::super::font::Font { name: name.into(), size, color, styles }
    }

    /// RC.EXE's own rich text (probe w_r1: each step's SaveToFile), step
    /// by step: SelAttributes = Font and its members, SelText at a caret
    /// after SelAttributes, AddStrings, Font, Alignment, Text, Clear.
    #[test]
    fn formats_and_saves_as_rapidqs_rich_edit() {
        // (the file ends with a NUL byte, as RC.EXE's)
        let rtf = |r: &TextEdit| String::from_utf8(r.save_bytes(false)).unwrap().strip_suffix('\0').expect("a NUL last").to_string();
        let head = "{\\rtf1\\ansi\\ansicpg1252\\deff0\\deflang1033{\\fonttbl{\\f0\\fnil MS Sans Serif;}";
        let mut r = TextEdit::rich_edit();
        r.set("text", &v_str("Hello world"));
        r.set("selstart", &v_int(6));
        r.set("sellength", &v_int(5));
        r.set_sel_font(&font("Courier New", 14, 0xFF, 1), Some(0xFF));
        assert_eq!(rtf(&r), format!("{head}{{\\f1\\fnil\\fcharset0 Courier New;}}}}\r\n{{\\colortbl ;\\red255\\green0\\blue0;}}\r\n\\viewkind4\\uc1\\pard\\f0\\fs16 Hello \\cf1\\b\\f1\\fs28 world\\cf0\\b0\\f0\\fs16 \r\n\\par }}\r\n"));
        r.set("selstart", &v_int(0));
        r.set("sellength", &v_int(5));
        r.set("selattributes.color", &v_int(0xFF0000));
        r.set("selattributes.italic", &v_int(1));
        r.set("selstart", &v_int(1));
        r.set("sellength", &v_int(2));
        r.call("selattributes.addstyles", &[v_int(2), v_int(3)]);
        r.set("selattributes.size", &v_int(20));
        r.set("selattributes.name", &v_str("Times New Roman"));
        r.set("selstart", &v_int(2));
        r.set("seltext", &v_str("XY"));
        r.set("selstart", &v_int(4));
        r.set("selattributes.color", &v_int(0x00FF00));
        r.set("seltext", &v_str("ZZ"));
        assert_eq!(
            rtf(&r),
            "{\\rtf1\\ansi\\ansicpg1252\\deff0\\deflang1033{\\fonttbl{\\f0\\fnil MS Sans Serif;}{\\f1\\fnil\\fcharset0 Times New Roman;}{\\f2\\fnil\\fcharset0 Courier New;}}\r\n{\\colortbl ;\\red0\\green0\\blue255;\\red0\\green255\\blue0;\\red255\\green0\\blue0;}\r\n\\viewkind4\\uc1\\pard\\cf1\\i\\f0\\fs16 H\\ul\\strike\\f1\\fs40 eXY\\cf2 ZZ\\cf1 l\\ulnone\\strike0\\f0\\fs16 lo\\cf0\\i0  \\cf3\\b\\f2\\fs28 world\\cf0\\b0\\f0\\fs16 \r\n\\par }\r\n"
        );
        // (a colour at the caret colours what AddStrings adds there)
        let end = r.get("text").unwrap().to_string_val().len();
        r.set("selstart", &v_int(end as i64));
        r.set("selattributes.color", &v_int(0x808000));
        r.call("addstrings", &[v_str("added")]);
        r.set("selattributes.color", &v_int(0x008080));
        r.call("addstrings", &[v_str("added2")]);
        assert_eq!(r.get("selstart").unwrap().to_i64(), 30);
        // (Font: what still has the default — the end — follows)
        r.set_default_format(CharFmt::from_font(&font("Arial", 10, 0, 0), None));
        let s = rtf(&r);
        assert!(s.ends_with("world\\cf4 \r\n\\par added\\cf5 \r\n\\par added2\\cf0\\b0\\f3\\fs20 \r\n\\par }\r\n"), "{s}");
        assert!(s.contains("{\\f3\\fnil Arial;}}\r\n{\\colortbl ;\\red0\\green0\\blue255;\\red0\\green255\\blue0;\\red255\\green0\\blue0;\\red0\\green128\\blue128;\\red128\\green128\\blue0;}"), "{s}");
        r.set("alignment", &v_int(2));
        assert_eq!(rtf(&r), s, "Alignment leaves the paragraphs");
        assert_eq!(r.get("selstart").unwrap().to_i64(), 0);
        r.set("alignment", &v_int(0));
        r.set("text", &v_str("New text\r\nsecond"));
        assert_eq!(rtf(&r), "{\\rtf1\\ansi\\ansicpg1252\\deff0\\deflang1033{\\fonttbl{\\f0\\fnil MS Sans Serif;}{\\f1\\fnil Arial;}}\r\n{\\colortbl ;\\red0\\green0\\blue255;}\r\n\\viewkind4\\uc1\\pard\\cf1\\i\\f0\\fs16 New text\r\n\\par second\\cf0\\i0\\f1\\fs20 \r\n\\par }\r\n");
        r.set("selstart", &v_int(0));
        r.set("sellength", &v_int(3));
        r.set("selattributes.bold", &v_int(1));
        r.call("clear", &[]);
        r.call("addstrings", &[v_str("a"), v_str("b")]);
        assert_eq!(rtf(&r), "{\\rtf1\\ansi\\ansicpg1252\\deff0\\deflang1033{\\fonttbl{\\f0\\fnil MS Sans Serif;}{\\f1\\fnil Arial;}}\r\n{\\colortbl ;\\red0\\green0\\blue255;}\r\n\\viewkind4\\uc1\\pard\\cf1\\i\\f0\\fs16 a\r\n\\par \\cf0\\i0\\f1\\fs20 b\r\n\\par \r\n\\par }\r\n");
        assert_eq!(r.save_bytes(true), b"a\r\nb\r\n");
        // (reading back: the last character's, the caret's)
        r.set("selstart", &v_int(0));
        assert_eq!(r.get("selattributes.italic").unwrap().to_i64(), 1);
        assert_eq!(r.get("selattributes.color").unwrap().to_i64(), 0xFF0000);
        r.set("selstart", &v_int(3));
        assert_eq!(r.get("selattributes.name").unwrap().to_string_val(), "Arial");
        assert_eq!(r.get("selattributes.color").unwrap().to_i64(), super::super::richtext::AUTO_COLOR);
    }

    /// The user's typing keeps the formats (the kernel's edits come as the
    /// whole text and the caret): typed at a caret, over a selection, a
    /// Backspace, a Delete.
    #[test]
    fn typing_keeps_the_formats() {
        let mut r = TextEdit::rich_edit();
        r.set("text", &v_str("abcd"));
        r.set("selstart", &v_int(1));
        r.set("sellength", &v_int(2));
        r.set("selattributes.bold", &v_int(1));
        // typed after "bc": bold (the character before); the typed text
        // repeats a letter, so only the caret tells where it went
        r.set("selstart", &v_int(3));
        r.user_edit("abccd", 4, 0);
        assert!(r.caret_format().unwrap().bold);
        assert_eq!(r.rich.as_ref().unwrap().fmt_at(3).bold, true);
        r.user_edit("abccd", 1, 1);
        r.user_edit("aXccd", 2, 0);
        assert!(r.rich.as_ref().unwrap().fmt_at(1).bold, "over a selection: its first character's");
        r.user_edit("accd", 1, 0);
        assert!(!r.rich.as_ref().unwrap().fmt_at(0).bold && r.rich.as_ref().unwrap().fmt_at(1).bold);
        r.user_edit("acd", 1, 0);
        assert_eq!(r.get("text").unwrap().to_string_val(), "acd");
        assert!(r.rich.as_ref().unwrap().fmt_at(1).bold && !r.rich.as_ref().unwrap().fmt_at(2).bold);
    }

    /// A rich edit's selection copied as RTF and pasted into another keeps
    /// its formats and paragraphs; the clipboard match is by its text.
    #[test]
    fn rich_text_pastes_with_its_formats() {
        // (a centred paragraph "red" copied with its break, and "tw")
        let mut a = TextEdit::rich_edit();
        a.set("text", &v_str("one\r\nred\r\ntwo"));
        a.set("selstart", &v_int(5));
        a.set("sellength", &v_int(3));
        a.set("selattributes.color", &v_int(0xFF));
        a.set("paragraph.alignment", &v_int(2));
        a.set("selstart", &v_int(5));
        a.set("sellength", &v_int(7));
        let rtf = a.selection_rtf().expect("a selection");
        rich_clip_set("red\ntw", Some(rtf.clone()));
        assert_eq!(rich_clip_for("red\r\ntw").as_deref(), Some(rtf.as_str()));
        assert_eq!(rich_clip_for("other"), None);
        let mut b = TextEdit::rich_edit();
        b.set("text", &v_str("[]"));
        b.set("selstart", &v_int(1));
        assert!(b.paste_rtf(&rtf));
        assert_eq!(b.get("text").unwrap().to_string_val(), "[red\r\ntw]");
        assert_eq!((b.get("selstart").unwrap().to_i64(), b.get("modified").unwrap().to_i64()), (8, 1));
        b.set("selstart", &v_int(1));
        b.set("sellength", &v_int(3));
        assert_eq!(b.get("selattributes.color").unwrap().to_i64(), 0xFF);
        // (the paragraph the pasted break ends has the copied one's
        // format; the last pasted line joins the paragraph it went into)
        assert_eq!(b.get("paragraph.alignment").unwrap().to_i64(), 2, "the pasted paragraph's alignment");
        b.set("selstart", &v_int(7));
        assert_eq!(b.get("paragraph.alignment").unwrap().to_i64(), 0);
        // a plain edit pastes nothing rich
        assert!(!TextEdit::new(true).paste_rtf(&rtf));
    }

    #[test]
    fn rtf_load_and_plain_text() {
        let mut r = TextEdit::rich_edit();
        r.load_bytes(b"{\\rtf1{\\fonttbl{\\f0 Arial;}}\\f0\\fs20 a\\b b\\par\\qc c\\par}", false);
        assert_eq!(r.get("text").unwrap().to_string_val(), "ab\r\nc");
        r.set("selstart", &v_int(1));
        r.set("sellength", &v_int(1));
        assert_eq!(r.get("selattributes.bold").unwrap().to_i64(), 1);
        r.set("selstart", &v_int(4));
        assert_eq!(r.get("paragraph.alignment").unwrap().to_i64(), 2);
        // PlainText: the code is the text
        r.load_bytes(b"{\\rtf1 x}", true);
        assert_eq!(r.get("text").unwrap().to_string_val(), "{\\rtf1 x}");
        // not RTF: plain text
        r.load_bytes(b"line1\r\nline2\r\n", false);
        assert_eq!(r.get("text").unwrap().to_string_val(), "line1\r\nline2\r\n");
        // TextRTF both ways
        r.set("textrtf", &v_str("{\\rtf1 caf\\'e9 \\i x}"));
        assert_eq!(r.get("text").unwrap().to_string_val(), "caf\u{e9} x");
        assert!(r.get("textrtf").unwrap().to_string_val().contains("caf\\'e9 \\i x"));
        // Paragraph (RapidR's)
        r.set("paragraph.numbering", &v_int(1));
        r.set("paragraph.leftindent", &v_int(40));
        assert_eq!((r.get("paragraph.numbering").unwrap().to_i64(), r.get("paragraph.leftindent").unwrap().to_i64()), (1, 40));
    }
}
