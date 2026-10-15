//! QRICHEDIT's formatted text: the character formats of its text
//! (RapidQ's SelAttributes and SelFont, the control's Font) and its
//! paragraphs' formats (the control's Alignment; RapidR's Paragraph), kept
//! beside the shared text model (`textedit.rs`) so every runtime formats
//! the same way, reads SelAttributes back the same, and saves and loads the
//! same rich text (`rtf.rs`). The UI kernel draws the formats as styled
//! runs and keeps them while the user types.
//!
//! The rules are RapidQ's rich edit's (RichEdit 1.0) as RC.EXE shows them
//! (probes w_r*, w_s*, w_t*, w_u*, w_v*: a program formats, then saves the
//! text as RTF, which holds every character's format):
//!
//! - **Typed or inserted text** (SelText, AddStrings, the user's typing)
//!   at a caret takes the format of the character before it — at the start
//!   of a paragraph (or of the text) the one after it — unless the program
//!   set SelAttributes at that caret: that format then (the *insertion
//!   format*, kept until the caret moves). Text replacing a selection takes
//!   the selection's first character's format. Deleting leaves no format
//!   behind.
//! - **SelAttributes** on a selection formats its characters; on an empty
//!   one it sets the insertion format. Reading it gives the selection's
//!   last character's format (the insertion format at a caret); an
//!   automatic colour reads clWindowText (&H80000008), Size 0 or less
//!   changes nothing, an empty Name gives Arial 10.
//! - **Font** is the text's *default* format: text never formatted follows
//!   it (all of it, while nothing was formatted). Once part of the text is
//!   formatted, the rest keeps the format it shows, and only the end of the
//!   text (what text added at an empty last line takes) still follows Font
//!   — unless Font had been set while nothing was formatted, then that end
//!   keeps its format too.
//! - **Text =** gives the new text the old first character's format (the
//!   insertion format at 0 in an empty control), its end the default, and
//!   its paragraphs the control's Alignment. **Clear** empties the text;
//!   what is typed next takes the old last character's format.
//! - **Alignment** aligns the paragraphs that have no alignment of their
//!   own (it moves the caret to the start: the control is made again);
//!   paragraphs set by Text = afterwards are given it.
//!
//! Positions here are the model's (`TextEdit`'s characters, a line break
//! one character); the text is passed in where a rule needs it.

use super::font::Font;

/// What an automatic colour reads as: clWindowText (&H80000008), as
/// RapidQ's SelAttributes.Color and Font.Color read it.
pub const AUTO_COLOR: i64 = 0x8000_0008u32 as i32 as i64;

/// A character's format.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CharFmt {
    /// The font's face.
    pub name: String,
    /// Its size in half points (RTF's `\fs`; RapidQ's Size is points).
    pub half_points: i64,
    /// &HBBGGRR; `None` automatic (the window text colour).
    pub color: Option<i64>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    /// 1 (DEFAULT_CHARSET) for the control's Font; 0 (ANSI) once set
    /// through SelAttributes (RC.EXE reads it so; RTF's `\fcharset0`).
    pub charset: i64,
}

impl Default for CharFmt {
    fn default() -> Self {
        CharFmt::from_font(&Font::default(), None)
    }
}

impl CharFmt {
    /// A QFONT's settings as a format (`color`: the font's colour, `None`
    /// automatic).
    pub fn from_font(f: &Font, color: Option<i64>) -> CharFmt {
        let half_points = if f.size > 0 { f.size * 2 } else if f.size < 0 { (-f.size * 72 * 2 + 48) / 96 } else { 16 };
        CharFmt {
            name: f.name.clone(),
            half_points: half_points.max(2),
            color,
            bold: f.styles & 1 != 0,
            italic: f.styles & 2 != 0,
            underline: f.styles & 4 != 0,
            strike: f.styles & 8 != 0,
            charset: 1,
        }
    }

    /// Its size in points (Size reads it so: half points rounded down).
    pub fn points(&self) -> i64 {
        self.half_points / 2
    }

