//! ROUTPUTCONSOLE's model (docs/ide-components.md §3.8): a program's
//! output on a [`screen::Screen`] (ANSI colours, CLS, LOCATE), the build
//! log on another, the problems found, the search, and what the kernel
//! keeps while the user reads it (the scroll, the selected lines, the
//! link under the mouse). The kernel draws it
//! (`rapidr-ui-kernel`'s `components/panels/console.rs`); the program's
//! members and events go through the runtime glue here.

pub mod links;
pub mod screen;

use super::runtime::{basic_bool, truth, Runtime};
use crate::Value;
use screen::Screen;

/// A console's pages.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    Output,
    Build,
    Problems,
}

impl Page {
    pub const ALL: [Page; 3] = [Page::Output, Page::Build, Page::Problems];

    /// Its name as the program writes it (`Page = "build"`).
    pub fn name(self) -> &'static str {
        match self {
            Page::Output => "output",
            Page::Build => "build",
            Page::Problems => "problems",
        }
    }

    /// Its tab's title.
    pub fn title(self) -> &'static str {
        match self {
            Page::Output => "Output",
            Page::Build => "Build",
            Page::Problems => "Problems",
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }

    /// The page a name (any case) or a number (0, 1, 2) means.
    pub fn parse(s: &str) -> Option<Page> {
        let s = s.trim().to_ascii_lowercase();
        Page::ALL.into_iter().find(|p| p.name() == s).or_else(|| s.parse::<usize>().ok().and_then(|i| Page::ALL.get(i).copied()))
    }
}

/// How bad a problem is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Severity {
    #[default]
    Error,
    Warning,
    Info,
}

impl Severity {
    pub fn parse(s: &str) -> Severity {
        match s.trim().to_ascii_lowercase().as_str() {
            "warning" | "warn" | "w" | "1" => Severity::Warning,
            "info" | "information" | "hint" | "note" | "i" | "2" => Severity::Info,
            _ => Severity::Error,
        }
    }

    /// Its name: the marker icon's (`rapidr_icons::marker`) and the text's.
    pub fn name(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }

    /// What a screen reader says before the message.
    pub fn title(self) -> &'static str {
        match self {
            Severity::Error => "Error",
            Severity::Warning => "Warning",
            Severity::Info => "Information",
        }
    }
}

/// A problem on the Problems page.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Problem {
    pub file: String,
    pub line: i64,
    pub col: i64,
    pub severity: Severity,
    pub message: String,
    /// A file's heading over its problems (AddProblem's severity `file`, or
    /// `file:<icon>` naming its icon): the file (its folder shown dimmed),
    /// `line` how many problems it has (a pill), and in `message` how many
    /// of each kind in words ("2 errors, 1 warning": what a screen reader
    /// hears). Not a problem: the counts leave it out. Under headings the
    /// problems show their line and column in a column of their own, not
    /// their file again.
    pub heading: bool,
    /// A heading's icon (RapidR's set; "": the file's, by its type).
    pub icon: String,
}

impl Problem {
    /// Where it is, `file:line:col` (as much as is known).
    pub fn place(&self) -> String {
        match (self.line, self.col) {
            (l, c) if l > 0 && c > 0 => format!("{}:{l}:{c}", self.file),
            (l, _) if l > 0 => format!("{}:{l}", self.file),
            _ => self.file.clone(),
        }
    }

    /// Its line of text: `Main.rr:12:5: error: message` (a heading's:
    /// `Main.rr: 2 errors`).
    pub fn text(&self) -> String {
        if self.heading {
            return format!("{}: {}", self.file, self.message);
        }
        if self.file.is_empty() {
            format!("{}: {}", self.severity.name(), self.message)
        } else {
            format!("{}: {}: {}", self.place(), self.severity.name(), self.message)
        }
    }

    /// What a screen reader is told: "Error: message, Main.rr line 12".
    pub fn spoken(&self) -> String {
        if self.heading {
            return format!("{}, {}", self.file, self.message);
        }
        let mut s = format!("{}: {}", self.severity.title(), self.message);
        if !self.file.is_empty() {
            s.push_str(&format!(", {}", self.file));
            if self.line > 0 {
                s.push_str(&format!(" line {}", self.line));
            }
        }
        s
    }
}

/// What the user sees of a page: where it's scrolled, whether it follows
/// new output, the selected lines (absolute line numbers: `Screen::first`
/// and on).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct View {
    /// New output scrolls it to its end (AutoScroll, and the user hasn't
    /// scrolled up).
    pub follow: bool,
    /// Its top line (absolute) while it doesn't follow.
    pub top: u64,
    /// How far into that line the page is scrolled, in pixels (a wrapped
    /// line takes several rows; the wheel stops between them).
    pub sub: i64,
    /// The selection: the line it started at and the line it reaches (the
    /// caret). A Problems page's focused row is `caret`.
    pub anchor: Option<u64>,
    pub caret: Option<u64>,
    /// (RapidR's) A text selection's columns, in characters from 0: the
    /// anchor's on its line and the caret's on its (a drag, a double
    /// click). `None`: the lines from the anchor's to the caret's are
    /// selected whole (a click, the keys, a match found).
    pub cols: Option<(usize, usize)>,
}

