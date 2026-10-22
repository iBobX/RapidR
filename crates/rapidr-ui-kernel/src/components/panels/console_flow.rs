//! How ROUTPUTCONSOLE's shown page is laid out in rows at a width (its
//! WordWrap): each line's top and height in pixels, and where a line that
//! doesn't fit breaks onto more rows — at a space where there is one near
//! the edge, else wherever the edge falls — so a long message reads whole.
//!
//! Without wrapping every line is one row (the Output and Build pages'
//! monospaced lines, the Problems page's fixed rows), cut at the edge as
//! before. With it, a line of Output is as many rows as it needs; a problem
//! is its message wrapped to the page's width, its place after the last
//! row (or under it when it doesn't fit there), the row as tall as that
//! text.
//!
//! A [`Flow`] is made for (page, text revision, width, fonts, wrap) and kept
//! until one of them changes; what was worked out for a line is kept while
//! its text is the same, so streaming output costs only its new lines.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use rapidr_value::objects::font::Font;
use rapidr_value::objects::text::text_size;
use rapidr_value::panels::console::{Page, Problem};
use rapidr_value::panels::rows;

/// A line's text left of its edge (the text's margin) and right of it.
pub const MARGIN: i64 = 6;
const RIGHT: i64 = 4;
/// The Problems page's text: left of it the marker icon, right of it a margin.
pub const PROBLEM_LEFT: i64 = 30;
const PROBLEM_RIGHT: i64 = 8;
/// The gap between a problem's message and its place.
pub const PLACE_GAP: i64 = 12;
/// A wrapped problem's room above and below its text rows together.
const PROBLEM_PAD: i64 = 8;

/// One page's rows at a width.
#[derive(Debug)]
pub struct Flow {
    pub page: Page,
    pub wrap: bool,
    /// The width the rows were broken for (the page's, a scroll bar's width
    /// taken off).
    pub width: i64,
    /// Each line's top, in pixels from the page's first: `tops[i]` .. `tops[i + 1]`
    /// is line `i`'s height (one more entry than there are lines).
    tops: Vec<i64>,
    /// A line (a problem's message) that takes more than a row: where its
    /// rows after the first start (character indexes).
    breaks: HashMap<usize, Rc<Broken>>,
    /// The height of a row of text: the output's line, the problems' text line.
    pub text_h: i64,
}

/// How a line breaks.
#[derive(Debug, PartialEq, Eq)]
pub struct Broken {
    /// The characters its rows after the first start at.
    pub starts: Vec<usize>,
    /// (a problem's) its place doesn't fit after the message's last row: it
    /// has a row of its own.
    pub place_below: bool,
}

impl Flow {
    /// How many lines (problems) the page has.
    pub fn count(&self) -> usize {
        self.tops.len() - 1
    }

    /// Everything's height in pixels.
    pub fn total(&self) -> i64 {
        *self.tops.last().unwrap_or(&0)
    }

    /// Line `i`'s top (the end of the page past the last).
    pub fn top(&self, i: usize) -> i64 {
        self.tops[i.min(self.count())]
    }

    pub fn height(&self, i: usize) -> i64 {
        self.top(i + 1) - self.top(i)
    }

    /// The line a pixel row `y` (from the page's top) is in: the last one
    /// when `y` is past the end; `None` without lines.
    pub fn line_at(&self, y: i64) -> Option<usize> {
        let n = self.count();
        if n == 0 {
            return None;
        }
        Some(self.tops.partition_point(|&t| t <= y).saturating_sub(1).min(n - 1))
    }

    /// The line at pixel `y` and how far into it `y` is.
    pub fn locate(&self, y: i64) -> (usize, i64) {
        match self.line_at(y.max(0)) {
            Some(i) => (i, (y.max(0) - self.top(i)).clamp(0, self.height(i).max(1) - 1)),
            None => (0, 0),
        }
    }

    /// The pixel row `sub` pixels into line `i`.
    pub fn pos_of(&self, i: usize, sub: i64) -> i64 {
        (self.top(i) + sub).clamp(0, self.total())
    }

    /// Where line `i` breaks (`None`: it is one row).
    pub fn broken(&self, i: usize) -> Option<&Broken> {
        self.breaks.get(&i).map(Rc::as_ref)
    }

