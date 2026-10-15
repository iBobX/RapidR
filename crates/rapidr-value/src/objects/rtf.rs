//! Rich Text Format for QRICHEDIT: what LoadFromFile / SaveToFile (and the
//! streams) read and write while PlainText is False, as RapidQ's rich edit
//! does — RapidR's own reader and writer of the part of RTF a rich edit
//! uses (Microsoft's published RTF specification):
//!
//! - **Writing** gives what RapidQ's rich edit writes (RC.EXE's SaveToFile,
//!   probes w_r*): the header with its font table (`\fnil`, `\fcharset0`
//!   for fonts set through SelAttributes) and colour table in the order the
//!   text first uses them, `\viewkind4\uc1\pard`, then each paragraph's
//!   format where it changes (`\qc` `\qr` `\qj`, `\fi` `\li` `\ri`, a
//!   bullet), each character format change as `\cf` `\b` `\i` `\ul`
//!   `\strike` `\f` `\fs`, a paragraph break as CR LF `\par`, characters
//!   past ASCII as `\'hh` (Windows-1252) or `\uN?`, and the end of the text
//!   as a last `\par`.
//! - **Reading** takes any RTF a word processor writes: groups, control
//!   words, the font and colour tables, `\'hh`, `\uN` (with `\uc`),
//!   paragraph and character formats, `\tab` `\line` `\par` and the named
//!   characters (`\emdash`, `\lquote` …); destinations it doesn't show
//!   (`\*`, pictures, headers, the stylesheet, `\info` …) are skipped, a
//!   field shows its result. A file that isn't RTF is `None` (the caller
//!   loads it as plain text, as RapidQ's does).

use super::richtext::{CharFmt, ParaFmt, Rich};

/// Windows-1252's characters 128 … 159 (the rest of 128 … 255 is Latin-1;
/// the holes are U+0081 …).
const CP1252: [char; 32] = [
    '\u{20AC}', '\u{0081}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}', '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{008D}', '\u{017D}', '\u{008F}', '\u{0090}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}', '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}', '\u{009D}', '\u{017E}', '\u{0178}',
];

/// Byte `b` of Windows-1252 as a character.
pub fn cp1252_char(b: u8) -> char {
    if (0x80..0xA0).contains(&b) { CP1252[(b - 0x80) as usize] } else { char::from(b) }
}

/// Character `c` as a Windows-1252 byte, if it has one.
pub fn cp1252_byte(c: char) -> Option<u8> {
    let u = c as u32;
    if u < 0x80 || (0xA0..0x100).contains(&u) {
        return Some(u as u8);
    }
    CP1252.iter().position(|&h| h == c).map(|i| 0x80 + i as u8)
}

// ------------------------------------------------------------- writing --