impl Default for View {
    fn default() -> Self {
        View { follow: true, top: 0, sub: 0, anchor: None, caret: None, cols: None }
    }
}

impl View {
    /// The selected lines, first to last (absolute).
    pub fn selection(&self) -> Option<(u64, u64)> {
        let (a, c) = (self.anchor?, self.caret?);
        Some((a.min(c), a.max(c)))
    }

    /// A text selection's ends, first to last: (line, character) each
    /// (`None`: no text selection — whole lines or nothing).
    pub fn text_range(&self) -> Option<((u64, usize), (u64, usize))> {
        let (ac, cc) = self.cols?;
        let (a, c) = ((self.anchor?, ac), (self.caret?, cc));
        Some(if a <= c { (a, c) } else { (c, a) })
    }

    /// The characters of absolute line `line` the selection covers, as
    /// `(from, to)` (`to` `None`: to its end, its line break with it) —
    /// `None`: none of it.
    pub fn cols_on(&self, line: u64) -> Option<(usize, Option<usize>)> {
        match self.text_range() {
            Some(((la, ca), (lb, cb))) => {
                if line < la || line > lb || (la == lb && ca == cb) {
                    return None;
                }
                let from = if line == la { ca } else { 0 };
                let to = if line == lb { Some(cb) } else { None };
                Some((from, to))
            }
            None => self.selection().filter(|(a, b)| (*a..=*b).contains(&line)).map(|_| (0, None)),
        }
    }
}

/// A search match: an absolute line, its characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Match {
    pub line: u64,
    pub start: usize,
    pub end: usize,
}

/// ROUTPUTCONSOLE's state.
#[derive(Clone, Debug, PartialEq)]
pub struct Console {
    pub output: Screen,
    pub build: Screen,
    pub problems: Vec<Problem>,
    pub page: Page,
    pub show_tabs: bool,
    pub auto_scroll: bool,
    /// (RapidR's, on by default) A line longer than the page is wide wraps
    /// onto more rows; off, it's cut at the edge.
    pub word_wrap: bool,
    pub max_lines: usize,
    /// The search box's text.
    pub filter: String,
    /// Each page's view (by `Page::index`).
    pub views: [View; 3],
    /// The current match (an index into [`Console::matches`]).
    pub current: Option<usize>,
    /// (the kernel's) The link under the mouse: absolute line, characters.
    pub hover_link: Option<Match>,
    /// (the kernel's) The tab, the problem row under the mouse.
    pub hover_tab: Option<usize>,
    pub hover_row: Option<usize>,
    /// (the kernel's) The line (absolute; a problem's row) the mouse last
    /// pressed on, either button: what a context menu's Copy Line copies.
    pub pressed_line: Option<u64>,
    /// The matches as last found, and for what (page, text revision,
    /// filter).
    found: Vec<Match>,
    found_for: Option<(Page, u64, u64, String)>,
    /// Problems' revision (their changes), for the search.
    problems_rev: u64,
}

impl Default for Console {
    fn default() -> Self {
        Console {
            output: Screen::new(5000),
            build: Screen::new(5000),
            problems: Vec::new(),
            page: Page::Output,
            show_tabs: true,
            auto_scroll: true,
            word_wrap: true,
            max_lines: 5000,
            filter: String::new(),
            // (the Problems page reads from its top — errors first —, never
            // following new rows to the end as Output does)
            views: [View::default(), View::default(), View { follow: false, ..View::default() }],
            current: None,
            hover_link: None,
            hover_tab: None,
            hover_row: None,
            pressed_line: None,
            found: Vec::new(),
            found_for: None,
            problems_rev: 0,
        }
    }
}

crate::panel_models!(Console);

impl Console {
    /// Page `p`'s screen (`None`: the Problems page).
    pub fn screen(&self, p: Page) -> Option<&Screen> {
        match p {
            Page::Output => Some(&self.output),
            Page::Build => Some(&self.build),
            Page::Problems => None,
        }
    }

    fn screen_mut(&mut self, p: Page) -> Option<&mut Screen> {
        match p {
            Page::Output => Some(&mut self.output),
            Page::Build => Some(&mut self.build),
            Page::Problems => None,
        }
    }

    /// The shown page's view.
    pub fn view(&self) -> &View {
        &self.views[self.page.index()]
    }

    pub fn view_mut(&mut self) -> &mut View {
        &mut self.views[self.page.index()]
    }

    /// The first absolute line of page `p` (0 for the problems).
    pub fn first(&self, p: Page) -> u64 {
        self.screen(p).map_or(0, |s| s.first)
    }