    /// Line `i`'s rows, each as a range of its characters (`len` of them).
    pub fn rows_of(&self, i: usize, len: usize) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        let mut start = 0;
        if let Some(b) = self.breaks.get(&i) {
            for &s in &b.starts {
                out.push((start, s.min(len)));
                start = s.min(len);
            }
        }
        out.push((start, len));
        out
    }

    /// How many lines, from line `from` on, fit in `height` pixels (at least one).
    pub fn lines_in(&self, from: usize, height: i64) -> usize {
        let from = from.min(self.count());
        let base = self.top(from);
        (from..self.count()).take_while(|&i| self.top(i + 1) - base <= height).count().max(1)
    }
}

/// What the flow is made from (a console's page and its fonts).
pub struct Params<'a> {
    pub id: &'a str,
    pub page: Page,
    /// The width of the page's rows (beside a scroll bar).
    pub width: i64,
    pub mono: &'a Font,
    pub chrome: &'a Font,
    /// The output's line height.
    pub lh: i64,
    pub wrap: bool,
}

/// Key: what a flow is for; its parts that change as the text does come last.
#[derive(Clone, PartialEq, Eq)]
struct Key {
    page: Page,
    width: i64,
    fonts: u64,
    lh: i64,
    wrap: bool,
}

struct Cached {
    key: Key,
    /// The text's revision, first line and line count the flow was made for.
    rev: (u64, u64, usize),
    flow: Rc<Flow>,
    /// What was worked out for each line's text (by its hash), for the
    /// flows made after the text changed.
    lines: HashMap<u64, Rc<Broken>>,
}

thread_local! {
    static CACHE: RefCell<HashMap<String, Cached>> = RefCell::new(HashMap::new());
}

fn font_sig(fonts: &[&Font]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for f in fonts {
        (&f.name, f.size, f.styles).hash(&mut h);
    }
    h.finish()
}

fn hash_of(parts: &[&str]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for p in parts {
        p.hash(&mut h);
        0u8.hash(&mut h);
    }
    h.finish()
}

/// The flow last made for console `id` (`None`: none yet) — what the last
/// paint laid out, for the mouse and the scroll bar to act on.
pub fn last(id: &str) -> Option<Rc<Flow>> {
    CACHE.with(|c| c.borrow().get(&id.to_lowercase()).map(|c| c.flow.clone()))
}

/// Forgets console `id`'s flow (it's gone).
pub fn forget(id: &str) {
    CACHE.with(|c| c.borrow_mut().remove(&id.to_lowercase()));
}

/// The flow for `p`, made again when the page's text, width, fonts or
/// wrapping changed since the last one.
pub fn flow(p: &Params) -> Rc<Flow> {
    let key = Key { page: p.page, width: p.width, fonts: font_sig(&[p.mono, p.chrome]), lh: p.lh, wrap: p.wrap };
    let rev = rapidr_value::panels::console::with(p.id, |c| (c.rev(p.page), c.first(p.page), c.count(p.page))).unwrap_or((0, 0, 0));
    let id = p.id.to_lowercase();
    let reuse = CACHE.with(|c| c.borrow().get(&id).filter(|c| c.key == key && c.rev == rev).map(|c| c.flow.clone()));
    if let Some(f) = reuse {
        return f;
    }
    let mut lines = CACHE.with(|c| match c.borrow_mut().remove(&id) {
        Some(old) if old.key == key => old.lines,
        _ => HashMap::new(),
    });
    // (a page that grew since: the lines' results bounded, as the console's lines are)
    if lines.len() > 20_000 {
        lines.clear();
    }
    let flow = Rc::new(make(p, &mut lines));
    CACHE.with(|c| {
        c.borrow_mut().insert(id, Cached { key, rev, flow: flow.clone(), lines });
    });
    flow
}

