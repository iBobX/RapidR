//! RMARKDOWNVIEW's model: a Markdown text (CommonMark with GitHub's tables
//! and strikethrough, read by pulldown-cmark) as the flat list of
//! paragraphs the kernel lays out — headings, text, code blocks, table
//! cells, rules — each with its styled runs (bold, italic, struck out,
//! code, a link), its list depth and marker, its block-quote depth; and
//! what the user does while reading it (the selection, a link clicked).
//! The kernel draws it (`rapidr-ui-kernel`'s
//! `components/panels/markdown.rs`); the program's members and events go
//! through the runtime glue here. RapidR Studio shows `.md` files with it
//! (Preview | Source).

use std::ops::Range;
use std::rc::Rc;

use pulldown_cmark::{Alignment, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use super::runtime::{basic_bool, truth, Runtime};
use crate::Value;

/// How a run of text is drawn (`Default`: as the paragraph).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    /// Inline code (monospaced, on a tinted ground).
    pub code: bool,
    /// A link: its index in [`Doc::links`].
    pub link: Option<usize>,
}

/// What a paragraph is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A heading, level 1 to 6.
    Heading(u8),
    /// Text: a paragraph, a list item's text.
    Text,
    /// A code block (fenced or indented), its language (the fence's first
    /// word, lower case; "" when none).
    Code(String),
    /// A table's cell: the table (index in [`Doc::tables`]), its row (0:
    /// the header) and column.
    Cell { table: usize, row: usize, col: usize },
    /// A horizontal rule.
    Rule,
}

/// A paragraph to lay out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Para {
    pub kind: Kind,
    /// Its text (a hard line break is a `\n`).
    pub text: String,
    /// Its styled runs, in order, not overlapping (byte ranges of `text`).
    pub runs: Vec<(Range<usize>, Style)>,
    /// How deep in lists it is (0: not in a list).
    pub depth: usize,
    /// How deep in block quotes.
    pub quote: usize,
    /// The list item's marker drawn left of it ("•", "3."), on the item's
    /// first paragraph.
    pub marker: Option<String>,
    /// In a tight list (no space between its items).
    pub tight: bool,
    /// The source line it starts on (from 0).
    pub line: usize,
}

/// A table: its columns' alignment and how many rows it has (the header
/// included).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Table {
    /// Per column: 0 left, 1 centre, 2 right.
    pub align: Vec<u8>,
    pub rows: usize,
}

/// A Markdown text, read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Doc {
    pub paras: Vec<Para>,
    /// The links' targets, as written.
    pub links: Vec<String>,
    pub tables: Vec<Table>,
}

impl Doc {
    /// Reads `text` (CommonMark, GitHub's tables and strikethrough; HTML is
    /// left out, an image shows its description).
    pub fn parse(text: &str) -> Doc {
        let mut b = Builder::default();
        let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
        let breaks: Vec<usize> = text.match_indices('\n').map(|(i, _)| i).collect();
        for (ev, range) in Parser::new_ext(text, opts).into_offset_iter() {
            b.line = breaks.partition_point(|&i| i < range.start);
            b.event(ev);
        }
        b.flush();
        b.doc
    }

    /// Every paragraph's text, a line each (a table's row as its cells
    /// separated by tabs): what a reader sees, without the marks.
    pub fn plain_text(&self) -> String {
        let mut out = String::new();
        for (i, p) in self.paras.iter().enumerate() {
            if i > 0 {
                let same_row = matches!((&self.paras[i - 1].kind, &p.kind), (Kind::Cell { table: a, row: r, .. }, Kind::Cell { table: b, row: s, .. }) if a == b && r == s);
                out.push(if same_row { '\t' } else { '\n' });
            }
            if let Some(m) = &p.marker {
                out.push_str(m);
                out.push(' ');
            }
            match p.kind {
                Kind::Rule => out.push_str("---"),
                _ => out.push_str(&p.text),
            }
        }
        out
    }