    /// Page `p`'s text revision: it goes up with every change to what the
    /// page shows (a layout of its lines is good while it stays).
    pub fn rev(&self, p: Page) -> u64 {
        self.screen(p).map_or(self.problems_rev, |s| s.rev)
    }

    /// How many lines (rows) page `p` has.
    pub fn count(&self, p: Page) -> usize {
        self.screen(p).map_or(self.problems.len(), Screen::len)
    }

    /// Page `p`'s line `i` (from 0), without colours.
    pub fn line_text(&self, p: Page, i: usize) -> Option<String> {
        match self.screen(p) {
            Some(s) => s.line(i).map(|l| l.text().to_string()),
            None => self.problems.get(i).map(Problem::text),
        }
    }

    /// Page `p`'s text.
    pub fn text(&self, p: Page) -> String {
        match self.screen(p) {
            Some(s) => s.text(),
            None => self.problems.iter().map(Problem::text).collect::<Vec<_>>().join("\n"),
        }
    }

    pub fn write(&mut self, text: &str) {
        self.output.write(text);
    }

    pub fn add_build_line(&mut self, text: &str) {
        self.build.write(text);
        self.build.write("\n");
    }

    pub fn add_problem(&mut self, p: Problem) {
        self.problems.push(p);
        self.problems_rev += 1;
        while self.problems.len() > self.max_lines {
            self.problems.remove(0);
        }
    }

    /// Empties page `p`; its view starts over. (The Problems page is filled
    /// again as the code is checked: it stays where the reader scrolled it —
    /// at its top until they scroll —, never following to the end.)
    pub fn clear(&mut self, p: Page) {
        match self.screen_mut(p) {
            Some(s) => s.clear(),
            None => {
                self.problems.clear();
                self.problems_rev += 1;
            }
        }
        let first = self.first(p);
        if p == Page::Problems {
            let was = self.views[p.index()];
            self.views[p.index()] = View { follow: false, top: was.top, sub: was.sub, ..View::default() };
        } else {
            self.views[p.index()] = View { top: first, ..View::default() };
        }
        if p == self.page {
            self.hover_link = None;
            self.hover_row = None;
            self.pressed_line = None;
        }
    }

    pub fn set_max_lines(&mut self, n: i64) {
        self.max_lines = n.max(1) as usize;
        self.output.set_max(self.max_lines);
        self.build.set_max(self.max_lines);
        while self.problems.len() > self.max_lines {
            self.problems.remove(0);
            self.problems_rev += 1;
        }
    }

    pub fn set_auto_scroll(&mut self, on: bool) {
        self.auto_scroll = on;
        for (k, v) in self.views.iter_mut().enumerate() {
            v.follow = on && k != Page::Problems.index();
        }
    }

    /// Shows page `p` (the search is the new page's).
    pub fn show(&mut self, p: Page) {
        if p != self.page {
            self.page = p;
            self.current = None;
            self.hover_link = None;
            self.hover_row = None;
            self.hover_tab = None;
            self.pressed_line = None;
            if !self.filter.is_empty() {
                self.find_first();
            }
        }
    }

    /// The shown page's matches for the filter (made again when the text
    /// or the filter changed).
    pub fn matches(&mut self) -> &[Match] {
        let rev = self.screen(self.page).map_or(self.problems_rev, |s| s.rev);
        let key = (self.page, rev, self.first(self.page), self.filter.clone());
        if self.found_for.as_ref() != Some(&key) {
            self.found = self.search();
            if self.current.is_some_and(|c| c >= self.found.len()) {
                self.current = (!self.found.is_empty()).then_some(self.found.len() - 1);
            }
            self.found_for = Some(key);
        }
        &self.found
    }

    fn search(&self) -> Vec<Match> {
        let needle: Vec<char> = self.filter.chars().map(lower).collect();
        if needle.is_empty() {
            return Vec::new();
        }
        let first = self.first(self.page);
        let mut out = Vec::new();
        let n = self.count(self.page);
        for i in 0..n {
            let Some(text) = self.line_text(self.page, i) else { continue };
            // (each character's lower case, one for one: the columns stay)
            let hay: Vec<char> = text.chars().map(lower).collect();
            find_all(&hay, &needle, first + i as u64, &mut out);
        }
        out
    }

    /// Sets the filter and goes to the first match: how many there are.
    pub fn find(&mut self, text: &str) -> usize {
        self.filter = text.to_string();
        self.current = None;
        self.find_first()
    }

    fn find_first(&mut self) -> usize {
        let n = self.matches().len();
        if n > 0 {
            self.current = Some(0);
            self.reveal_current();
        } else {
            self.current = None;
        }
        n
    }