    /// As a QFONT (to draw it: Size in points, the colour &HBBGGRR, black
    /// for automatic).
    pub fn font(&self) -> Font {
        let styles = u8::from(self.bold) | u8::from(self.italic) << 1 | u8::from(self.underline) << 2 | u8::from(self.strike) << 3;
        Font { name: self.name.clone(), size: self.points().max(1), color: self.color.unwrap_or(0), styles }
    }

    /// SelAttributes.<prop> as RapidQ reads it.
    pub fn get(&self, prop: &str) -> Option<crate::Value> {
        use crate::{v_int, v_str};
        Some(match prop {
            "color" => v_int(self.color.unwrap_or(AUTO_COLOR)),
            "name" => v_str(&self.name),
            "size" => v_int(self.points()),
            "bold" => v_int(i64::from(self.bold)),
            "italic" => v_int(i64::from(self.italic)),
            "underline" => v_int(i64::from(self.underline)),
            "strikeout" => v_int(i64::from(self.strike)),
            "charset" => v_int(self.charset),
            "pitch" => v_int(0),
            _ => return None,
        })
    }

    /// SelAttributes.<prop> = v (RC.EXE: Size 0 or less is ignored; an
    /// empty Name gives Arial 10; a Name, Size or colour makes the format
    /// ANSI's; clWindowText is automatic).
    pub fn set(&mut self, prop: &str, v: &crate::Value) -> bool {
        match prop {
            "color" => {
                let c = v.to_i64();
                self.color = (c != AUTO_COLOR && c != 0x8000_0008).then(|| i64::from(super::color_bgr(c)));
            }
            "name" => {
                let n = v.to_string_val();
                if n.trim().is_empty() {
                    self.name = "Arial".into();
                    self.half_points = 20;
                } else {
                    self.name = n;
                }
                self.charset = 0;
            }
            "size" => {
                let s = v.to_i64();
                if s > 0 {
                    self.half_points = s.min(1638) * 2;
                }
            }
            "bold" => self.bold = v.to_bool(),
            "italic" => self.italic = v.to_bool(),
            "underline" => self.underline = v.to_bool(),
            "strikeout" => self.strike = v.to_bool(),
            "charset" => self.charset = v.to_i64(),
            "pitch" => {}
            "addstyles" | "delstyles" => self.style(v.to_i64(), prop == "addstyles"),
            _ => return false,
        }
        true
    }

    /// Style `n` (fsBold 0 … fsStrikeOut 3) on or off.
    pub fn style(&mut self, n: i64, on: bool) {
        match n {
            0 => self.bold = on,
            1 => self.italic = on,
            2 => self.underline = on,
            3 => self.strike = on,
            _ => {}
        }
    }

    /// `SelAttributes = Font`: every setting of the QFONT (`color`: its
    /// colour, `None` automatic).
    pub fn assign(&mut self, f: &Font, color: Option<i64>) {
        let size = self.half_points;
        *self = CharFmt { charset: 0, ..CharFmt::from_font(f, color) };
        if f.size == 0 {
            self.half_points = size;
        }
        if f.name.trim().is_empty() {
            self.name = "Arial".into();
            self.half_points = 20;
        }
    }
}

/// A paragraph's format.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParaFmt {
    /// taLeftJustify 0, taRightJustify 1, taCenter 2, (RTF's justified) 3;
    /// `None`: the control's Alignment.
    pub align: Option<u8>,
    /// The paragraph's left edge, its first line's from that, its right
    /// edge's from the right: twips (1/1440 inch; 15 a pixel).
    pub left: i64,
    pub first: i64,
    pub right: i64,
    /// A bullet before its first line.
    pub bullet: bool,
}

/// Twips in a (logical, 1/96 inch) pixel.
pub const TWIPS_PER_PIXEL: i64 = 15;

/// A run's format: the default (the control's Font) or one of the palette's.
pub type Slot = Option<u32>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Run {
    len: usize,
    slot: Slot,
}

/// One paragraph as the views draw it: its runs (characters, format) and
/// its format (the alignment resolved).
#[derive(Clone, Debug, PartialEq)]
pub struct StyledPara {
    pub runs: Vec<(usize, CharFmt)>,
    pub para: ParaFmt,
    pub align: u8,
    /// Its break's format (the end of the text's for the last): what its
    /// line shows where it holds no text.
    pub mark: CharFmt,
}