/// The text (paragraphs separated by '\n') with `rich`'s formats as RTF.
pub fn write(text: &[char], rich: &Rich) -> String {
    // (every character's format, the end's last)
    let mut fmts: Vec<&CharFmt> = Vec::with_capacity(text.len() + 1);
    for (n, f) in rich.formats() {
        fmts.extend(std::iter::repeat_n(f, n));
    }
    let end = rich.end_format(text.len());
    fmts.resize(text.len() + 1, fmts.last().copied().unwrap_or(&end));
    let has_bullet = (0..=text.iter().filter(|c| **c == '\n').count()).any(|p| rich.para(p).bullet);
    // (the tables: in the order the text first uses them)
    let mut fonts: Vec<(&str, i64)> = Vec::new();
    let mut colors: Vec<i64> = Vec::new();
    for f in &fmts {
        if !fonts.iter().any(|(n, c)| *n == f.name && *c == f.charset) {
            fonts.push((&f.name, f.charset));
        }
        if let Some(c) = f.color {
            if !colors.contains(&c) {
                colors.push(c);
            }
        }
    }
    let symbol = has_bullet.then(|| {
        if !fonts.iter().any(|(n, c)| *n == "Symbol" && *c == 2) {
            fonts.push(("Symbol", 2));
        }
        fonts.iter().position(|(n, c)| *n == "Symbol" && *c == 2).unwrap_or(0)
    });
    let mut out = String::with_capacity(text.len() + 256);
    out.push_str("{\\rtf1\\ansi\\ansicpg1252\\deff0\\deflang1033{\\fonttbl");
    for (i, (name, charset)) in fonts.iter().enumerate() {
        out.push_str(&format!("{{\\f{i}\\fnil"));
        if *charset != 1 {
            out.push_str(&format!("\\fcharset{charset}"));
        }
        out.push(' ');
        escape_into(&mut out, name);
        out.push_str(";}");
    }
    out.push_str("}\r\n");
    if !colors.is_empty() {
        out.push_str("{\\colortbl ;");
        for c in &colors {
            out.push_str(&format!("\\red{}\\green{}\\blue{};", c & 0xFF, (c >> 8) & 0xFF, (c >> 16) & 0xFF));
        }
        out.push_str("}\r\n");
    }
    out.push_str("\\viewkind4\\uc1");
    let font_index = |f: &CharFmt| fonts.iter().position(|(n, c)| *n == f.name && *c == f.charset).unwrap_or(0);
    let color_index = |f: &CharFmt| f.color.and_then(|c| colors.iter().position(|x| *x == c)).map_or(0, |i| i + 1);
    let mut cur: Option<CharFmt> = None;
    let mut para: Option<ParaFmt> = None;
    let mut p = 0;
    // (a control word written: a space before text)
    let mut words = false;
    let mut start_para = true;
    for i in 0..=text.len() {
        if start_para {
            let pf = rich.para(p);
            if para.as_ref() != Some(&pf) {
                out.push_str("\\pard");
                para_words(&mut out, &pf, symbol);
                words = true;
                para = Some(pf);
            }
            start_para = false;
        }
        let f = fmts[i];
        // (the format's changes, in RapidQ's rich edit's order)
        let mut w = String::new();
        let first = cur.is_none();
        let c = cur.clone().unwrap_or_else(|| CharFmt { color: None, bold: false, italic: false, underline: false, strike: false, name: String::new(), half_points: 0, charset: f.charset });
        if color_index(&c) != color_index(f) || (first && f.color.is_some()) {
            w.push_str(&format!("\\cf{}", color_index(f)));
        }
        // (bold, underline, italic, strike-out: RC.EXE writes `\cf3\ul\i0`)
        for (on, was, word) in [(f.bold, c.bold, "b"), (f.underline, c.underline, "ul"), (f.italic, c.italic, "i"), (f.strike, c.strike, "strike")] {
            if on != was {
                w.push('\\');
                w.push_str(word);
                if !on {
                    w.push_str(if word == "ul" { "none" } else { "0" });
                }
            }
        }
        if first || font_index(&c) != font_index(f) || c.name != f.name {
            w.push_str(&format!("\\f{}", font_index(f)));
        }
        if first || c.half_points != f.half_points {
            w.push_str(&format!("\\fs{}", f.half_points));
        }
        if !w.is_empty() {
            out.push_str(&w);
            words = true;
            cur = Some(f.clone());
        }
        if i == text.len() {
            if words {
                out.push(' ');
            }
            out.push_str("\r\n\\par }\r\n");
            break;
        }
        let ch = text[i];
        if ch == '\n' {
            if words {
                out.push(' ');
            }
            out.push_str("\r\n\\par ");
            words = false;
            p += 1;
            start_para = true;
            continue;
        }
        if words && !matches!(ch, '\t') {
            out.push(' ');
        }
        words = false;
        match ch {
            '\t' => {
                out.push_str("\\tab");
                words = true;
            }
            '\\' | '{' | '}' => {
                out.push('\\');
                out.push(ch);
            }
            c if (c as u32) < 0x80 => out.push(c),
            c => match cp1252_byte(c) {
                Some(b) => out.push_str(&format!("\\'{b:02x}")),
                None => {
                    // (past the code page: \uN and a ? for readers without it)
                    let mut buf = [0u16; 2];
                    for unit in c.encode_utf16(&mut buf) {
                        out.push_str(&format!("\\u{}?", *unit as i16));
                    }
                }
            },
        }
    }
    out
}