    /// The next (or previous) match: its line (from 0 on the page), `None`
    /// when there is none.
    pub fn find_next(&mut self, back: bool) -> Option<usize> {
        let n = self.matches().len();
        if n == 0 {
            self.current = None;
            return None;
        }
        self.current = Some(match (self.current, back) {
            (None, false) => 0,
            (None, true) => n - 1,
            (Some(c), false) => (c + 1) % n,
            (Some(c), true) => (c + n - 1) % n,
        });
        self.reveal_current();
        let m = self.found[self.current?];
        Some((m.line - self.first(self.page)) as usize)
    }

    /// The current match's line is selected and scrolled to (the view
    /// stops following).
    fn reveal_current(&mut self) {
        let Some(m) = self.current.and_then(|c| self.found.get(c).copied()) else { return };
        let v = self.view_mut();
        v.follow = false;
        v.anchor = Some(m.line);
        v.caret = Some(m.line);
        v.cols = None;
    }

    /// "3 of 12" (the strip's), "No results", or "" without a filter.
    pub fn found_text(&mut self) -> String {
        if self.filter.is_empty() {
            return String::new();
        }
        let n = self.matches().len();
        match (n, self.current) {
            (0, _) => "No results".into(),
            (n, Some(c)) => format!("{} of {n}", c + 1),
            (n, None) => format!("{n} found"),
        }
    }

    /// The search cleared (Escape).
    pub fn clear_filter(&mut self) {
        self.filter.clear();
        self.current = None;
    }

    /// What's selected as plain text (Ctrl+C; no colours, no escape
    /// sequences), a line to a line — a wrapped line is one: the
    /// characters a drag or a double click selected, else the whole lines a
    /// click or the keys selected (a problem's row as `file:line:col:
    /// severity: message`). `None`: nothing (an empty text selection).
    pub fn selected_text(&self) -> Option<String> {
        let v = self.view();
        let (a, b) = v.selection()?;
        let first = self.first(self.page);
        let text_of = |l: u64| l.checked_sub(first).and_then(|i| self.line_text(self.page, i as usize));
        if let Some(((la, ca), (lb, cb))) = v.text_range() {
            if (la, ca) == (lb, cb) {
                return None;
            }
            let mut out = Vec::new();
            for l in la..=lb {
                // (a line gone past MaxLines: nothing of it)
                let Some(text) = text_of(l) else { continue };
                let chars: Vec<char> = text.chars().collect();
                let to = if l == lb { cb.min(chars.len()) } else { chars.len() };
                let from = if l == la { ca.min(to) } else { 0 };
                out.push(chars[from..to].iter().collect::<String>());
            }
            return (!out.is_empty()).then(|| out.join("\n"));
        }
        let lines: Vec<String> = (a..=b).filter_map(text_of).collect();
        (!lines.is_empty()).then(|| lines.join("\n"))
    }

    /// Whether something is selected (Copy has something to copy).
    pub fn has_selection(&self) -> bool {
        let v = self.view();
        match v.text_range() {
            Some((a, b)) => a != b && self.count(self.page) > 0,
            None => v.selection().is_some() && self.count(self.page) > 0,
        }
    }

    /// Every line (every problem) of the shown page selected (Ctrl+A).
    pub fn select_all(&mut self) {
        let (first, count) = (self.first(self.page), self.count(self.page));
        let v = self.view_mut();
        v.cols = None;
        if count == 0 {
            v.anchor = None;
            v.caret = None;
        } else {
            v.anchor = Some(first);
            v.caret = Some(first + count as u64 - 1);
        }
    }

    /// The word at character `at` of absolute line `line` selected (a
    /// double click): [`word_at`].
    pub fn select_word(&mut self, line: u64, at: usize) {
        let Some(i) = line.checked_sub(self.first(self.page)) else { return };
        let chars: Vec<char> = self.line_text(self.page, i as usize).unwrap_or_default().chars().collect();
        let (a, b) = word_at(&chars, at);
        let v = self.view_mut();
        v.anchor = Some(line);
        v.caret = Some(line);
        v.cols = Some((a, b));
        v.follow = false;
    }

    /// What Copy Line copies: line `index` of the shown page (from 0), or
    /// without one the line the mouse last pressed on, else the caret's.
    pub fn copy_line_text(&self, index: Option<usize>) -> Option<String> {
        let i = match index {
            Some(i) => i,
            None => self.pressed_line.or(self.view().caret)?.checked_sub(self.first(self.page))? as usize,
        };
        self.line_text(self.page, i)
    }

    /// The line the mouse last pressed on, from 0 on the shown page.
    pub fn pressed_index(&self) -> Option<usize> {
        self.pressed_line?.checked_sub(self.first(self.page)).map(|i| i as usize).filter(|&i| i < self.count(self.page))
    }

    /// The links on page `p`'s line `i` (from 0): its hyperlinks and the
    /// places its text names (none on the Problems page: its rows are).
    pub fn links_of(&self, p: Page, i: usize) -> Vec<links::Link> {
        self.screen(p).and_then(|s| s.line(i)).map(links::of_line).unwrap_or_default()
    }