    /// The headings: (level, text, anchor, source line) — the anchor as
    /// GitHub makes it (`#getting-started`).
    pub fn headings(&self) -> Vec<(u8, String, String, usize)> {
        let mut seen: Vec<String> = Vec::new();
        self.paras
            .iter()
            .filter_map(|p| match p.kind {
                Kind::Heading(l) => {
                    let base = slug(&p.text);
                    let n = seen.iter().filter(|s| **s == base).count();
                    seen.push(base.clone());
                    Some((l, p.text.clone(), if n == 0 { base } else { format!("{base}-{n}") }, p.line))
                }
                _ => None,
            })
            .collect()
    }

    /// The paragraph of the heading an anchor (`#getting-started`, any
    /// case) names.
    pub fn anchor_para(&self, anchor: &str) -> Option<usize> {
        let want = anchor.trim_start_matches('#').to_lowercase();
        let heads = self.headings();
        let k = heads.iter().position(|h| h.2 == want)?;
        self.paras.iter().enumerate().filter(|(_, p)| matches!(p.kind, Kind::Heading(_))).nth(k).map(|(i, _)| i)
    }

    /// The link at byte `at` of paragraph `para`.
    pub fn link_at(&self, para: usize, at: usize) -> Option<usize> {
        let p = self.paras.get(para)?;
        p.runs.iter().find(|(r, _)| r.start <= at && at < r.end).and_then(|(_, s)| s.link)
    }

    /// The text between two places ((paragraph, byte), in order), the
    /// paragraphs' breaks as line breaks (a table row's cells by tabs).
    pub fn text_between(&self, from: (usize, usize), to: (usize, usize)) -> String {
        let mut out = String::new();
        for i in from.0..=to.0.min(self.paras.len().saturating_sub(1)) {
            let Some(p) = self.paras.get(i) else { break };
            let a = if i == from.0 { from.1 } else { 0 }.min(p.text.len());
            let b = if i == to.0 { to.1 } else { p.text.len() }.min(p.text.len());
            if i > from.0 {
                let same_row = matches!((&self.paras[i - 1].kind, &p.kind), (Kind::Cell { table: x, row: r, .. }, Kind::Cell { table: y, row: s, .. }) if x == y && r == s);
                out.push(if same_row { '\t' } else { '\n' });
            }
            if a < b && p.text.is_char_boundary(a) && p.text.is_char_boundary(b) {
                out.push_str(&p.text[a..b]);
            }
        }
        out
    }
}

/// A heading's anchor as GitHub makes it: lower case, spaces as `-`,
/// punctuation other than `-` and `_` left out.
pub fn slug(text: &str) -> String {
    text.trim()
        .to_lowercase()
        .chars()
        .filter_map(|c| match c {
            ' ' => Some('-'),
            c if c.is_alphanumeric() || c == '-' || c == '_' => Some(c),
            _ => None,
        })
        .collect()
}

/// pulldown-cmark's events made paragraphs.
#[derive(Default)]
struct Builder {
    doc: Doc,
    cur: Option<Para>,
    /// Nested styles' counts (an emphasis inside an emphasis).
    bold: usize,
    italic: usize,
    strike: usize,
    links: Vec<usize>,
    /// The lists open: each one's next number (`None`: bulleted) and
    /// whether it is tight.
    lists: Vec<(Option<u64>, bool)>,
    quote: usize,
    /// The marker for the next paragraph (a list item's first).
    marker: Option<String>,
    /// A code block being read: its language and text.
    code: Option<(String, String)>,
    /// A table being read: its index, the row, the column.
    table: Option<(usize, usize, usize)>,
    heading: Option<u8>,
    /// The source line the event being read starts on.
    line: usize,
}

impl Builder {
    fn flush(&mut self) {
        if let Some(p) = self.cur.take() {
            self.doc.paras.push(p);
        }
    }

    /// The paragraph text goes into (begun if none is).
    fn para(&mut self) -> &mut Para {
        if self.cur.is_none() {
            let kind = match (self.heading, self.table) {
                (Some(l), _) => Kind::Heading(l),
                (_, Some((t, r, c))) => Kind::Cell { table: t, row: r, col: c },
                _ => Kind::Text,
            };
            self.begin(kind);
        }
        self.cur.as_mut().expect("begun")
    }