fn make(p: &Params, memo: &mut HashMap<u64, Rc<Broken>>) -> Flow {
    let problems = p.page == Page::Problems;
    let text_h = if problems { text_size("Ag", p.chrome).1.max(1) } else { p.lh };
    let one = if problems { rows::ROW } else { p.lh };
    // (the text's room in a row)
    let room = if problems { p.width - PROBLEM_LEFT - PROBLEM_RIGHT } else { p.width - MARGIN - RIGHT }.max(1);
    let mut tops = vec![0];
    let mut breaks = HashMap::new();
    let mut y = 0;
    let rows_of_page: Vec<(String, Option<Problem>)> = rapidr_value::panels::console::with(p.id, |c| match p.page {
        Page::Problems => c.problems.iter().map(|pr| (pr.message.clone(), Some(pr.clone()))).collect(),
        page => (0..c.count(page)).map(|i| (c.line_text(page, i).unwrap_or_default(), None)).collect(),
    })
    .unwrap_or_default();
    for (i, (text, problem)) in rows_of_page.iter().enumerate() {
        let mut h = one;
        if p.wrap {
            let place = problem.as_ref().map(Problem::place).unwrap_or_default();
            let b = memo.entry(hash_of(&[text, &place])).or_insert_with(|| Rc::new(break_line(text, &place, room, if problems { p.chrome } else { p.mono }))).clone();
            let n = b.starts.len() as i64 + 1 + i64::from(b.place_below);
            if n > 1 {
                h = if problems { n * text_h + PROBLEM_PAD } else { n * p.lh };
                breaks.insert(i, b);
            }
        }
        y += h;
        tops.push(y);
    }
    Flow { page: p.page, wrap: p.wrap, width: p.width, tops, breaks, text_h }
}

/// Where `text` breaks in rows `room` pixels wide in `font`, its `place`
/// (a problem's, after the message) taken into account.
fn break_line(text: &str, place: &str, room: i64, font: &Font) -> Broken {
    let chars: Vec<char> = text.chars().collect();
    let mut starts = Vec::new();
    let mut start = 0;
    let n = chars.len();
    // (a line that fits whole — the common one — takes one measure)
    while start < n && width(&chars[start..], font) > room {
        let fit = fit_chars(&chars[start..], room, font);
        let end = start + fit;
        // (a space near the edge to break after, else the edge itself)
        let cut = match chars[start..end].iter().rposition(|&c| c == ' ') {
            Some(at) if at + 1 > fit / 3 => start + at + 1,
            _ => end,
        };
        starts.push(cut);
        start = cut;
    }
    let place_below = !place.is_empty() && width(&chars[start.min(n)..], font) + PLACE_GAP + text_size(place, font).0 > room;
    Broken { starts, place_below }
}

fn width(chars: &[char], font: &Font) -> i64 {
    text_size(&chars.iter().collect::<String>(), font).0
}