/// A rich edit's formats (see the module's rules).
#[derive(Clone, Debug)]
pub struct Rich {
    /// The text's characters' formats, then the end of the text's (one
    /// character more: what an empty last line shows and types in).
    runs: Vec<Run>,
    palette: Vec<CharFmt>,
    /// The control's Font: what default slots show.
    default: CharFmt,
    /// One per paragraph (the text's line breaks + 1).
    paras: Vec<ParaFmt>,
    /// The control's Alignment.
    align: u8,
    /// The insertion format: SelAttributes set at caret `.0` (or what Clear
    /// left at 0).
    pending: Option<(usize, Slot)>,
    /// Font was set while nothing was formatted: the end of the text is
    /// formatted with the rest once part of it is.
    fonted: bool,
    /// Goes up with every change of a format (the views draw again).
    pub revision: u64,
}

impl Default for Rich {
    fn default() -> Self {
        Rich { runs: vec![Run { len: 1, slot: None }], palette: Vec::new(), default: CharFmt::default(), paras: vec![ParaFmt::default()], align: 0, pending: None, fonted: false, revision: 0 }
    }
}

fn breaks(s: &[char]) -> usize {
    s.iter().filter(|c| **c == '\n').count()
}

impl Rich {
    // ------------------------------------------------------------ runs --

    fn total(&self) -> usize {
        self.runs.iter().map(|r| r.len).sum()
    }

    /// Slot of character `i` (the text's length: the end).
    pub fn slot_at(&self, i: usize) -> Slot {
        let mut at = 0;
        for r in &self.runs {
            if i < at + r.len {
                return r.slot;
            }
            at += r.len;
        }
        self.runs.last().map_or(None, |r| r.slot)
    }

    fn fmt(&self, s: Slot) -> &CharFmt {
        match s {
            Some(i) => self.palette.get(i as usize).unwrap_or(&self.default),
            None => &self.default,
        }
    }

    /// Character `i`'s format.
    pub fn fmt_at(&self, i: usize) -> CharFmt {
        self.fmt(self.slot_at(i)).clone()
    }

    fn intern(&mut self, f: CharFmt) -> Slot {
        if let Some(i) = self.palette.iter().position(|p| *p == f) {
            return Some(i as u32);
        }
        // (a long-lived control's palette made small again now and then)
        if self.palette.len() >= 4096 {
            self.compact();
            if let Some(i) = self.palette.iter().position(|p| *p == f) {
                return Some(i as u32);
            }
        }
        self.palette.push(f);
        Some(self.palette.len() as u32 - 1)
    }

    /// The palette with only the formats in use.
    fn compact(&mut self) {
        let mut used: Vec<CharFmt> = Vec::new();
        let mut remap = |s: Slot, pal: &[CharFmt]| -> Slot {
            let f = pal.get(s? as usize)?.clone();
            let i = used.iter().position(|u| *u == f).unwrap_or_else(|| {
                used.push(f);
                used.len() - 1
            });
            Some(i as u32)
        };
        let pal = std::mem::take(&mut self.palette);
        for r in &mut self.runs {
            r.slot = remap(r.slot, &pal);
        }
        if let Some((_, s)) = &mut self.pending {
            *s = remap(*s, &pal);
        }
        self.palette = used;
    }

    /// A run boundary at `i`: the index of the run starting there.
    fn split(&mut self, i: usize) -> usize {
        let mut at = 0;
        for k in 0..self.runs.len() {
            let r = self.runs[k];
            if i == at {
                return k;
            }
            if i < at + r.len {
                self.runs[k].len = i - at;
                self.runs.insert(k + 1, Run { len: at + r.len - i, slot: r.slot });
                return k + 1;
            }
            at += r.len;
        }
        self.runs.len()
    }

    /// Adjacent runs of one format joined, empty ones dropped.
    fn tidy(&mut self) {
        let mut out: Vec<Run> = Vec::with_capacity(self.runs.len());
        for r in self.runs.drain(..) {
            if r.len == 0 {
                continue;
            }
            match out.last_mut() {
                Some(l) if l.slot == r.slot => l.len += r.len,
                _ => out.push(r),
            }
        }
        if out.is_empty() {
            out.push(Run { len: 1, slot: None });
        }
        self.runs = out;
    }