    fn begin(&mut self, kind: Kind) {
        self.flush();
        let in_list = !matches!(kind, Kind::Cell { .. });
        self.cur = Some(Para {
            kind,
            text: String::new(),
            runs: Vec::new(),
            depth: if in_list { self.lists.len() } else { 0 },
            quote: self.quote,
            marker: if in_list { self.marker.take() } else { None },
            tight: self.lists.last().is_some_and(|l| l.1),
            line: self.line,
        });
    }

    fn current_style(&self) -> Style {
        Style { bold: self.bold > 0, italic: self.italic > 0, strike: self.strike > 0, code: false, link: self.links.last().copied() }
    }

    fn push_text(&mut self, t: &str, code: bool) {
        if t.is_empty() {
            return;
        }
        let mut st = self.current_style();
        st.code = code;
        let p = self.para();
        let start = p.text.len();
        p.text.push_str(t);
        let end = p.text.len();
        if st != Style::default() {
            match p.runs.last_mut() {
                Some((r, s)) if *s == st && r.end == start => r.end = end,
                _ => p.runs.push((start..end, st)),
            }
        }
    }

    fn event(&mut self, ev: Event) {
        match ev {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(t) => {
                if let Some((_, code)) = self.code.as_mut() {
                    code.push_str(&t);
                } else {
                    self.push_text(&t, false);
                }
            }
            Event::Code(t) => self.push_text(&t, true),
            Event::InlineMath(t) | Event::DisplayMath(t) => self.push_text(&t, true),
            Event::SoftBreak => self.push_text(" ", false),
            Event::HardBreak => self.push_text("\n", false),
            Event::Rule => {
                self.begin(Kind::Rule);
                self.flush();
            }
            Event::InlineHtml(h) => {
                // (a line break written as HTML; other tags are left out)
                let h = h.trim().to_ascii_lowercase();
                if h == "<br>" || h == "<br/>" || h == "<br />" {
                    self.push_text("\n", false);
                }
            }
            Event::FootnoteReference(r) => self.push_text(&format!("[{r}]"), false),
            Event::TaskListMarker(done) => self.push_text(if done { "[x] " } else { "[ ] " }, false),
            Event::Html(_) => {}
        }
    }