/// A paragraph format's control words (after `\pard`).
fn para_words(out: &mut String, p: &ParaFmt, symbol: Option<usize>) {
    if p.bullet {
        out.push_str(&format!("{{\\pntext\\f{}\\'B7\\tab}}{{\\*\\pn\\pnlvlblt\\pnf{}\\pnindent0{{\\pntxtb\\'B7}}}}", symbol.unwrap_or(0), symbol.unwrap_or(0)));
    }
    match p.align {
        Some(1) => out.push_str("\\qr"),
        Some(2) => out.push_str("\\qc"),
        Some(3) => out.push_str("\\qj"),
        _ => {}
    }
    if p.first != 0 {
        out.push_str(&format!("\\fi{}", p.first));
    }
    if p.left != 0 {
        out.push_str(&format!("\\li{}", p.left));
    }
    if p.right != 0 {
        out.push_str(&format!("\\ri{}", p.right));
    }
}

fn escape_into(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '\\' | '{' | '}' => {
                out.push('\\');
                out.push(c);
            }
            c if (c as u32) < 0x80 => out.push(c),
            c => match cp1252_byte(c) {
                Some(b) => out.push_str(&format!("\\'{b:02x}")),
                None => out.push('?'),
            },
        }
    }
}

// ------------------------------------------------------------- reading --

/// What a document read gives: its text (paragraphs separated by '\n'),
/// each run of characters' format (covering the text and its end), each
/// paragraph's format.
#[derive(Clone, Debug, PartialEq)]
pub struct Doc {
    pub text: Vec<char>,
    pub runs: Vec<(usize, CharFmt)>,
    pub paras: Vec<ParaFmt>,
}

/// Whether `bytes` are RTF (they start `{\rtf`, spaces before allowed).
pub fn is_rtf(bytes: &[u8]) -> bool {
    let s: Vec<u8> = bytes.iter().copied().skip_while(|b| b.is_ascii_whitespace()).take(5).collect();
    s == b"{\\rtf"
}