    /// `remove` characters at `at` replaced by `insert` of `slot`.
    fn splice_runs(&mut self, at: usize, remove: usize, insert: usize, slot: Slot) {
        let a = self.split(at);
        let b = self.split(at + remove);
        self.runs.splice(a..b, std::iter::once(Run { len: insert, slot }));
        self.tidy();
    }

    /// Each run of characters `a .. b` given `f(its slot)`.
    fn map_slots(&mut self, a: usize, b: usize, mut f: impl FnMut(&mut Self, Slot) -> Slot) {
        if a >= b {
            return;
        }
        let i = self.split(a);
        let j = self.split(b);
        for k in i..j {
            let s = self.runs[k].slot;
            self.runs[k].slot = f(self, s);
        }
        self.tidy();
    }

    /// Whether nothing is formatted (every character the default's).
    fn unformatted(&self) -> bool {
        self.runs.iter().all(|r| r.slot.is_none())
    }

    /// The first formatting of part of the text: every character still
    /// showing the default keeps what it shows (the end of the text too
    /// when Font was set while nothing was formatted).
    fn make_explicit(&mut self, text_len: usize) {
        if !self.runs.iter().any(|r| r.slot.is_none()) {
            return;
        }
        let d = self.default.clone();
        let slot = self.intern(d);
        let end = if self.fonted { text_len + 1 } else { text_len };
        self.map_slots(0, end, |_, s| s.or(slot));
        self.fonted = false;
    }

    fn changed(&mut self) {
        self.revision += 1;
    }

    // ---------------------------------------------------- the rules --

    /// The format text typed at caret `at` takes (the insertion format).
    fn insertion_slot(&self, text: &[char], at: usize) -> Slot {
        if let Some((p, s)) = self.pending {
            if p == at {
                return s;
            }
        }
        if at == 0 || text.get(at - 1) == Some(&'\n') {
            self.slot_at(at)
        } else {
            self.slot_at(at - 1)
        }
    }

    /// The format text typed at caret `at` takes.
    pub fn insertion_format(&self, text: &[char], at: usize) -> CharFmt {
        self.fmt(self.insertion_slot(text, at)).clone()
    }

    /// Characters `a .. b` of `old` (the text before) replaced by `ins`:
    /// typed text, SelText, a paste, AddStrings, Line(i) = s.
    pub fn replace(&mut self, old: &[char], a: usize, b: usize, ins: &[char]) {
        let (a, b) = (a.min(old.len()), b.min(old.len()).max(a.min(old.len())));
        let slot = if b > a { self.slot_at(a) } else { self.insertion_slot(old, a) };
        self.splice_runs(a, b - a, ins.len(), slot);
        // (a paragraph's format is its break's: joined paragraphs take the
        // second's, a split one's halves both have it)
        let p0 = breaks(&old[..a]);
        let removed = breaks(&old[a..b]);
        self.paras.drain(p0..(p0 + removed).min(self.paras.len()));
        let k = breaks(ins);
        if k > 0 {
            let f = self.paras.get(p0).cloned().unwrap_or_default();
            self.paras.splice(p0..p0, std::iter::repeat_n(f, k));
        }
        self.fix_paras(breaks(old) - removed + k);
        self.pending = None;
        self.changed();
    }

    fn fix_paras(&mut self, line_breaks: usize) {
        self.paras.resize(line_breaks + 1, ParaFmt::default());
    }

    /// Text = `new` (the text before was `old_len` characters long).
    pub fn set_text(&mut self, old_len: usize, new: &[char]) {
        let slot = if old_len == 0 { self.pending.filter(|p| p.0 == 0).map_or_else(|| self.slot_at(0), |p| p.1) } else { self.slot_at(0) };
        self.runs = vec![Run { len: new.len(), slot }, Run { len: 1, slot: None }];
        self.tidy();
        let p = ParaFmt { align: (self.align != 0).then_some(self.align), ..ParaFmt::default() };
        self.paras = vec![p; breaks(new) + 1];
        self.pending = None;
        self.fonted = false;
        self.changed();
    }