    fn start(&mut self, tag: Tag) {
        match tag {
            Tag::Paragraph => {
                // (a list whose items are paragraphs isn't tight)
                if self.table.is_none() {
                    if let Some(l) = self.lists.last_mut() {
                        l.1 = false;
                    }
                }
                let kind = match self.table {
                    Some((t, r, c)) => Kind::Cell { table: t, row: r, col: c },
                    None => Kind::Text,
                };
                self.begin(kind);
            }
            Tag::Heading { level, .. } => {
                let l = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };
                self.heading = Some(l);
                self.begin(Kind::Heading(l));
            }
            Tag::BlockQuote(_) => {
                self.flush();
                self.quote += 1;
            }
            Tag::CodeBlock(kind) => {
                self.flush();
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => info.split(|c: char| c.is_whitespace() || c == ',' || c == '{').next().unwrap_or("").to_lowercase(),
                    CodeBlockKind::Indented => String::new(),
                };
                self.code = Some((lang, String::new()));
            }
            Tag::List(start) => {
                self.flush();
                self.lists.push((start, true));
            }
            Tag::Item => {
                self.flush();
                let depth = self.lists.len();
                let marker = match self.lists.last_mut() {
                    Some((Some(n), _)) => {
                        let m = format!("{n}.");
                        *n += 1;
                        m
                    }
                    _ => (if depth > 1 { "◦" } else { "•" }).to_string(),
                };
                self.marker = Some(marker);
            }
            Tag::Table(aligns) => {
                self.flush();
                let align = aligns
                    .iter()
                    .map(|a| match a {
                        Alignment::Center => 1,
                        Alignment::Right => 2,
                        _ => 0,
                    })
                    .collect();
                self.doc.tables.push(Table { align, rows: 0 });
                self.table = Some((self.doc.tables.len() - 1, 0, 0));
            }
            Tag::TableHead | Tag::TableRow => {
                if let Some((t, _, _)) = self.table {
                    let r = self.doc.tables[t].rows;
                    self.doc.tables[t].rows += 1;
                    self.table = Some((t, r, 0));
                }
            }
            Tag::TableCell => {
                if let Some((t, r, c)) = self.table {
                    self.begin(Kind::Cell { table: t, row: r, col: c });
                }
            }
            Tag::Emphasis => self.italic += 1,
            Tag::Strong => self.bold += 1,
            Tag::Strikethrough => self.strike += 1,
            Tag::Link { dest_url, .. } => {
                self.doc.links.push(dest_url.to_string());
                self.links.push(self.doc.links.len() - 1);
            }
            Tag::Image { .. } => {
                // (its description, in italics, stands for it)
                self.italic += 1;
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => {
                // (a cell's paragraph: the cell goes on)
                if self.table.is_none() {
                    self.flush();
                }
            }
            TagEnd::Heading(_) => {
                // (an empty heading still is one)
                self.para();
                self.heading = None;
                self.flush();
            }
            TagEnd::BlockQuote(_) => {
                self.flush();
                self.quote = self.quote.saturating_sub(1);
            }
            TagEnd::CodeBlock => {
                if let Some((lang, mut text)) = self.code.take() {
                    if text.ends_with('\n') {
                        text.pop();
                    }
                    self.begin(Kind::Code(lang));
                    if let Some(p) = self.cur.as_mut() {
                        p.text = text;
                    }
                    self.flush();
                }
            }
            TagEnd::List(_) => {
                self.flush();
                self.lists.pop();
                self.marker = None;
            }
            TagEnd::Item => {
                // (an item with nothing in it still shows its marker)
                if self.marker.is_some() {
                    self.begin(Kind::Text);
                }
                self.flush();
            }
            TagEnd::Table => {
                self.flush();
                self.table = None;
            }
            TagEnd::TableHead | TagEnd::TableRow => self.flush(),
            TagEnd::TableCell => {
                if let Some((t, r, c)) = self.table {
                    if self.cur.is_none() {
                        self.begin(Kind::Cell { table: t, row: r, col: c });
                    }
                    self.flush();
                    self.table = Some((t, r, c + 1));
                }
            }
            TagEnd::Emphasis => self.italic = self.italic.saturating_sub(1),
            TagEnd::Strong => self.bold = self.bold.saturating_sub(1),
            TagEnd::Strikethrough => self.strike = self.strike.saturating_sub(1),
            TagEnd::Link => {
                self.links.pop();
            }
            TagEnd::Image => {
                self.italic = self.italic.saturating_sub(1);
            }
            _ => {}
        }
    }
}

/// A place in the text: a paragraph and a byte of its text.
pub type Place = (usize, usize);

/// An RMARKDOWNVIEW's model.
#[derive(Clone, Debug)]
pub struct Markdown {
    /// The Markdown text.
    pub text: String,
    /// Goes up when the text changes (the kernel lays it out again).
    pub rev: u64,
    doc: Option<Rc<Doc>>,
    /// The file it was loaded from ("" when set as Text).
    pub file_name: String,
    /// Web links (http, https, mailto) open in the browser when clicked.
    pub open_links: bool,
    /// The selection: where it started and where it ends now.
    pub selection: Option<(Place, Place)>,
    /// A heading to show at the top at the next paint (ScrollTo, a `#`
    /// link): its paragraph.
    pub scroll_to: Option<usize>,
    /// What EmptyText says while there is no text.
    pub empty_text: String,
}

impl Default for Markdown {
    fn default() -> Self {
        Markdown { text: String::new(), rev: 1, doc: None, file_name: String::new(), open_links: true, selection: None, scroll_to: None, empty_text: String::new() }
    }
}

crate::panel_models!(Markdown);

impl Markdown {
    /// The text, read (once per change).
    pub fn doc(&mut self) -> Rc<Doc> {
        if self.doc.is_none() {
            self.doc = Some(Rc::new(Doc::parse(&self.text)));
        }
        self.doc.clone().expect("read above")
    }

    /// A new text: read again, the selection gone (the scroll stays, as an
    /// editor's preview keeps its place while the source is typed).
    pub fn set_text(&mut self, text: &str) {
        if self.text == text && self.doc.is_some() {
            return;
        }
        self.text = text.to_string();
        self.doc = None;
        self.selection = None;
        self.rev += 1;
    }

