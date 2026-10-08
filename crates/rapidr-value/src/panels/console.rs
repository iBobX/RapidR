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

    /// Its line of text: `Main.rr:12:5: error: message`.
    pub fn text(&self) -> String {
        if self.file.is_empty() {
            format!("{}: {}", self.severity.name(), self.message)
        } else {
            format!("{}: {}: {}", self.place(), self.severity.name(), self.message)
        }
    }

    /// What a screen reader is told: "Error: message, Main.rr line 12".
    pub fn spoken(&self) -> String {
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
    /// The selection: the line it started at and the line it reaches (the
    /// caret). A Problems page's focused row is `caret`.
    pub anchor: Option<u64>,
    pub caret: Option<u64>,
}

impl Default for View {
    fn default() -> Self {
        View { follow: true, top: 0, anchor: None, caret: None }
    }
}

impl View {
    /// The selected lines, first to last (absolute).
    pub fn selection(&self) -> Option<(u64, u64)> {
        let (a, c) = (self.anchor?, self.caret?);
        Some((a.min(c), a.max(c)))
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
            max_lines: 5000,
            filter: String::new(),
            views: [View::default(); 3],
            current: None,
            hover_link: None,
            hover_tab: None,
            hover_row: None,
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

    /// Empties page `p`; its view starts over.
    pub fn clear(&mut self, p: Page) {
        match self.screen_mut(p) {
            Some(s) => s.clear(),
            None => {
                self.problems.clear();
                self.problems_rev += 1;
            }
        }
        let first = self.first(p);
        self.views[p.index()] = View { top: first, ..View::default() };
        if p == self.page {
            self.hover_link = None;
            self.hover_row = None;
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
        for v in &mut self.views {
            v.follow = on;
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

    /// The selected lines' text (Ctrl+C), one to a line.
    pub fn selected_text(&self) -> Option<String> {
        let (a, b) = self.view().selection()?;
        let first = self.first(self.page);
        let lines: Vec<String> = (a..=b).filter_map(|l| l.checked_sub(first)).filter_map(|i| self.line_text(self.page, i as usize)).collect();
        (!lines.is_empty()).then(|| lines.join("\n"))
    }

    /// The link at absolute line `line`, character `at` of the shown page.
    pub fn link_at(&self, line: u64, at: usize) -> Option<(links::Link, Match)> {
        let i = line.checked_sub(self.first(self.page))? as usize;
        let text = self.line_text(self.page, i)?;
        links::find(&text).into_iter().find(|l| (l.start..l.end).contains(&at)).map(|l| {
            let m = Match { line, start: l.start, end: l.end };
            (l, m)
        })
    }
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
pub fn rt_method<R: Runtime>(_rt: R, name: &str, method: &str, args: &[Value]) -> Option<Value> {
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
            let p = Problem { file: s(0), line: n(1), col: n(2), severity: Severity::parse(&s(3)), message: s(4) };
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
        _ => return None,
    })
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
            "linecount" => Value::Integer(c.count(c.page) as i64),
            "problemcount" => Value::Integer(c.problems.len() as i64),
            "text" => Value::String(c.text(c.page)),
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
        c.add_problem(Problem { file: "Main.rr".into(), line: 12, col: 5, severity: Severity::Error, message: "Undefined variable X".into() });
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

    #[test]
    fn links_in_the_shown_page() {
        let mut c = Console::default();
        c.write("x\nsee Main.rr:12 now\n");
        let (l, m) = c.link_at(1, 6).unwrap();
        assert_eq!((l.file.as_str(), l.line), ("Main.rr", 12));
        assert_eq!((m.start, m.end), (4, 14));
        assert!(c.link_at(1, 1).is_none());
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