    /// A plain text file loaded (LoadFromFile of text that isn't RTF): as
    /// Text = , but the end of the text too takes the first character's
    /// format (RC.EXE).
    pub fn load_plain(&mut self, old_len: usize, new: &[char]) {
        self.set_text(old_len, new);
        let first = self.slot_at(0);
        let n = new.len();
        self.splice_runs(n, 1, 1, first);
        self.changed();
    }

    /// Clear (`old`: the text before).
    pub fn clear(&mut self, old: &[char]) {
        let end = self.slot_at(old.len());
        let pending = (!old.is_empty()).then(|| self.slot_at(old.len() - 1));
        self.runs = vec![Run { len: 1, slot: end }];
        let last = self.paras.last().cloned().unwrap_or_default();
        self.paras = vec![last];
        self.pending = pending.map(|s| (0, s));
        self.fonted = false;
        self.changed();
    }

    /// The selection is now `start`, `len` long: the insertion format stays
    /// only at its caret.
    pub fn select(&mut self, start: usize, len: usize) {
        if self.pending.is_some_and(|(p, _)| len != 0 || p != start) {
            self.pending = None;
        }
    }

    /// SelAttributes changed by `f` on characters `a .. b` of `text` — at a
    /// caret (`a == b`), the insertion format.
    pub fn apply(&mut self, text: &[char], a: usize, b: usize, f: &dyn Fn(&mut CharFmt)) {
        if a >= b {
            // (an insertion format is ANSI's: RC.EXE's RTF gives it
            // `\fcharset0`)
            let mut fmt = CharFmt { charset: 0, ..self.insertion_format(text, a) };
            f(&mut fmt);
            let slot = self.intern(fmt);
            self.pending = Some((a, slot));
            self.changed();
            return;
        }
        self.make_explicit(text.len());
        let b = b.min(text.len());
        self.map_slots(a, b, |me, s| {
            let mut fmt = me.fmt(s).clone();
            f(&mut fmt);
            me.intern(fmt)
        });
        self.changed();
    }

    /// What SelAttributes reads for characters `a .. b` of `text`: the last
    /// one's format (the insertion format at a caret).
    pub fn read(&self, text: &[char], a: usize, b: usize) -> CharFmt {
        if a >= b { self.insertion_format(text, a) } else { self.fmt_at(b.min(text.len()) - 1) }
    }

    /// The control's Font is now `f` (see the module's rules).
    pub fn set_font(&mut self, f: CharFmt) {
        if f == self.default {
            return;
        }
        if self.unformatted() {
            self.fonted = true;
        }
        self.default = f;
        self.pending = None;
        self.changed();
    }

    /// The control's Font.
    pub fn font(&self) -> &CharFmt {
        &self.default
    }

    /// The control's Alignment (paragraphs without their own follow it).
    pub fn set_alignment(&mut self, align: u8) {
        if align != self.align {
            self.align = align;
            self.changed();
        }
    }

    pub fn alignment(&self) -> u8 {
        self.align
    }

    // ------------------------------------------------------ paragraphs --

    /// The paragraphs characters `a .. b` of `text` touch (a caret's: its
    /// own).
    fn para_range(text: &[char], a: usize, b: usize) -> std::ops::Range<usize> {
        let a = a.min(text.len());
        let b = b.min(text.len()).max(a);
        let first = breaks(&text[..a]);
        // (a selection ending at a paragraph's start doesn't take it in)
        let end = if b > a && text[b - 1] == '\n' { b - 1 } else { b };
        first..breaks(&text[..end]) + 1
    }

    /// Paragraph `p`'s format.
    pub fn para(&self, p: usize) -> ParaFmt {
        self.paras.get(p).cloned().unwrap_or_default()
    }

    /// The format of the paragraph character `at` of `text` is in.
    pub fn para_at(&self, text: &[char], at: usize) -> ParaFmt {
        self.para(breaks(&text[..at.min(text.len())]))
    }