    /// The link at absolute line `line`, character `at` of the shown page.
    pub fn link_at(&self, line: u64, at: usize) -> Option<(links::Link, Match)> {
        let i = line.checked_sub(self.first(self.page))? as usize;
        self.links_of(self.page, i).into_iter().find(|l| (l.start..l.end).contains(&at)).map(|l| {
            let m = Match { line, start: l.start, end: l.end };
            (l, m)
        })
    }
}

/// The word around character `at` of `chars`, as (start, end): letters,
/// digits and `_` together, else the run of spaces there, else the one
/// other character (`at` past the end: the last character's).
pub fn word_at(chars: &[char], at: usize) -> (usize, usize) {
    if chars.is_empty() {
        return (0, 0);
    }
    let at = at.min(chars.len() - 1);
    let class = |c: char| {
        if c.is_alphanumeric() || c == '_' {
            0
        } else if c.is_whitespace() {
            1
        } else {
            2
        }
    };
    let k = class(chars[at]);
    if k == 2 {
        return (at, at + 1);
    }
    let mut a = at;
    while a > 0 && class(chars[a - 1]) == k {
        a -= 1;
    }
    let mut b = at + 1;
    while b < chars.len() && class(chars[b]) == k {
        b += 1;
    }
    (a, b)
}

/// A character's lower case (its first, when it has more).
fn lower(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

fn find_all(hay: &[char], needle: &[char], line: u64, out: &mut Vec<Match>) {
    if needle.len() > hay.len() {
        return;
    }
    let mut i = 0;
    while i + needle.len() <= hay.len() {
        if hay[i..i + needle.len()] == *needle {
            out.push(Match { line, start: i, end: i + needle.len() });
            i += needle.len();
        } else {
            i += 1;
        }
    }
}

/// What the user did to it (from the kernel).
#[derive(Clone, Debug, PartialEq)]
pub enum User {
    /// A page's tab clicked: it shows, OnPageChange.
    Page(Page),
    /// A `file:line` place or a problem clicked (or Enter on it):
    /// OnLinkClick (File, Line).
    Link(String, i64),
}

/// A page argument (`None`: the shown one).
fn page_arg(args: &[Value], k: usize, shown: Page) -> Page {
    match args.get(k) {
        None | Some(Value::Null) => shown,
        Some(Value::Integer(i)) => Page::ALL.get(*i as usize).copied().unwrap_or(shown),
        Some(v) => Page::parse(&v.to_string_val()).unwrap_or(shown),
    }
}

/// Its methods; `None`: not one of its own.
pub fn rt_method<R: Runtime>(rt: R, name: &str, method: &str, args: &[Value]) -> Option<Value> {
    let s = |k: usize| args.get(k).map(Value::to_string_val).unwrap_or_default();
    let n = |k: usize| args.get(k).map_or(0, Value::to_i64);
    Some(match method {
        "write" => {
            with_mut(name, |c| c.write(&s(0)));
            Value::Null
        }
        "writeline" => {
            with_mut(name, |c| {
                c.write(&s(0));
                c.write("\n");
            });
            Value::Null
        }
        "clear" => {
            with_mut(name, |c| {
                let p = page_arg(args, 0, c.page);
                c.clear(p);
            });
            Value::Null
        }
        "addbuildline" => {
            with_mut(name, |c| c.add_build_line(&s(0)));
            Value::Null
        }
        "addproblem" => {
            let sev = s(3);
            let kind = sev.trim();
            let heading = kind.eq_ignore_ascii_case("file") || kind.get(..5).is_some_and(|k| k.eq_ignore_ascii_case("file:"));
            let icon = if heading { kind.get(5..).unwrap_or("").trim().to_string() } else { String::new() };
            let p = Problem { file: s(0), line: n(1), col: n(2), severity: if heading { Severity::Info } else { Severity::parse(&sev) }, message: s(4), heading, icon };
            with_mut(name, |c| c.add_problem(p));
            Value::Null
        }
        "clearproblems" => {
            with_mut(name, |c| c.clear(Page::Problems));
            Value::Null
        }
        "line" => Value::String(with(name, |c| c.line_text(c.page, usize::try_from(n(0)).unwrap_or(usize::MAX))).flatten().unwrap_or_default()),
        "find" => Value::Integer(with_mut(name, |c| c.find(&s(0))) as i64),
        "findnext" => Value::Integer(with_mut(name, |c| c.find_next(false)).map_or(-1, |l| l as i64)),
        // (RapidR's: the selection and the clipboard — what the user's
        // Ctrl+C, a context menu's Copy, Copy Line, Copy All do)
        "selectall" => {
            with_mut(name, |c| c.select_all());
            rt.invalidate();
            Value::Null
        }
        "copyselection" => copied(rt, with(name, |c| c.selected_text()).flatten()),
        "copyall" => copied(rt, with(name, |c| Some(c.text(c.page)).filter(|t| c.count(c.page) > 0 && !t.is_empty())).flatten()),
        "copyline" => {
            let index = match args.first() {
                None | Some(Value::Null) => None,
                Some(v) => Some(usize::try_from(v.to_i64()).unwrap_or(usize::MAX)),
            };
            copied(rt, with(name, |c| c.copy_line_text(index)).flatten())
        }
        _ => return None,
    })
}

/// `text` put on the clipboard: True when there was some.
fn copied<R: Runtime>(rt: R, text: Option<String>) -> Value {
    match text {
        Some(t) => {
            rt.set_clipboard(&t);
            basic_bool(true)
        }
        None => basic_bool(false),
    }
}

/// Its properties its model answers.
pub fn rt_get<R: Runtime>(_rt: R, name: &str, prop: &str) -> Option<Value> {
    with_mut(name, |c| {
        Some(match prop {
            "page" => Value::String(c.page.name().into()),
            "maxlines" => Value::Integer(c.max_lines as i64),
            "filter" => Value::String(c.filter.clone()),
            "showtabs" => basic_bool(c.show_tabs),
            "autoscroll" => basic_bool(c.auto_scroll),
            "wordwrap" => basic_bool(c.word_wrap),
            "linecount" => Value::Integer(c.count(c.page) as i64),
            "problemcount" => Value::Integer(c.problems.iter().filter(|p| !p.heading).count() as i64),
            // (the first line — the Problems page's row — the page shows, from 0)
            "topline" => Value::Integer(c.view().top.saturating_sub(c.first(c.page)) as i64),
            "text" => Value::String(c.text(c.page)),
            "seltext" => Value::String(c.selected_text().unwrap_or_default()),
            "hasselection" => basic_bool(c.has_selection()),
            "clickedline" => Value::Integer(c.pressed_index().map_or(-1, |i| i as i64)),
            _ => return None,
        })
    })
}

/// Its properties its model keeps: whether `prop` was one.
pub fn rt_set<R: Runtime>(_rt: R, name: &str, prop: &str, v: &Value) -> bool {
    match prop {
        "page" => {
            let shown = with(name, |c| c.page).unwrap_or_default();
            let p = page_arg(std::slice::from_ref(v), 0, shown);
            with_mut(name, |c| c.show(p));
        }
        "maxlines" => with_mut(name, |c| c.set_max_lines(v.to_i64())),
        "filter" => {
            let f = v.to_string_val();
            with_mut(name, |c| {
                c.find(&f);
            });
        }
        "showtabs" => with_mut(name, |c| c.show_tabs = truth(v)),
        "autoscroll" => with_mut(name, |c| c.set_auto_scroll(truth(v))),
        "wordwrap" => with_mut(name, |c| c.word_wrap = truth(v)),
        _ => return false,
    }
    true
}

/// What the user did.
pub fn rt_user<R: Runtime>(rt: R, name: &str, action: User) {
    match action {
        User::Page(p) => {
            let changed = with_mut(name, |c| {
                let was = c.page;
                c.show(p);
                was != p
            });
            if changed {
                rt.fire(name, "onpagechange", &[Value::String(p.name().into())]);
            }
        }
        User::Link(file, line) => rt.fire(name, "onlinkclick", &[Value::String(file), Value::Integer(line)]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_lines_and_problems() {
        let mut c = Console::default();
        c.write("Hello\n\x1b[31mred\x1b[0m");
        c.add_build_line("Compiling Main.rr");
        c.add_build_line("Main.rr:3:5: error: x");
        c.add_problem(Problem { file: "Main.rr".into(), line: 12, col: 5, severity: Severity::Error, message: "Undefined variable X".into(), heading: false, icon: String::new() });
        assert_eq!(c.count(Page::Output), 2);
        assert_eq!(c.text(Page::Output), "Hello\nred");
        assert_eq!(c.line_text(Page::Build, 1).as_deref(), Some("Main.rr:3:5: error: x"));
        assert_eq!(c.text(Page::Problems), "Main.rr:12:5: error: Undefined variable X");
        assert_eq!(c.problems[0].spoken(), "Error: Undefined variable X, Main.rr line 12");
        c.clear(Page::Build);
        assert_eq!(c.count(Page::Build), 0);
        assert_eq!(Page::parse("PROBLEMS"), Some(Page::Problems));
        assert_eq!(Page::parse("1"), Some(Page::Build));
    }

    #[test]
    fn search_counts_and_steps() {
        let mut c = Console::default();
        c.write("alpha beta\nbeta gamma beta\ndelta\n");
        assert_eq!(c.find("BETA"), 3);
        assert_eq!(c.found_text(), "1 of 3");
        assert_eq!(c.view().selection(), Some((0, 0)));
        assert!(!c.view().follow, "a match shown stops following");
        assert_eq!(c.find_next(false), Some(1));
        assert_eq!(c.find_next(false), Some(1));
        assert_eq!(c.found_text(), "3 of 3");
        assert_eq!(c.find_next(false), Some(0), "wraps");
        assert_eq!(c.find_next(true), Some(1), "Shift+F3: back");
        assert_eq!(c.find("zeta"), 0);
        assert_eq!(c.found_text(), "No results");
        assert_eq!(c.find_next(false), None);
        c.clear_filter();
        assert_eq!(c.found_text(), "");
        // (the matches follow the text)
        c.find("delta");
        c.write("delta again\n");
        assert_eq!(c.matches().len(), 2);
    }

    #[test]
    fn selection_copies_lines() {
        let mut c = Console::default();
        c.write("one\ntwo\nthree\n");
        let v = c.view_mut();
        v.anchor = Some(2);
        v.caret = Some(1);
        assert_eq!(c.selected_text().as_deref(), Some("two\nthree"));
    }

    /// (COPY-ANYWHERE) A text selection's characters, across lines, either
    /// way; the colours' escape sequences never in it; an empty one is
    /// nothing to copy.
    #[test]
    fn a_text_selection_copies_its_characters() {
        let mut c = Console::default();
        c.write("one two\n\x1b[32mthree\x1b[0m four\nfive\n");
        let v = c.view_mut();
        v.anchor = Some(0);
        v.caret = Some(0);
        v.cols = Some((4, 7));
        assert_eq!(c.selected_text().as_deref(), Some("two"));
        let v = c.view_mut();
        v.anchor = Some(2);
        v.caret = Some(0);
        v.cols = Some((2, 4));
        assert_eq!(c.selected_text().as_deref(), Some("two\nthree four\nfi"), "backwards, the middle line whole, no escapes");
        assert!(c.has_selection());
        let v = c.view_mut();
        v.cols = Some((3, 3));
        v.anchor = Some(1);
        v.caret = Some(1);
        assert_eq!(c.selected_text(), None);
        assert!(!c.has_selection());
        // (from a line's end to the next's start: its line break)
        let v = c.view_mut();
        v.anchor = Some(0);
        v.caret = Some(1);
        v.cols = Some((7, 0));
        assert_eq!(c.selected_text().as_deref(), Some("\n"));
        // the whole lines (a click), Select All
        let v = c.view_mut();
        v.cols = None;
        v.anchor = Some(1);
        v.caret = Some(1);
        assert_eq!(c.selected_text().as_deref(), Some("three four"));
        c.select_all();
        assert_eq!(c.selected_text().as_deref(), Some("one two\nthree four\nfive"));
        // a double click's word
        c.select_word(1, 7);
        assert_eq!(c.selected_text().as_deref(), Some("four"));
        assert_eq!(word_at(&"a.rr:12 x".chars().collect::<Vec<_>>(), 1), (1, 2), "punctuation: itself");
        assert_eq!(word_at(&"my_var  = 1".chars().collect::<Vec<_>>(), 3), (0, 6));
        assert_eq!(word_at(&"my_var  = 1".chars().collect::<Vec<_>>(), 6), (6, 8), "spaces: their run");
        assert_eq!(word_at(&"abc".chars().collect::<Vec<_>>(), 99), (0, 3), "past the end: the last word");
    }

    /// The Problems page copies its rows as `file:line:col: severity:
    /// message`; Copy Line copies the row pressed on.
    #[test]
    fn problems_copy_as_lines_and_copy_line_takes_the_pressed_one() {
        let mut c = Console::default();
        c.add_problem(Problem { file: "main.rr".into(), line: 4, col: 7, severity: Severity::Error, message: "Undefined variable X".into(), heading: false, icon: String::new() });
        c.add_problem(Problem { file: "main.rr".into(), line: 0, col: 0, severity: Severity::Warning, message: "w".into(), heading: false, icon: String::new() });
        c.show(Page::Problems);
        c.select_all();
        assert_eq!(c.selected_text().as_deref(), Some("main.rr:4:7: error: Undefined variable X\nmain.rr: warning: w"));
        c.pressed_line = Some(1);
        assert_eq!(c.copy_line_text(None).as_deref(), Some("main.rr: warning: w"));
        assert_eq!(c.copy_line_text(Some(0)).as_deref(), Some("main.rr:4:7: error: Undefined variable X"));
        assert_eq!(c.pressed_index(), Some(1));
        c.show(Page::Output);
        assert_eq!(c.pressed_index(), None, "another page: no line pressed there");
    }

    /// The members through a runtime: SelText, HasSelection, ClickedLine,
    /// SelectAll, CopySelection, CopyAll and CopyLine (the clipboard the
    /// runtime's).
    #[test]
    fn copy_members_put_text_on_the_runtimes_clipboard() {
        thread_local! { static CLIP: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
        #[derive(Clone, Copy)]
        struct Fake;
        impl Runtime for Fake {
            fn get(self, _: &str, _: &str) -> Value {
                Value::Null
            }
            fn set(self, _: &str, _: &str, _: Value) {}
            fn call(self, _: &str, _: &str, _: &[Value]) -> Value {
                Value::Null
            }
            fn exists(self, _: &str) -> bool {
                true
            }
            fn type_of(self, _: &str) -> String {
                "ROUTPUTCONSOLE".into()
            }
            fn components(self) -> Vec<(String, String)> {
                Vec::new()
            }
            fn children(self, _: &str) -> Vec<(String, String)> {
                Vec::new()
            }
            fn form_of(self, _: &str) -> Option<String> {
                None
            }
            fn fire(self, _: &str, _: &str, _: &[Value]) {}
            fn fire_then(self, _: &str, _: &str, _: &[Value], _: super::super::runtime::Then) {}
            fn focus(self, _: &str) {}
            fn invalidate(self) {}
            fn restructure(self) {}
            fn drop_list(self, _: &str, _: &str, _: Vec<String>, _: crate::objects::ops::Rect) {}
            fn set_clipboard(self, text: &str) {
                CLIP.with(|c| *c.borrow_mut() = Some(text.to_string()));
            }
        }
        let name = "con_members_test";
        remove(name);
        with_mut(name, |c| c.write("first\nsecond line\nthird\n"));
        let get = |p: &str| rt_get(Fake, name, p).unwrap();
        let call = |m: &str, a: &[Value]| rt_method(Fake, name, m, a).unwrap();
        let clip = || CLIP.with(|c| c.borrow().clone());
        assert_eq!((get("seltext").to_string_val(), get("hasselection").to_i64(), get("clickedline").to_i64()), (String::new(), 0, -1));
        assert_eq!(call("copyselection", &[]).to_i64(), 0, "nothing selected: nothing copied");
        assert_eq!(clip(), None);
        with_mut(name, |c| {
            let v = c.view_mut();
            v.anchor = Some(1);
            v.caret = Some(1);
            v.cols = Some((0, 6));
            c.pressed_line = Some(2);
        });
        assert_eq!((get("seltext").to_string_val(), get("hasselection").to_i64(), get("clickedline").to_i64()), ("second".into(), 1, 2));
        assert_eq!(call("copyselection", &[]).to_i64(), 1);
        assert_eq!(clip().as_deref(), Some("second"));
        assert_eq!(call("copyline", &[]).to_i64(), 1);
        assert_eq!(clip().as_deref(), Some("third"));
        assert_eq!(call("copyline", &[Value::Integer(0)]).to_i64(), 1);
        assert_eq!(clip().as_deref(), Some("first"));
        assert_eq!(call("copyline", &[Value::Integer(9)]).to_i64(), 0, "no such line");
        assert_eq!(call("copyall", &[]).to_i64(), 1);
        assert_eq!(clip().as_deref(), Some("first\nsecond line\nthird"));
        call("selectall", &[]);
        assert_eq!(get("seltext").to_string_val(), "first\nsecond line\nthird");
        // (an empty page: nothing to copy)
        rt_method(Fake, name, "clear", &[]);
        assert_eq!(call("copyall", &[]).to_i64(), 0);
        remove(name);
    }

    #[test]
    fn links_in_the_shown_page() {
        let mut c = Console::default();
        c.write("x\nsee Main.rr:12 now\n");
        let (l, m) = c.link_at(1, 6).unwrap();
        assert_eq!((l.file.as_str(), l.line), ("Main.rr", 12));
        assert_eq!((m.start, m.end), (4, 14));
        assert!(c.link_at(1, 1).is_none());
        // (a hyperlink on the build page — Studio's "Reveal in Finder" —
        // and a place after it on the same line)
        c.add_build_line("Built. \x1b]8;;reveal:/x/Notes.app\x1b\\Reveal in Finder\x1b]8;;\x1b\\ or main.rr:3");
        c.show(Page::Build);
        let (l, m) = c.link_at(c.first(Page::Build), 9).unwrap();
        assert_eq!((l.file.as_str(), l.line, m.start, m.end), ("reveal:/x/Notes.app", 0, 7, 23));
        let links = c.links_of(Page::Build, 0);
        assert_eq!(links.iter().map(|l| l.file.as_str()).collect::<Vec<_>>(), ["reveal:/x/Notes.app", "main.rr"]);
    }

    #[test]
    fn max_lines_trims_every_page() {
        let mut c = Console::default();
        c.set_max_lines(2);
        c.write("a\nb\nc\n");
        assert_eq!(c.text(Page::Output), "b\nc");
        for i in 0..3 {
            c.add_problem(Problem { message: i.to_string(), ..Problem::default() });
        }
        assert_eq!(c.problems.len(), 2);
    }
}