#[derive(Clone, Debug)]
struct State {
    fmt: CharFmt,
    /// The font table index in effect (`\f`), the colour's (`\cf`).
    font: i64,
    color: usize,
    para: ParaFmt,
    /// Characters a `\u` replaces (`\uc`).
    uc: usize,
    /// Skipping this group (a destination not shown).
    skip: bool,
    /// Hidden text (`\v`).
    hidden: bool,
    /// What the group is: the font table, a font in it, the colour table.
    dest: Dest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Dest {
    Text,
    FontTable,
    ColorTable,
}

/// Reads RTF; `None` if `bytes` aren't RTF. `base` is the format text
/// without one takes (the control's Font).
pub fn read(bytes: &[u8], base: &CharFmt) -> Option<Doc> {
    if !is_rtf(bytes) {
        return None;
    }
    let mut fonts: Vec<(i64, String, i64)> = Vec::new();
    let mut colors: Vec<Option<i64>> = Vec::new();
    let mut deff: i64 = 0;
    let base_state = State { fmt: CharFmt { charset: 0, ..base.clone() }, font: -1, color: 0, para: ParaFmt::default(), uc: 1, skip: false, hidden: false, dest: Dest::Text };
    let mut st = base_state.clone();
    let mut stack: Vec<State> = Vec::new();
    let mut text: Vec<char> = Vec::new();
    let mut runs: Vec<(usize, CharFmt)> = Vec::new();
    let mut paras: Vec<ParaFmt> = Vec::new();
    // (the font name being read in the table, its number and charset)
    let mut font_name = String::new();
    let mut font_no: i64 = 0;
    let mut font_charset: i64 = 1;
    let (mut r, mut g, mut b) = (0i64, 0i64, 0i64);
    let mut color_set = false;
    // (characters still to skip after a \u)
    let mut skip_chars = 0usize;
    // (a \par last: the end of the text, unless more text comes)
    let mut ended_par: Option<CharFmt> = None;
    let mut bullet_seen = false;
    let mut i = 0;
    let n = bytes.len();
    let resolve_font = |fonts: &[(i64, String, i64)], f: i64| fonts.iter().find(|x| x.0 == f).map(|x| (x.1.clone(), x.2));

    // pushes a character in the current format
    fn put(c: char, st: &State, text: &mut Vec<char>, runs: &mut Vec<(usize, CharFmt)>, ended: &mut Option<CharFmt>) {
        if let Some(f) = ended.take() {
            // (the \par before was a paragraph break after all)
            text.push('\n');
            push_run(runs, f, 1);
        }
        text.push(c);
        push_run(runs, st.fmt.clone(), 1);
    }
    fn push_run(runs: &mut Vec<(usize, CharFmt)>, f: CharFmt, n: usize) {
        match runs.last_mut() {
            Some((m, g)) if *g == f => *m += n,
            _ => runs.push((n, f)),
        }
    }

    while i < n {
        let c = bytes[i];
        match c {
            b'{' => {
                stack.push(st.clone());
                i += 1;
                // (an ignorable destination: `{\*\…}`)
                if bytes.get(i) == Some(&b'\\') && bytes.get(i + 1) == Some(&b'*') {
                    st.skip = true;
                    i += 2;
                }
            }
            b'}' => {
                if st.dest == Dest::FontTable && !st.skip && !font_name.is_empty() && stack.last().is_some_and(|s| s.dest == Dest::FontTable) {
                    // (a font entry without its `;`)
                    fonts.push((font_no, std::mem::take(&mut font_name).trim().to_string(), font_charset));
                }
                st = stack.pop().unwrap_or_else(|| base_state.clone());
                if bullet_seen && !st.skip {
                    st.para.bullet = true;
                    bullet_seen = false;
                }
                i += 1;
                if stack.is_empty() {
                    break;
                }
            }
            b'\\' => {
                i += 1;
                let Some(&k) = bytes.get(i) else { break };
                if k.is_ascii_alphabetic() {
                    let s = i;
                    while i < n && bytes[i].is_ascii_alphabetic() {
                        i += 1;
                    }
                    let word = std::str::from_utf8(&bytes[s..i]).unwrap_or("");
                    let ps = i;
                    if i < n && (bytes[i] == b'-' || bytes[i].is_ascii_digit()) {
                        i += 1;
                        while i < n && bytes[i].is_ascii_digit() {
                            i += 1;
                        }
                    }
                    let param: Option<i64> = (i > ps).then(|| std::str::from_utf8(&bytes[ps..i]).ok().and_then(|p| p.parse().ok())).flatten();
                    if i < n && bytes[i] == b' ' {
                        i += 1;
                    }
                    if st.skip {
                        // (a bullet's list level, in its `{\*\pn …}`)
                        if matches!(word, "pnlvlblt" | "pnlvlbody" | "pnlvlcont") {
                            bullet_seen = true;
                        }
                        // (`\bin N`: N bytes of data)
                        if word == "bin" {
                            i += param.unwrap_or(0).max(0) as usize;
                        }
                        continue;
                    }
                    if skip_chars > 0 && st.dest == Dest::Text {
                        skip_chars -= 1;
                        continue;
                    }
                    let on = param.is_none_or(|p| p != 0);
                    match word {
                        // destinations not shown
                        "fonttbl" => st.dest = Dest::FontTable,
                        "colortbl" => {
                            st.dest = Dest::ColorTable;
                            color_set = false;
                        }
                        "stylesheet" | "info" | "pict" | "object" | "header" | "footer" | "headerl" | "headerr" | "headerf" | "footerl" | "footerr" | "footerf" | "fldinst" | "pntext" | "pntxtb" | "pntxta" | "listtable" | "listoverridetable" | "revtbl" | "rsidtbl" | "generator" | "xmlnstbl" | "themedata" | "colorschememapping" | "latentstyles" | "datastore" | "mmathPr" | "filetbl" | "footnote" | "nonshppict" | "shppict" | "bkmkstart" | "bkmkend" | "author" | "title" | "operator" => st.skip = true,
                        "deff" => deff = param.unwrap_or(0),
                        "uc" => st.uc = param.unwrap_or(1).max(0) as usize,
                        "u" => {
                            let v = param.unwrap_or(0);
                            let v = if v < 0 { v + 65536 } else { v } as u32;
                            if st.dest == Dest::Text && !st.hidden {
                                if let Some(ch) = char::from_u32(v) {
                                    put(ch, &st, &mut text, &mut runs, &mut ended_par);
                                }
                            } else if st.dest == Dest::FontTable {
                                if let Some(ch) = char::from_u32(v) {
                                    font_name.push(ch);
                                }
                            }
                            skip_chars = st.uc;
                        }
                        // the font table
                        "f" if st.dest == Dest::FontTable => {
                            font_no = param.unwrap_or(0);
                            font_name.clear();
                            font_charset = 1;
                        }
                        "fcharset" if st.dest == Dest::FontTable => font_charset = param.unwrap_or(1),
                        // the colour table
                        "red" if st.dest == Dest::ColorTable => {
                            r = param.unwrap_or(0);
                            color_set = true;
                        }
                        "green" if st.dest == Dest::ColorTable => {
                            g = param.unwrap_or(0);
                            color_set = true;
                        }
                        "blue" if st.dest == Dest::ColorTable => {
                            b = param.unwrap_or(0);
                            color_set = true;
                        }
                        // character formats
                        "plain" => {
                            st.fmt = CharFmt { charset: st.fmt.charset, ..base_state.fmt.clone() };
                            if let Some((name, cs)) = resolve_font(&fonts, deff) {
                                st.fmt.name = name;
                                st.fmt.charset = cs;
                            }
                            st.hidden = false;
                        }
                        "b" => st.fmt.bold = on,
                        "i" => st.fmt.italic = on,
                        "ul" | "uld" | "uldash" | "uldashd" | "uldashdd" | "uldb" | "ulth" | "ulw" | "ulwave" | "ulhwave" | "ululdbwave" | "ulthd" | "ulthdash" | "ulldash" | "ulthldash" | "ulthdashd" | "ulthdashdd" => st.fmt.underline = on,
                        "ulnone" => st.fmt.underline = false,
                        "strike" | "striked" => st.fmt.strike = on,
                        "v" => st.hidden = on,
                        "cf" => {
                            st.color = param.unwrap_or(0).max(0) as usize;
                            st.fmt.color = colors.get(st.color).copied().flatten();
                        }
                        "f" => {
                            st.font = param.unwrap_or(0);
                            if let Some((name, cs)) = resolve_font(&fonts, st.font) {
                                st.fmt.name = name;
                                st.fmt.charset = cs;
                            }
                        }
                        "fs" => {
                            let hp = param.unwrap_or(24);
                            if hp > 0 {
                                st.fmt.half_points = hp;
                            }
                        }
                        // paragraph formats
                        "pard" => st.para = ParaFmt::default(),
                        "ql" => st.para.align = Some(0),
                        "qr" => st.para.align = Some(1),
                        "qc" => st.para.align = Some(2),
                        "qj" | "qd" => st.para.align = Some(3),
                        "li" => st.para.left = param.unwrap_or(0),
                        "fi" => st.para.first = param.unwrap_or(0),
                        "ri" => st.para.right = param.unwrap_or(0),
                        "pnlvlblt" => st.para.bullet = true,
                        "pnlvlbody" | "pnlvlcont" => st.para.bullet = true,
                        "ls" if param.unwrap_or(0) > 0 => st.para.bullet = true,
                        // text
                        "par" | "sect" | "page" | "row" | "line" if st.dest == Dest::Text && !(st.hidden && word == "line") => {
                            if let Some(f) = ended_par.take() {
                                text.push('\n');
                                push_run(&mut runs, f, 1);
                            }
                            paras.push(st.para.clone());
                            ended_par = Some(st.fmt.clone());
                        }
                        "tab" | "cell" if st.dest == Dest::Text && !st.hidden => put('\t', &st, &mut text, &mut runs, &mut ended_par),
                        w if st.dest == Dest::Text && !st.hidden => {
                            let named = match w {
                                "emdash" => Some('\u{2014}'),
                                "endash" => Some('\u{2013}'),
                                "emspace" => Some('\u{2003}'),
                                "enspace" => Some('\u{2002}'),
                                "bullet" => Some('\u{2022}'),
                                "lquote" => Some('\u{2018}'),
                                "rquote" => Some('\u{2019}'),
                                "ldblquote" => Some('\u{201C}'),
                                "rdblquote" => Some('\u{201D}'),
                                _ => None,
                            };
                            if let Some(ch) = named {
                                put(ch, &st, &mut text, &mut runs, &mut ended_par);
                            }
                        }
                        _ => {}
                    }
                } else {
                    // control symbols
                    i += 1;
                    if st.skip {
                        if k == b'\'' {
                            i += 2;
                        }
                        continue;
                    }
                    if skip_chars > 0 && st.dest == Dest::Text {
                        skip_chars -= 1;
                        if k == b'\'' {
                            i += 2;
                        }
                        continue;
                    }
                    let ch = match k {
                        b'\'' => {
                            let hex = bytes.get(i..i + 2).and_then(|h| std::str::from_utf8(h).ok()).and_then(|h| u8::from_str_radix(h, 16).ok());
                            i += 2;
                            hex.map(cp1252_char)
                        }
                        b'~' => Some('\u{00A0}'),
                        b'_' => Some('\u{2011}'),
                        b'-' => None,
                        b'\\' | b'{' | b'}' => Some(char::from(k)),
                        b'\r' | b'\n' => {
                            // (`\` + a line break: a paragraph)
                            if st.dest == Dest::Text && !st.hidden {
                                if let Some(f) = ended_par.take() {
                                    text.push('\n');
                                    push_run(&mut runs, f, 1);
                                }
                                paras.push(st.para.clone());
                                ended_par = Some(st.fmt.clone());
                            }
                            None
                        }
                        _ => None,
                    };
                    if let Some(ch) = ch {
                        match st.dest {
                            Dest::Text if !st.hidden => put(ch, &st, &mut text, &mut runs, &mut ended_par),
                            Dest::FontTable => font_name.push(ch),
                            _ => {}
                        }
                    }
                }
            }
            b'\r' | b'\n' => i += 1,
            _ => {
                i += 1;
                if st.skip {
                    continue;
                }
                if skip_chars > 0 && st.dest == Dest::Text {
                    skip_chars -= 1;
                    continue;
                }
                match st.dest {
                    Dest::FontTable => {
                        if c == b';' {
                            fonts.push((font_no, std::mem::take(&mut font_name).trim().to_string(), font_charset));
                        } else {
                            font_name.push(cp1252_char(c));
                        }
                    }
                    Dest::ColorTable => {
                        if c == b';' {
                            colors.push(color_set.then_some(r | g << 8 | b << 16));
                            (r, g, b) = (0, 0, 0);
                            color_set = false;
                        }
                    }
                    Dest::Text if !st.hidden => put(cp1252_char(c), &st, &mut text, &mut runs, &mut ended_par),
                    Dest::Text => {}
                }
            }
        }
        // (a font table read: the default font for text without \f)
        if st.dest == Dest::Text && st.font < 0 && !fonts.is_empty() {
            if let Some((name, cs)) = resolve_font(&fonts, deff) {
                if st.fmt.name != name {
                    st.fmt.name = name;
                    st.fmt.charset = cs;
                }
            }
            st.font = deff;
        }
    }
    // (the last paragraph's; the end of the text: the last \par's format,
    // else the format reached)
    let end = match ended_par.take() {
        Some(f) => f,
        None => {
            paras.push(st.para.clone());
            st.fmt.clone()
        }
    };
    push_run(&mut runs, end, 1);
    Some(Doc { text, runs, paras })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rich_of(doc: &Doc) -> Rich {
        let mut r = Rich::default();
        r.load(doc.runs.clone(), doc.paras.clone(), &doc.text);
        r
    }

    #[test]
    fn writes_what_rapidqs_rich_edit_writes() {
        // (RC.EXE w_r1 "text" and "empty")
        let r = Rich::default();
        assert_eq!(write(&[], &r), "{\\rtf1\\ansi\\ansicpg1252\\deff0\\deflang1033{\\fonttbl{\\f0\\fnil MS Sans Serif;}}\r\n\\viewkind4\\uc1\\pard\\f0\\fs16 \r\n\\par }\r\n");
        let t: Vec<char> = "Hello world".chars().collect();
        let mut r = Rich::default();
        r.set_text(0, &t);
        assert_eq!(write(&t, &r), "{\\rtf1\\ansi\\ansicpg1252\\deff0\\deflang1033{\\fonttbl{\\f0\\fnil MS Sans Serif;}}\r\n\\viewkind4\\uc1\\pard\\f0\\fs16 Hello world\r\n\\par }\r\n");
        // (w_r1 "selattr-font": world in Courier New 14, red, bold)
        r.apply(&t, 6, 11, &|f| {
            f.assign(&super::super::font::Font { name: "Courier New".into(), size: 14, color: 0xFF, styles: 1 }, Some(0xFF));
        });
        assert_eq!(
            write(&t, &r),
            "{\\rtf1\\ansi\\ansicpg1252\\deff0\\deflang1033{\\fonttbl{\\f0\\fnil MS Sans Serif;}{\\f1\\fnil\\fcharset0 Courier New;}}\r\n{\\colortbl ;\\red255\\green0\\blue0;}\r\n\\viewkind4\\uc1\\pard\\f0\\fs16 Hello \\cf1\\b\\f1\\fs28 world\\cf0\\b0\\f0\\fs16 \r\n\\par }\r\n"
        );
    }

    #[test]
    fn writes_paragraphs_and_their_formats() {
        let t: Vec<char> = "ab\ncd".chars().collect();
        let mut r = Rich::default();
        r.set_text(0, &t);
        r.apply(&t, 1, 3, &|f| f.italic = true);
        r.apply_para(&t, 3, 3, &|p| p.align = Some(2));
        let s = write(&t, &r);
        assert!(s.contains("\\pard\\f0\\fs16 a\\i b\r\n\\par \\pard\\qc\\i0 cd\r\n\\par }"), "{s}");
    }

    #[test]
    fn reads_back_what_it_writes() {
        let t: Vec<char> = "Plain bold\nred\tcaf\u{e9} \u{20ac} \u{263a}".chars().collect();
        let mut r = Rich::default();
        r.set_text(0, &t);
        r.apply(&t, 6, 10, &|f| f.bold = true);
        r.apply(&t, 11, 14, &|f| f.color = Some(0xFF));
        r.apply_para(&t, 11, 11, &|p| {
            p.align = Some(1);
            p.left = 720;
            p.bullet = true;
        });
        let s = write(&t, &r);
        let d = read(s.as_bytes(), &CharFmt::default()).expect("RTF");
        assert_eq!(d.text, t);
        let back = rich_of(&d);
        assert!(back.fmt_at(7).bold && !back.fmt_at(5).bold);
        assert_eq!(back.fmt_at(12).color, Some(0xFF));
        assert_eq!(back.para(1), ParaFmt { align: Some(1), left: 720, first: 0, right: 0, bullet: true });
        assert_eq!(write(&d.text, &back), s);
    }

    #[test]
    fn reads_a_word_processors_rtf() {
        // (w_r5's input: tables, \'hh, \u with its ?, paragraph formats; the
        // last \par is the end of the text)
        let src = "{\\rtf1\\ansi\\deff0{\\fonttbl{\\f0\\fswiss Arial;}{\\f1\\fmodern Courier New;}}{\\colortbl ;\\red255\\green0\\blue0;\\red0\\green0\\blue255;}\r\n\\pard\\f0\\fs20 Plain \\b bold\\b0  \\i italic\\i0  \\ul under\\ulnone  \\strike strike\\strike0  \\cf1 red\\cf0  \\f1\\fs28 big courier\\f0\\fs20\\par\r\n\\pard\\qc centered line\\par\r\n\\pard\\qr right line\\par\r\n\\pard\\li720 indented\\tab tab\\par\r\ncaf\\'e9 \\u8364? end\\par\r\n}\r\n";
        let d = read(src.as_bytes(), &CharFmt::default()).unwrap();
        let text: String = d.text.iter().collect();
        assert_eq!(text, "Plain bold italic under strike red big courier\ncentered line\nright line\nindented\ttab\ncaf\u{e9} \u{20ac} end");
        let r = rich_of(&d);
        assert_eq!(r.fmt_at(0).name, "Arial");
        assert_eq!(r.fmt_at(0).half_points, 20);
        assert!(r.fmt_at(6).bold && r.fmt_at(11).italic && r.fmt_at(18).underline && r.fmt_at(25).strike);
        assert_eq!(r.fmt_at(31).color, Some(0xFF));
        assert_eq!((r.fmt_at(40).name.as_str(), r.fmt_at(40).half_points), ("Courier New", 28));
        assert_eq!((r.para(1).align, r.para(2).align, r.para(3).left, r.para(4).left), (Some(2), Some(1), 720, 720));
        assert!(read(b"plain text", &CharFmt::default()).is_none());
        // (skipped destinations, fields' results, escapes)
        let d = read(b"{\\rtf1{\\*\\generator X;}{\\info{\\title T}}a\\{b\\}\\\\{\\field{\\*\\fldinst HYPERLINK x}{\\fldrslt link}}\\emdash}", &CharFmt::default()).unwrap();
        assert_eq!(d.text.iter().collect::<String>(), "a{b}\\link\u{2014}");
    }
}