    /// (RapidR's Paragraph) The paragraphs of characters `a .. b` of
    /// `text` changed by `f`.
    pub fn apply_para(&mut self, text: &[char], a: usize, b: usize, f: &dyn Fn(&mut ParaFmt)) {
        for p in Self::para_range(text, a, b) {
            if let Some(pf) = self.paras.get_mut(p) {
                f(pf);
            }
        }
        self.changed();
    }

    /// Paragraph `p`'s alignment as shown.
    pub fn shown_align(&self, p: usize) -> u8 {
        self.para(p).align.unwrap_or(self.align)
    }

    // ----------------------------------------------------------- views --

    /// Every paragraph of `text` with its runs (characters, format), as the
    /// views draw them.
    pub fn styled(&self, text: &[char]) -> Vec<StyledPara> {
        let mut out = Vec::with_capacity(self.paras.len());
        let mut cur: Vec<(usize, CharFmt)> = Vec::new();
        let mut run_i = 0;
        let mut left = self.runs.first().map_or(0, |r| r.len);
        let push = |cur: &mut Vec<(usize, CharFmt)>, n: usize, f: &CharFmt| match cur.last_mut() {
            Some((m, g)) if g == f => *m += n,
            _ => cur.push((n, f.clone())),
        };
        let mut i = 0;
        while i < text.len() {
            while left == 0 && run_i + 1 < self.runs.len() {
                run_i += 1;
                left = self.runs[run_i].len;
            }
            let f = self.fmt(self.runs.get(run_i).and_then(|r| r.slot));
            if text[i] == '\n' {
                let p = out.len();
                out.push(StyledPara { runs: std::mem::take(&mut cur), para: self.para(p), align: self.shown_align(p), mark: f.clone() });
                i += 1;
                left = left.saturating_sub(1);
                continue;
            }
            // (as many characters of this run as before the next break)
            let mut n = 0;
            while i + n < text.len() && n < left && text[i + n] != '\n' {
                n += 1;
            }
            let n = n.max(1).min(text.len() - i);
            push(&mut cur, n, f);
            i += n;
            left = left.saturating_sub(n);
        }
        let p = out.len();
        out.push(StyledPara { runs: cur, para: self.para(p), align: self.shown_align(p), mark: self.end_format(text.len()) });
        out
    }

    /// The format of the end of the text (an empty last line's).
    pub fn end_format(&self, text_len: usize) -> CharFmt {
        self.fmt_at(text_len)
    }

    /// Characters `a .. b` of `text` as a document of their own (what Copy
    /// puts on the clipboard as rich text): their formats, their
    /// paragraphs', the last character's format as its end.
    pub fn slice(&self, text: &[char], a: usize, b: usize) -> Rich {
        let (a, b) = (a.min(text.len()), b.min(text.len()).max(a.min(text.len())));
        let mut runs: Vec<(usize, CharFmt)> = Vec::new();
        for i in a..b {
            let f = self.fmt_at(i);
            match runs.last_mut() {
                Some((n, g)) if *g == f => *n += 1,
                _ => runs.push((1, f)),
            }
        }
        let end = if b > a { self.fmt_at(b - 1) } else { self.insertion_format(text, a) };
        runs.push((1, end));
        let p0 = breaks(&text[..a]);
        let paras: Vec<ParaFmt> = (p0..=p0 + breaks(&text[a..b])).map(|p| self.para(p)).collect();
        let mut r = Rich { default: self.default.clone(), align: self.align, ..Rich::default() };
        r.load(runs, paras, &text[a..b]);
        r
    }

    /// `ins` (with its formats `runs`, covering it, and its paragraphs'
    /// `paras`) put in place of characters `a .. b` of `old`: a paste of
    /// rich text.
    pub fn replace_formatted(&mut self, old: &[char], a: usize, b: usize, ins: &[char], runs: &[(usize, CharFmt)], paras: &[ParaFmt]) {
        self.replace(old, a, b, ins);
        let a = a.min(old.len());
        let mut at = a;
        for (n, f) in runs {
            let n = (*n).min((a + ins.len()).saturating_sub(at));
            if n == 0 {
                break;
            }
            let slot = self.intern(f.clone());
            self.map_slots(at, at + n, |_, _| slot);
            at += n;
        }
        // (the pasted paragraphs' formats: those its breaks end)
        let p0 = breaks(&old[..a]);
        for (k, pf) in paras.iter().enumerate().take(breaks(ins)) {
            if let Some(p) = self.paras.get_mut(p0 + k) {
                *p = pf.clone();
            }
        }
        self.changed();
    }