    /// The selected text ("" when nothing is).
    pub fn selected_text(&mut self) -> String {
        let Some((a, b)) = self.selection else { return String::new() };
        let (from, to) = if a <= b { (a, b) } else { (b, a) };
        if from == to {
            return String::new();
        }
        self.doc().text_between(from, to)
    }

    /// Everything selected.
    pub fn select_all(&mut self) {
        let doc = self.doc();
        if let Some(last) = doc.paras.len().checked_sub(1) {
            self.selection = Some(((0, 0), (last, doc.paras[last].text.len())));
        }
    }
}

/// The folder of a file's path ("" for none), with its separator.
pub fn folder_of(path: &str) -> String {
    match path.rfind(['/', '\\']) {
        Some(i) => path[..=i].to_string(),
        None => String::new(),
    }
}

/// Whether a link goes to the web (opened in the browser): http, https,
/// mailto.
pub fn is_web_link(url: &str) -> bool {
    let u = url.trim().to_ascii_lowercase();
    u.starts_with("http://") || u.starts_with("https://") || u.starts_with("mailto:")
}

/// What the user did to it (from the kernel).
#[derive(Clone, Debug, PartialEq)]
pub enum User {
    /// A link clicked (or Enter / a screen reader's click on it): its
    /// target as written. OnLinkClick (Url); a web link opens in the
    /// browser (OpenLinks), a `#heading` scrolls to it.
    Link(String),
}

/// Its methods; `None`: not one of its own.
pub fn rt_method<R: Runtime>(rt: R, name: &str, method: &str, args: &[Value]) -> Option<Value> {
    let s = |k: usize| args.get(k).map(Value::to_string_val).unwrap_or_default();
    Some(match method {
        "loadfromfile" => {
            let path = s(0);
            let text = rt.read_file(&path);
            let ok = text.is_some();
            with_mut(name, |m| {
                m.set_text(&text.unwrap_or_default());
                m.file_name = path;
            });
            basic_bool(ok)
        }
        "clear" => {
            with_mut(name, |m| {
                m.set_text("");
                m.file_name.clear();
            });
            Value::Null
        }
        "selectall" => {
            with_mut(name, |m| m.select_all());
            Value::Null
        }
        "scrollto" => {
            // (a heading's anchor or its text)
            let want = s(0);
            let found = with_mut(name, |m| {
                let doc = m.doc();
                let at = doc.anchor_para(&want).or_else(|| doc.paras.iter().position(|p| matches!(p.kind, Kind::Heading(_)) && p.text.eq_ignore_ascii_case(want.trim())));
                m.scroll_to = at;
                at.is_some()
            });
            basic_bool(found)
        }
        _ => return None,
    })
}

/// Its properties its model answers.
pub fn rt_get<R: Runtime>(_rt: R, name: &str, prop: &str) -> Option<Value> {
    with_mut(name, |m| {
        Some(match prop {
            "text" => Value::String(m.text.clone()),
            "filename" => Value::String(m.file_name.clone()),
            "openlinks" => basic_bool(m.open_links),
            "emptytext" => Value::String(m.empty_text.clone()),
            "plaintext" => Value::String(m.doc().plain_text()),
            "seltext" => Value::String(m.selected_text()),
            "linkcount" => Value::Integer(m.doc().links.len() as i64),
            "headings" => Value::String(m.doc().headings().iter().map(|(l, t, a, n)| format!("{l}\t{t}\t{a}\t{}\n", n + 1)).collect()),
            _ => return None,
        })
    })
}

/// Its properties its model keeps: whether `prop` was one.
pub fn rt_set<R: Runtime>(_rt: R, name: &str, prop: &str, v: &Value) -> bool {
    match prop {
        "text" => with_mut(name, |m| m.set_text(&v.to_string_val())),
        "openlinks" => with_mut(name, |m| m.open_links = truth(v)),
        "emptytext" => with_mut(name, |m| m.empty_text = v.to_string_val()),
        _ => return false,
    }
    true
}