/// How many of `chars` (the first, at least) fit in `room` pixels.
fn fit_chars(chars: &[char], room: i64, font: &Font) -> usize {
    let (mut lo, mut hi) = (1, chars.len());
    // (the most that fits: the widths only grow with the characters)
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        if width(&chars[..mid], font) <= room {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    lo
}

#[cfg(test)]
mod tests {
    use rapidr_value::panels::console::{self as model, Page, Problem, Severity};

    use super::*;

    fn mono() -> Font {
        Font { name: "Courier New".into(), size: -13, color: 0, styles: 0 }
    }

    fn params<'a>(id: &'a str, page: Page, width: i64, f: &'a Font, wrap: bool) -> Params<'a> {
        Params { id, page, width, mono: f, chrome: f, lh: text_size("Ag", f).1 + 2, wrap }
    }

    #[test]
    fn long_lines_break_at_spaces_and_short_ones_stay() {
        let f = mono();
        let adv = text_size("0", &f).0;
        // ten characters to a row
        let room = adv * 10;
        let b = break_line("aaaa bbbb cccc dddd", "", room, &f);
        // "aaaa bbbb " | "cccc dddd"
        assert_eq!(b.starts, vec![10]);
        // no space to break at: the edge
        let b = break_line("abcdefghijklmnopqrstuvwxyz", "", room, &f);
        assert_eq!(b.starts, vec![10, 20]);
        // fits whole
        assert!(break_line("short", "", room, &f).starts.is_empty());
        // a problem's place that doesn't fit after the last row goes under it
        assert!(break_line("0123456789012", "a.rr:3", room, &f).place_below);
        assert!(!break_line("012", "a.rr:3", adv * 20, &f).place_below);
    }

    #[test]
    fn the_flow_gives_each_line_its_rows_and_the_scroll_arithmetic() {
        let f = mono();
        let adv = text_size("0", &f).0;
        model::remove("flow1");
        model::with_mut("flow1", |c| c.write("short\naaaa bbbb cccc dddd\nlast\n"));
        // (a width of ten characters, the margins added)
        let width = adv * 10 + MARGIN + RIGHT;
        let wrapped = flow(&params("flow1", Page::Output, width, &f, true));
        let lh = text_size("Ag", &f).1 + 2;
        assert_eq!(wrapped.count(), 3);
        assert_eq!((wrapped.height(0), wrapped.height(1), wrapped.height(2)), (lh, 2 * lh, lh));
        assert_eq!(wrapped.total(), 4 * lh);
        assert_eq!(wrapped.rows_of(1, 19), vec![(0, 10), (10, 19)]);
        assert_eq!(wrapped.rows_of(0, 5), vec![(0, 5)]);
        // (the pixel row's line, and how far into it)
        assert_eq!(wrapped.line_at(lh + 1), Some(1));
        assert_eq!(wrapped.locate(2 * lh + 3), (1, lh + 3));
        assert_eq!(wrapped.locate(3 * lh), (2, 0));
        assert_eq!(wrapped.pos_of(1, lh), 2 * lh);
        assert_eq!(wrapped.lines_in(0, 3 * lh), 2);
        // Off: one row each, nothing breaks.
        let off = flow(&params("flow1", Page::Output, width, &f, false));
        assert_eq!((off.total(), off.broken(1).is_none()), (3 * lh, true));
        // The same again: the very flow, till the text changes.
        let again = flow(&params("flow1", Page::Output, width, &f, false));
        assert!(Rc::ptr_eq(&off, &again));
        model::with_mut("flow1", |c| c.write("aaaa bbbb cccc dddd\n"));
        let more = flow(&params("flow1", Page::Output, width, &f, false));
        assert_eq!(more.count(), 4);
        model::remove("flow1");
    }

    #[test]
    fn a_problem_wraps_to_its_message_and_keeps_its_place() {
        let f = mono();
        let adv = text_size("0", &f).0;
        model::remove("flow2");
        model::with_mut("flow2", |c| {
            c.add_problem(Problem { file: "a.rr".into(), line: 3, col: 7, severity: Severity::Error, message: "short".into() });
            c.add_problem(Problem { file: "a.rr".into(), line: 9, col: 1, severity: Severity::Warning, message: "aaaa bbbb cccc dddd eeee ffff".into() });
        });
        // (twenty characters to a row: the short one and its place fit)
        let width = adv * 20 + PROBLEM_LEFT + PROBLEM_RIGHT;
        let fl = flow(&params("flow2", Page::Problems, width, &f, true));
        let th = text_size("Ag", &f).1;
        assert_eq!(fl.height(0), rows::ROW, "a short one keeps its row");
        // (two rows of message — "aaaa bbbb cccc dddd " and "eeee ffff" — and the place after the last)
        let b = fl.broken(1).unwrap();
        assert_eq!((b.starts.clone(), b.place_below), (vec![20], false));
        assert_eq!(fl.height(1), 2 * th + PROBLEM_PAD);
        // (a width that leaves the place no room after the last row: under it)
        let narrow = flow(&params("flow2", Page::Problems, adv * 14 + PROBLEM_LEFT + PROBLEM_RIGHT, &f, true));
        let b = narrow.broken(1).unwrap();
        assert_eq!((b.starts.len(), b.place_below), (2, true));
        assert_eq!(narrow.height(1), 4 * th + PROBLEM_PAD);
        assert!(narrow.broken(0).is_some_and(|b| b.place_below), "even the short one, now");
        // (unwrapped: every row the same)
        let off = flow(&params("flow2", Page::Problems, width, &f, false));
        assert_eq!((off.height(0), off.height(1)), (rows::ROW, rows::ROW));
        model::remove("flow2");
    }
}