    /// Every character's format (`text_len` of them, then the end's), for
    /// the RTF writer.
    pub(crate) fn formats(&self) -> impl Iterator<Item = (usize, &CharFmt)> {
        self.runs.iter().map(|r| (r.len, self.fmt(r.slot)))
    }

    /// Made from a loaded document: each character's format (`runs`,
    /// covering the text and its end) and each paragraph's.
    pub(crate) fn load(&mut self, runs: Vec<(usize, CharFmt)>, paras: Vec<ParaFmt>, text: &[char]) {
        self.palette.clear();
        self.runs.clear();
        for (n, f) in runs {
            let slot = self.intern(f);
            self.runs.push(Run { len: n, slot });
        }
        let total = self.total();
        if total < text.len() + 1 {
            let slot = self.runs.last().map_or(None, |r| r.slot);
            self.runs.push(Run { len: text.len() + 1 - total, slot });
        } else if total > text.len() + 1 {
            let extra = total - text.len() - 1;
            let a = self.split(text.len() + 1);
            let _ = extra;
            self.runs.truncate(a);
        }
        self.tidy();
        self.paras = paras;
        self.fix_paras(breaks(text));
        self.pending = None;
        self.fonted = false;
        self.changed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    fn bold(f: &mut CharFmt) {
        f.bold = true;
    }

    fn red(f: &mut CharFmt) {
        f.color = Some(0xFF);
    }

    #[test]
    fn insertion_takes_the_format_before_or_at_a_paragraph_start_after() {
        // (RC.EXE w_s1: insert-at-para-start, insert-at-doc-start,
        // insert-at-para-end)
        let mut r = Rich::default();
        let t = chars("AB\nCD");
        r.set_text(0, &t);
        r.apply(&t, 1, 2, &bold);
        r.apply(&t, 3, 4, &red);
        assert!(r.insertion_format(&t, 2).bold, "after B");
        assert_eq!(r.insertion_format(&t, 3).color, Some(0xFF), "a paragraph's start: C's");
        assert!(!r.insertion_format(&t, 0).bold, "the text's start: A's");
        r.replace(&t, 3, 3, &['x']);

        assert_eq!(r.fmt_at(3).color, Some(0xFF));
    }

    #[test]
    fn replacing_takes_the_first_selected_characters_format_and_deleting_leaves_none() {
        let mut r = Rich::default();
        let t = chars("ABCD");
        r.set_text(0, &t);
        r.apply(&t, 1, 2, &bold);
        r.replace(&t, 1, 3, &chars("xy"));
        let t = chars("AxyD");
        assert!(r.fmt_at(1).bold && r.fmt_at(2).bold && !r.fmt_at(3).bold);
        // delete "yD": typing then takes x's (the one before)
        r.replace(&t, 2, 4, &[]);
        let t = chars("Ax");
        r.replace(&t, 2, 2, &['q']);
        assert!(r.fmt_at(2).bold);
    }

    #[test]
    fn selattributes_at_a_caret_is_the_insertion_format_until_the_caret_moves() {
        // (RC.EXE w_u1: pending-survives-same-selstart / sellength0; read
        // back; dropped after a move)
        let mut r = Rich::default();
        let t = chars("abc");
        r.set_text(0, &t);
        r.apply(&t, 1, 1, &bold);
        assert!(r.read(&t, 1, 1).bold);
        r.select(1, 0);
        assert!(r.read(&t, 1, 1).bold, "the same caret again keeps it");
        r.select(2, 0);
        r.select(1, 0);
        assert!(!r.read(&t, 1, 1).bold, "a move drops it");
        r.apply(&t, 3, 3, &red);
        r.replace(&t, 3, 3, &chars("\nadded"));
        let t = chars("abc\nadded");
        assert_eq!(r.fmt_at(5).color, Some(0xFF), "AddStrings at that caret: the colour (RQ_Ychat)");
        assert_eq!(r.end_format(t.len()).color, None, "the end keeps its own");
    }

    #[test]
    fn reading_a_selection_gives_its_last_characters_format() {
        // (RC.EXE w_s3: 1-4, 0-6 read d's / f's)
        let mut r = Rich::default();
        let t = chars("abcdef");
        r.set_text(0, &t);
        r.apply(&t, 0, 3, &bold);
        r.apply(&t, 3, 6, &red);
        assert_eq!(r.read(&t, 1, 4).color, Some(0xFF));
        assert!(!r.read(&t, 0, 6).bold);
        assert!(r.read(&t, 3, 3).bold, "a caret: the character before");
        assert!(r.read(&t, 0, 0).bold, "at the start: the first");
        assert_eq!(r.read(&t, 0, 0).get("color").unwrap().to_i64(), AUTO_COLOR);
    }

    #[test]
    fn font_reformats_what_was_never_formatted() {
        let arial = CharFmt { name: "Arial".into(), half_points: 20, ..CharFmt::default() };
        let courier = CharFmt { name: "Courier New".into(), half_points: 24, ..CharFmt::default() };
        // (w_v1 Q1-Q5: no formatting: everything follows Font)
        let mut r = Rich::default();
        let t = chars("abc");
        r.set_text(0, &t);
        r.set_font(arial.clone());
        r.set_font(courier.clone());
        assert_eq!(r.fmt_at(1).name, "Courier New");
        // (w_t1 P1: formatted part: the rest keeps MS Sans Serif, the end
        // follows)
        let mut r = Rich::default();
        let t = chars("Hello world");
        r.set_text(0, &t);
        r.apply(&t, 6, 11, &bold);
        r.set_font(arial.clone());
        assert_eq!(r.fmt_at(0).name, "MS Sans Serif");
        assert_eq!(r.end_format(t.len()).name, "Arial");
        // (P2: Font set before the first formatting: the end keeps it)
        let mut r = Rich::default();
        r.set_text(0, &t);
        r.set_font(courier.clone());
        r.apply(&t, 6, 11, &bold);
        r.set_font(arial.clone());
        assert_eq!(r.end_format(t.len()).name, "Courier New");
        assert_eq!(r.fmt_at(0).name, "Courier New");
        // (w_s2: Text = then takes the first character's; the end the Font)
        let t2 = chars("new");
        r.set_text(t.len(), &t2);
        assert_eq!((r.fmt_at(0).name.as_str(), r.end_format(3).name.as_str()), ("Courier New", "Arial"));
        // (Clear: typed text takes the old last character's)
        r.apply(&t2, 2, 3, &bold);
        r.clear(&t2);
        r.replace(&[], 0, 0, &['z']);
        assert!(r.fmt_at(0).bold);
    }

    #[test]
    fn paragraphs_split_and_join_keep_the_breaks_format() {
        let mut r = Rich::default();
        let t = chars("ab\ncd");
        r.set_text(0, &t);
        r.apply_para(&t, 4, 4, &|p| p.align = Some(2));
        assert_eq!((r.shown_align(0), r.shown_align(1)), (0, 2));
        // joined: the second's (its break survives)
        r.replace(&t, 1, 4, &[]);
        let t = chars("ad");
        assert_eq!(r.shown_align(0), 2);
        r.replace(&t, 1, 1, &['\n']);
        assert_eq!((r.shown_align(0), r.shown_align(1)), (2, 2));
        r.set_alignment(1);
        let t = chars("x\ny");
        r.set_text(3, &t);
        assert_eq!(r.para(0).align, Some(1), "Text = after Alignment: given it");
    }

    #[test]
    fn styled_paragraphs_carry_their_runs() {
        let mut r = Rich::default();
        let t = chars("ab\ncd");
        r.set_text(0, &t);
        r.apply(&t, 1, 4, &bold);
        let s = r.styled(&t);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].runs.iter().map(|(n, f)| (*n, f.bold)).collect::<Vec<_>>(), [(1, false), (1, true)]);
        assert_eq!(s[1].runs.iter().map(|(n, f)| (*n, f.bold)).collect::<Vec<_>>(), [(1, true), (1, false)]);
    }
}