/// What the user did.
pub fn rt_user<R: Runtime>(rt: R, name: &str, action: User) {
    match action {
        User::Link(url) => {
            if url.starts_with('#') {
                with_mut(name, |m| {
                    let at = m.doc().anchor_para(&url);
                    if at.is_some() {
                        m.scroll_to = at;
                    }
                });
            } else if is_web_link(&url) && with(name, |m| m.open_links).unwrap_or(true) {
                rt.open_url(url.trim());
            }
            rt.fire(name, "onlinkclick", &[Value::String(url)]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_and_runs() {
        let d = Doc::parse("# Title\n\nSome **bold**, *it*, ~~gone~~ and `code` [a link](other.md).\n\n---\n\n> quoted\n");
        assert_eq!(d.paras[0].kind, Kind::Heading(1));
        assert_eq!(d.paras[0].text, "Title");
        let p = &d.paras[1];
        assert_eq!(p.text, "Some bold, it, gone and code a link.");
        let style_of = |w: &str| {
            let at = p.text.find(w).unwrap();
            p.runs.iter().find(|(r, _)| r.start <= at && at < r.end).map(|(_, s)| *s).unwrap_or_default()
        };
        assert!(style_of("bold").bold);
        assert!(style_of("it,").italic);
        assert!(style_of("gone").strike);
        assert!(style_of("code").code);
        assert_eq!(style_of("a link").link, Some(0));
        assert_eq!(d.links, vec!["other.md".to_string()]);
        assert_eq!(d.paras[2].kind, Kind::Rule);
        assert_eq!((d.paras[3].text.as_str(), d.paras[3].quote), ("quoted", 1));
    }

    #[test]
    fn lists_code_and_tables() {
        let d = Doc::parse("- one\n- two\n  1. a\n  2. b\n\n```basic\nPRINT \"hi\"\n```\n\n| A | B |\n|---|--:|\n| 1 | 2 |\n");
        let items: Vec<_> = d.paras.iter().take(4).map(|p| (p.marker.clone().unwrap_or_default(), p.text.clone(), p.depth)).collect();
        assert_eq!(items, vec![("•".into(), "one".into(), 1), ("•".into(), "two".into(), 1), ("1.".into(), "a".into(), 2), ("2.".into(), "b".into(), 2)]);
        assert_eq!(d.paras[4].kind, Kind::Code("basic".into()));
        assert_eq!(d.paras[4].text, "PRINT \"hi\"");
        let cells: Vec<_> = d.paras[5..].iter().map(|p| (p.kind.clone(), p.text.clone())).collect();
        assert_eq!(cells[0], (Kind::Cell { table: 0, row: 0, col: 0 }, "A".into()));
        assert_eq!(cells[3], (Kind::Cell { table: 0, row: 1, col: 1 }, "2".into()));
        assert_eq!(d.tables[0], Table { align: vec![0, 2], rows: 2 });
        assert!(d.plain_text().ends_with("A\tB\n1\t2"));
    }

    #[test]
    fn headings_anchors_selection() {
        let mut m = Markdown::default();
        m.set_text("# Getting started\n\ntext\n\n## Getting started\n\n[go](#getting-started-1)\n");
        let doc = m.doc();
        assert_eq!(doc.headings()[1].2, "getting-started-1");
        assert_eq!(doc.headings()[1].3, 4);
        assert_eq!(doc.anchor_para("#Getting-Started-1"), Some(2));
        assert_eq!(doc.link_at(3, 0), Some(0));
        m.selection = Some(((1, 2), (0, 4)));
        assert_eq!(m.selected_text(), "ing started\nte");
        m.select_all();
        assert!(m.selected_text().starts_with("Getting started\ntext"));
    }

    #[test]
    fn html_left_out_images_described() {
        let d = Doc::parse("<p align=\"center\"><img src=\"x.png\"></p>\n\n[![Release](https://x/badge.svg)](https://example.com) line<br>two\n");
        assert_eq!(d.paras.len(), 1);
        assert_eq!(d.paras[0].text, "Release line\ntwo");
        assert!(d.paras[0].runs[0].1.italic);
        assert_eq!(d.links, vec!["https://example.com".to_string()]);
        assert!(is_web_link(" HTTPS://example.com"));
        assert!(!is_web_link("docs/other.md"));
        assert_eq!(slug("RapidQ's names — today!"), "rapidqs-names--today");
    }
}
