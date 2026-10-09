//! ROUTPUTCONSOLE drawn and driven by the kernel; its model is
//! `rapidr_value::panels::console` (the web draws the same).
//!
//! Its parts, top down: a strip with the pages' tabs (Output, Build,
//! Problems — the problems' count in a badge) and the search box with
//! "3 of 12" beside it (ShowTabs = False hides the strip); then the shown
//! page: a program's output or the build log in a monospaced font (the
//! component's Font when the program set one, else Courier New), only the
//! lines in view laid out and drawn, coloured as the program asked
//! (ANSI's 16 colours; a high-contrast theme keeps its own), `file:line`
//! places as links (underlined under the mouse), the search's matches
//! marked; or the problems, a row each with its marker icon, its message
//! and its place dimmed.
//!
//! The mouse: a tab shows its page; a line clicked is selected (Shift:
//! the selection reaches it; a drag extends it); a link or a problem
//! clicked is OnLinkClick; the wheel and the bar scroll (the page stops
//! following new output until it's scrolled back to its end). The keys:
//! the arrows, Page Up / Down, Home / End (Shift extends), Ctrl+A,
//! Ctrl+C / Cmd+C copies the selected lines, Enter opens the caret line's
//! link (a problem), Ctrl+F or typing starts a search, Enter / F3 the next
//! match, Shift+F3 the previous, Escape clears the search,
//! Ctrl+Page Up / Down the pages.

use rapidr_value::objects::a11y::{part_id, AccessNode, Action, Role, PART_EDITOR, PART_ITEM, PART_ROW, PART_TAB};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::{Place, Rect};
use rapidr_value::objects::text::text_size;
use rapidr_value::panels::console::screen::{Line, PALETTE};
use rapidr_value::panels::console::{self as model, Match, Page, Problem, Severity};
use rapidr_value::panels::rows::{self, vk};
use rapidr_value::panels::User;

use super::common::{self, look, Look};
use crate::a11y::AccessValue;
use crate::components::list::{self, bar_mouse, bar_tick, begin_edit, editing, end_edit, vscroll_at, vscroll_state, InPlace};
use crate::components::{ComponentKind, Cx, Ime, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::input::{Clipboard, KernelEvent};
use crate::paint::Painter;
use crate::store::{self, Store};

pub struct Console;

/// The accessibility parts' indexes (`a11y::part_id`).
const A11Y_TABLIST: usize = 9;
const A11Y_SEARCH: usize = 10;
const A11Y_TEXT: usize = 11;
const A11Y_PROBLEMS: usize = 12;
const A11Y_PROBLEM: usize = 1000;
/// The most problems described one by one.
const A11Y_MAX_PROBLEMS: usize = 2000;
/// The text's margin at a line's left.
const MARGIN: i64 = 6;

/// Where its parts are (its own pixels).
#[derive(Clone, Debug)]
struct Geo {
    /// The tab strip (ShowTabs).
    strip: Option<Rect>,
    tabs: Vec<Rect>,
    /// The search box and the "3 of 12" beside it.
    search: Option<Rect>,
    /// The page: lines or rows, with the scroll bar.
    body: Rect,
    /// A line's (a problem's row's) height.
    lh: i64,
    mono: Font,
}

/// The output's font: the component's when the program set one, else
/// Courier New at 13 pixels (Liberation Mono where it isn't installed).
fn mono(cx: &Cx) -> Font {
    if !store::string(cx.store, cx.id, "fontname").trim().is_empty() {
        return Font { color: 0, ..cx.font.clone() };
    }
    Console::mono_default()
}

/// The tabs', the search box's and the problems' font: the theme's chrome
/// font (Inter in RapidR's look, MS Sans Serif in the classic one); the
/// component's Font is the output's (RapidR Studio sets a monospaced one).
fn chrome(cx: &Cx) -> Font {
    Font { color: cx.font.color, ..rapidr_value::ide_theme::chrome_font(rapidr_value::theme::current()) }
}

impl Console {
    /// The output's font when the program set none.
    fn mono_default() -> Font {
        Font { name: "Courier New".into(), size: -13, color: 0, styles: 0 }
    }
}

/// The tabs' rectangles in `strip` (as `common::tabs` lays them out).
fn tab_rects(strip: Rect, font: &Font) -> Vec<Rect> {
    let (x, y, _, h) = strip;
    let mut at = x + 4;
    Page::ALL
        .iter()
        .map(|p| {
            let (tw, _) = text_size(p.title(), font);
            let r = (at, y, tw + 20, h);
            at += r.2 + 2;
            r
        })
        .collect()
}

fn geo(cx: &Cx) -> Geo {
    let l = look(rapidr_value::theme::current());
    let (w, h) = (cx.width(), cx.height());
    let i = common::inset(&l);
    let show_tabs = model::with(cx.id, |c| c.show_tabs).unwrap_or(true);
    let mono = mono(cx);
    let lh = text_size("Ag", &mono).1 + 2;
    let (strip, tabs, search, top) = if show_tabs {
        let strip = (i, i, w - 2 * i, rows::TABS);
        let tabs = tab_rects(strip, &chrome(cx));
        let tabs_end = tabs.last().map_or(i, |t| t.0 + t.2 + 28);
        let sw = ((w - 2 * i) / 3).clamp(120, 220);
        let sx = w - i - sw - 4;
        let search = (sx >= tabs_end + 70).then_some((sx, i + 4, sw, rows::TABS - 8));
        (Some(strip), tabs, search, i + rows::TABS)
    } else {
        (None, Vec::new(), None, i)
    };
    let body = (i, top, (w - 2 * i).max(0), (h - top - i).max(0));
    Geo { strip, tabs, search, body, lh, mono }
}

/// The search editor's box over the search box's text (`list::editor_area`
/// puts the text where the box shows it).
fn editor_rect(search: Rect) -> Rect {
    let (x, y, w, h) = search;
    let s = (h - 8).clamp(10, 16);
    let area = (x + 10 + s, y + 1, (w - 14 - s).max(0), (h - 2).max(0));
    (area.0 - 3, area.1 - 1, area.2 + 6, area.3 + 2)
}

/// The shown page's row height (a line's, a problem's).
fn row_height(g: &Geo, page: Page) -> i64 {
    if page == Page::Problems {
        rows::ROW
    } else {
        g.lh
    }
}

/// What a paint needs from the model (taken in one go).
struct Snapshot {
    page: Page,
    first: u64,
    count: usize,
    view: model::View,
    lines: Vec<(u64, Line)>,
    problems: Vec<(usize, Problem)>,
    matches: Vec<Match>,
    current: Option<Match>,
    hover_link: Option<Match>,
    hover_tab: Option<usize>,
    hover_row: Option<usize>,
    filter: String,
    found: String,
    badge: Option<(usize, Severity)>,
    pos: i64,
}

/// The least contrast a line's text keeps: terminals' 4.5 : 1, less what
/// Windows' own highlight with white text lacks of it (4.499 : 1).
const MIN_CONTRAST: f64 = 4.45;

/// `ink` on `behind`, made readable ([`MIN_CONTRAST`]): toward the theme's
/// text or ground, whichever reads better there — a program's light cyan
/// on a light theme's white.
fn readable(ink: u32, behind: u32, l: &Look) -> u32 {
    let c = rapidr_icons::contrast;
    if c(ink, behind) >= MIN_CONTRAST {
        return ink;
    }
    let to = if c(l.text, behind) >= c(l.body, behind) { l.text } else { l.body };
    (1..=10).map(|k| common::mix(ink, to, f64::from(k) / 10.0)).find(|&m| c(m, behind) >= MIN_CONTRAST).unwrap_or(to)
}

/// The errors' count (else the warnings', else the others'), for the
/// Problems tab's badge.
fn badge_of(problems: &[Problem]) -> Option<(usize, Severity)> {
    [Severity::Error, Severity::Warning, Severity::Info].into_iter().find_map(|s| {
        let n = problems.iter().filter(|p| p.severity == s).count();
        (n > 0).then_some((n, s))
    })
}

impl Console {
    /// The bar's place now: the shown page's position (pixels), the width
    /// beside the bar, the bar's ops; the view kept in step with it.
    fn scroll(cx: &Cx, g: &Geo) -> (i64, i64, Vec<rapidr_value::objects::ops::Op>) {
        let (_, _, bw, bh) = g.body;
        model::with_mut(cx.id, |c| {
            let page = c.page;
            let rh = row_height(g, page);
            let (first, count) = (c.first(page), c.count(page));
            let pad = if page == Page::Problems { 0 } else { 4 };
            let content = count as i64 * rh + pad;
            let view_rows = ((bh - pad) / rh).max(1);
            let v = c.view_mut();
            // (a match or a key moved the caret: shown)
            if let (Some(caret), true) = (v.caret, !v.follow) {
                let top = v.top.max(first);
                if caret < top {
                    v.top = caret;
                } else if caret >= top + view_rows as u64 {
                    v.top = caret + 1 - view_rows as u64;
                }
            }
            let at = if v.follow { content } else { (v.top.saturating_sub(first) as i64) * rh };
            let (pos, cw, ops) = vscroll_at(cx.id, bw, bh, content, rh, Some(at));
            if !v.follow {
                v.top = first + (pos / rh) as u64;
            }
            (pos, cw, ops)
        })
    }

    /// The bar moved (the mouse, the wheel, a held arrow): the view follows
    /// it — and new output again once it's at the end.
    fn follow_bar(cx: &Cx, g: &Geo) {
        let (pos, shown, range) = vscroll_state(cx.id);
        model::with_mut(cx.id, |c| {
            let page = c.page;
            let rh = row_height(g, page);
            let first = c.first(page);
            let auto = c.auto_scroll;
            let v = c.view_mut();
            v.follow = auto && (!shown || pos >= range - g.body.3);
            v.top = first + (pos / rh) as u64;
        });
    }

    fn snapshot(cx: &Cx, g: &Geo, pos: i64) -> Snapshot {
        let (_, _, _, bh) = g.body;
        model::with_mut(cx.id, |c| {
            let page = c.page;
            let rh = row_height(g, page);
            let first = c.first(page);
            let count = c.count(page);
            let pad = if page == Page::Problems { 0 } else { 2 };
            let start = ((pos - pad).max(0) / rh) as usize;
            let end = (((pos + bh) / rh) as usize + 1).min(count);
            let lines = match c.screen(page) {
                Some(s) => (start..end).filter_map(|i| s.line(i).map(|l| (first + i as u64, l.clone()))).collect(),
                None => Vec::new(),
            };
            let problems = if page == Page::Problems { (start..end).filter_map(|i| c.problems.get(i).map(|p| (i, p.clone()))).collect() } else { Vec::new() };
            let (lo, hi) = (first + start as u64, first + end as u64);
            let all = c.matches();
            let a = all.partition_point(|m| m.line < lo);
            let b = all.partition_point(|m| m.line < hi);
            let matches = all[a..b].to_vec();
            let current = c.current.and_then(|k| c.matches().get(k).copied());
            let found = c.found_text();
            Snapshot {
                page,
                first,
                count,
                view: *c.view(),
                lines,
                problems,
                matches,
                current,
                hover_link: c.hover_link,
                hover_tab: c.hover_tab,
                hover_row: c.hover_row,
                filter: c.filter.clone(),
                found,
                badge: badge_of(&c.problems),
                pos,
            }
        })
    }

    fn paint_strip(cx: &mut Cx, p: &mut Painter, g: &Geo, s: &Snapshot, l: &Look) {
        let Some(strip) = g.strip else { return };
        let titles: Vec<&str> = Page::ALL.iter().map(|p| p.title()).collect();
        let tabs = common::tabs(p, strip, l, &titles, s.page.index(), s.hover_tab, &chrome(cx));
        // (the problems' count after their tab's title)
        if let (Some((n, sev)), Some(&tab)) = (s.badge, tabs.get(Page::Problems.index())) {
            let small = Font { size: (chrome(cx).size - 1).max(6), styles: 1, ..chrome(cx) };
            let text = n.to_string();
            let (tw, th) = text_size(&text, &small);
            let (bw, bh) = ((tw + 8).max(th + 2), th + 2);
            // (clear of the title, drawn bold while its page shows)
            let title_font = Font { styles: chrome(cx).styles | u8::from(s.page == Page::Problems && !l.classic), ..chrome(cx) };
            let (title_w, _) = text_size(Page::Problems.title(), &title_font);
            let x = (tab.0 + tab.2 - 5).max(tab.0 + (tab.2 + title_w) / 2 + 3);
            let r = (x, strip.1 + (strip.3 - bh) / 2, bw, bh);
            let color = match sev {
                Severity::Error => l.error,
                Severity::Warning => l.warning,
                Severity::Info => l.info,
            };
            if l.contrast {
                p.round(r, bh as f64 / 2.0, None, Some(color), 1.0);
                p.text(r, &text, &small, color, Place::Center);
            } else {
                p.round(r, bh as f64 / 2.0, Some(color), None, 1.0);
                p.text(r, &text, &small, l.body, Place::Center);
            }
        }
        if let Some(sr) = g.search {
            let ed = editing(cx.id);
            let focused = cx.state.focused && ed.is_some();
            if ed.is_some() {
                common::search_box(p, sr, l, "", "", focused, &chrome(cx));
                crate::components::edit::paint_line(cx, p, list::editor_area(editor_rect(sr)), crate::components::edit::Source::InPlace);
            } else {
                common::search_box(p, sr, l, &s.filter, "Search", false, &chrome(cx));
            }
            if !s.found.is_empty() {
                let (tw, _) = text_size(&s.found, &chrome(cx));
                let x = sr.0 - 8 - tw;
                if tabs.last().is_none_or(|t| x > t.0 + t.2 + 24) {
                    let color = if s.found == "No results" { l.error } else { l.dim };
                    p.text((x, strip.1, tw + 2, strip.3), &s.found, &chrome(cx), color, Place::Left);
                }
            }
        }
    }

    fn paint_lines(cx: &Cx, p: &mut Painter, g: &Geo, s: &Snapshot, l: &Look, cw: i64) {
        let (bx, by, _, bh) = g.body;
        let lh = g.lh;
        let focused = cx.state.focused && editing(cx.id).is_none();
        let sel = s.view.selection();
        let font = &g.mono;
        // (EmptyText while the page is empty: what to do — "Press F5 to run")
        if s.lines.is_empty() || s.lines.iter().all(|(_, line)| line.text().is_empty()) {
            let empty = crate::store::string(cx.store, cx.id, "emptytext");
            if !empty.trim().is_empty() && s.page == Page::Output {
                common::wrapped_center(p, (bx + 12, by + 12, (cw - 24).max(0), bh), &empty, &chrome(cx), l.dim);
            }
        }
        p.clipped((bx, by, cw, bh), |p| {
            for (abs, line) in &s.lines {
                let i = (abs - s.first) as i64;
                let y = by + 2 + i * lh - s.pos;
                let row = (bx, y, cw, lh);
                let selected = sel.is_some_and(|(a, b)| (a..=b).contains(abs));
                if selected {
                    p.fill(row, if focused { l.selected } else { l.inactive });
                }
                let chars: Vec<char> = line.text().chars().collect();
                let x_of = |k: usize| -> i64 { bx + MARGIN + if k == 0 { 0 } else { text_size(&chars[..k.min(chars.len())].iter().collect::<String>(), font).0 } };
                // (the program's backgrounds; none in high contrast)
                let spans = line.spans();
                if !l.contrast {
                    for &(a, b, attr) in &spans {
                        if let Some(bg) = attr.bg {
                            let (xa, xb) = (x_of(a), x_of(b));
                            p.fill((xa, y, xb - xa, lh), PALETTE[bg as usize & 15]);
                        }
                    }
                }
                // (the search's matches; the current one ringed)
                for m in s.matches.iter().filter(|m| m.line == *abs) {
                    let (xa, xb) = (x_of(m.start), x_of(m.end));
                    let r = (xa - 1, y, xb - xa + 2, lh);
                    p.round(r, 2.0, Some(l.found), None, 1.0);
                    if s.current == Some(*m) {
                        p.ring(r, 2.0, if l.contrast { l.focus } else { l.warning }, if l.contrast { 2.0 } else { 1.5 });
                    }
                }
                let links = rapidr_value::panels::console::links::of_line(line);
                let default_ink = if selected && focused { l.selected_text } else { l.text };
                for (a, b, attr) in spans {
                    // (cut where links start and end: a link in the link colour)
                    let mut cuts = vec![a, b];
                    for k in &links {
                        for e in [k.start, k.end] {
                            if e > a && e < b {
                                cuts.push(e);
                            }
                        }
                    }
                    cuts.sort_unstable();
                    cuts.dedup();
                    for pair in cuts.windows(2) {
                        let (ra, rb) = (pair[0], pair[1]);
                        let in_link = links.iter().any(|k| k.start <= ra && rb <= k.end);
                        let ink = match attr.fg {
                            Some(fg) if !l.contrast => PALETTE[fg as usize & 15],
                            _ if in_link => l.link,
                            _ => default_ink,
                        };
                        // (a colour that wouldn't read on what's behind it: made to)
                        let behind = match attr.bg {
                            Some(bg) if !l.contrast => PALETTE[bg as usize & 15],
                            _ if selected => if focused { l.selected } else { l.inactive },
                            _ => l.body,
                        };
                        let ink = readable(ink, behind, l);
                        let run: String = chars[ra..rb].iter().collect();
                        let xa = x_of(ra);
                        p.text((xa, y, cw.max(1) * 4, lh), &run, font, ink, Place::Left);
                    }
                }
                // (the link under the mouse: underlined)
                if let Some(h) = s.hover_link.filter(|h| h.line == *abs) {
                    let (xa, xb) = (x_of(h.start), x_of(h.end));
                    p.fill((xa, y + lh - 3, xb - xa, 1), l.link);
                }
                if focused && s.view.caret == Some(*abs) && sel.is_some_and(|(a, b)| a != b) {
                    common::row_focus(p, row, l);
                }
            }
        });
    }

    fn paint_problems(cx: &Cx, p: &mut Painter, g: &Geo, s: &Snapshot, l: &Look, cw: i64) {
        let (bx, by, _, bh) = g.body;
        let focused = cx.state.focused && editing(cx.id).is_none();
        if s.count == 0 {
            p.text((bx, by, cw, bh.min(rows::ROW * 3)), "No problems", &chrome(cx), l.dim, Place::Center);
            return;
        }
        p.clipped((bx, by, cw, bh), |p| {
            for (i, pr) in &s.problems {
                let y = by + *i as i64 * rows::ROW - s.pos;
                let row = (bx, y, cw, rows::ROW);
                let selected = s.view.caret == Some(*i as u64);
                let ink = common::row(p, row, l, selected, focused, s.hover_row == Some(*i));
                let marker = rapidr_icons::marker(pr.severity.name()).map_or(pr.severity.name(), |m| m.id);
                common::icon(p, marker, bx + 8, y + (rows::ROW - 16) / 2, 16, None, false);
                let place = pr.place();
                let (pw, _) = text_size(&place, &chrome(cx));
                let tx = bx + 30;
                let room = cw - (tx - bx) - 8;
                let mroom = if place.is_empty() { room } else { (room - pw - 12).max(room.min(60)) };
                let msg = common::elide(&pr.message, &chrome(cx), mroom);
                let (mw, _) = text_size(&msg, &chrome(cx));
                p.text((tx, y, mroom, rows::ROW), &msg, &chrome(cx), ink, Place::Left);
                if !place.is_empty() {
                    let px = tx + mw + 12;
                    let shown = common::elide(&place, &chrome(cx), bx + cw - 8 - px);
                    let dim = if selected && focused && !l.classic { ink } else if selected && focused { l.selected_text } else { l.dim };
                    p.text((px, y, bx + cw - px, rows::ROW), &shown, &chrome(cx), dim, Place::Left);
                }
                // (the selection shows the focus; classic's dotted rectangle too)
                if selected && focused && l.classic {
                    common::row_focus(p, row, l);
                }
            }
        });
    }

    /// The line (absolute) or problem row at (x, y) of the component, and
    /// the character there (a line's).
    fn hit(cx: &Cx, g: &Geo, x: f64, y: f64) -> Option<(u64, usize)> {
        let (bx, by, bw, bh) = g.body;
        let (xi, yi) = (x.floor() as i64, y.floor() as i64);
        if xi < bx || yi < by || xi >= bx + bw || yi >= by + bh {
            return None;
        }
        let (pos, _, _) = vscroll_state(cx.id);
        model::with(cx.id, |c| {
            let page = c.page;
            let first = c.first(page);
            if page == Page::Problems {
                let i = (yi - by + pos) / rows::ROW;
                return usize::try_from(i).ok().filter(|&i| i < c.problems.len()).map(|i| (i as u64, 0));
            }
            let i = (yi - by - 2 + pos).div_euclid(g.lh);
            let i = usize::try_from(i).ok().filter(|&i| i < c.count(page))?;
            let text = c.line_text(page, i).unwrap_or_default();
            // (the character under x: the one whose middle is past it)
            let mut at = 0;
            let mut prefix = String::new();
            for (k, ch) in text.chars().enumerate() {
                prefix.push(ch);
                let w = text_size(&prefix, &g.mono).0;
                if bx + MARGIN + w > xi {
                    at = k;
                    break;
                }
                at = k + 1;
            }
            Some((first + i as u64, at))
        })
        .flatten()
    }

    /// The caret moves to `to` (absolute; Shift: the selection reaches it).
    fn move_caret(cx: &Cx, to: u64, extend: bool) {
        model::with_mut(cx.id, |c| {
            let v = c.view_mut();
            if !(extend && v.anchor.is_some()) {
                v.anchor = Some(to);
            }
            v.caret = Some(to);
            v.follow = false;
        });
    }

    /// OnLinkClick for the problem at row `i`, or the first link on line
    /// `line` (absolute).
    fn open(cx: &mut Cx, at: u64) {
        let link = model::with(cx.id, |c| {
            let page = c.page;
            if page == Page::Problems {
                return c.problems.get(at as usize).map(|p| (p.file.clone(), p.line));
            }
            let i = at.checked_sub(c.first(page))? as usize;
            c.links_of(page, i).into_iter().next().map(|k| (k.file, k.line))
        })
        .flatten();
        if let Some((file, line)) = link {
            super::send(cx, User::Console(model::User::Link(file, line)));
        }
    }

    /// The search box takes the keyboard (with `text` typed into it).
    fn start_search(cx: &mut Cx, g: &Geo, typed: Option<&str>) {
        let Some(sr) = g.search else { return };
        let filter = model::with(cx.id, |c| c.filter.clone()).unwrap_or_default();
        cx.ui.edit = None;
        match typed {
            Some(t) => {
                let text = t.to_string();
                list::begin_edit_typed(cx.id, InPlace { target: (0, 0), text: text.clone(), rect: Some(editor_rect(sr)) });
                model::with_mut(cx.id, |c| c.find(&text));
            }
            None => begin_edit(cx.id, InPlace { target: (0, 0), text: filter, rect: Some(editor_rect(sr)) }),
        }
    }

    /// The search box's editor gives the keyboard back (its text kept).
    fn stop_search(cx: &mut Cx) {
        if end_edit(cx.id).is_some() {
            cx.ui.edit = None;
        }
    }

    /// The search's text as typed so far: the matches found again.
    fn sync_filter(cx: &Cx) {
        let Some(ed) = editing(cx.id) else { return };
        model::with_mut(cx.id, |c| {
            if c.filter != ed.text {
                c.find(&ed.text);
            }
        });
    }

    /// A key while the search box has the keyboard: whether it was its.
    fn search_key(cx: &mut Cx, g: &Geo, k: &KeyIn, clip: &mut dyn Clipboard) -> bool {
        let Some(sr) = g.search else {
            Self::stop_search(cx);
            return false;
        };
        match k.vk {
            vk::ENTER | 114 => {
                model::with_mut(cx.id, |c| c.find_next(k.mods.shift));
            }
            vk::DOWN => {
                model::with_mut(cx.id, |c| c.find_next(false));
            }
            vk::UP => {
                model::with_mut(cx.id, |c| c.find_next(true));
            }
            vk::ESCAPE => {
                Self::stop_search(cx);
                model::with_mut(cx.id, |c| c.clear_filter());
            }
            vk::TAB => {
                Self::stop_search(cx);
                return false;
            }
            _ => {
                let before = cx.events.len();
                list::edit_key(cx, k, clip, editor_rect(sr));
                // (the editor's OnChange isn't the console's)
                let id = cx.id.to_string();
                let mut k = before;
                while k < cx.events.len() {
                    if matches!(&cx.events[k], KernelEvent::Change(c) if *c == id) {
                        cx.events.remove(k);
                    } else {
                        k += 1;
                    }
                }
                Self::sync_filter(cx);
            }
        }
        true
    }
}

impl ComponentKind for Console {
    fn name(&self) -> &'static str {
        "ROUTPUTCONSOLE"
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        if editing(cx.id).is_none() {
            cx.ui.edit = None;
        }
        let l = look(p.theme());
        let (w, h) = (cx.width(), cx.height());
        common::ground(p, w, h, &l);
        let g = geo(cx);
        let (pos, cw, bar) = Self::scroll(cx, &g);
        let s = Self::snapshot(cx, &g, pos);
        if s.page == Page::Problems {
            Self::paint_problems(cx, p, &g, &s, &l, cw);
        } else {
            Self::paint_lines(cx, p, &g, &s, &l, cw);
        }
        p.at((g.body.0, g.body.1), |p| p.ops(bar));
        Self::paint_strip(cx, p, &g, &s, &l);
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let g = geo(cx);
        let (bx, by, bw, bh) = g.body;
        let inner = MouseIn { x: m.x - bx as f64, y: m.y - by as f64, ..*m };
        if bar_mouse(cx, &inner, bw, bh) {
            Self::follow_bar(cx, &g);
            return MouseOut::default();
        }
        // (the search box's editor)
        if let (Some(sr), Some(_)) = (g.search, editing(cx.id)) {
            if list::editor_mouse(cx, m, editor_rect(sr)) {
                return MouseOut::default();
            }
        }
        let (x, y) = (m.x.floor() as i64, m.y.floor() as i64);
        match m.kind {
            MouseKind::Leave => {
                model::with_mut(cx.id, |c| {
                    c.hover_link = None;
                    c.hover_tab = None;
                    c.hover_row = None;
                });
            }
            MouseKind::Move if m.captured => {
                // (a drag over the lines: the selection follows)
                if let Some((at, _)) = Self::hit(cx, &g, m.x, m.y.clamp(by as f64, (by + bh - 1) as f64)) {
                    if model::with(cx.id, |c| c.page != Page::Problems).unwrap_or(false) {
                        Self::move_caret(cx, at, true);
                    }
                }
            }
            MouseKind::Move => {
                let tab = g.tabs.iter().position(|&r| common::inside(r, x, y));
                let hit = Self::hit(cx, &g, m.x, m.y);
                let link = hit.and_then(|(line, at)| model::with(cx.id, |c| if c.page == Page::Problems { None } else { c.link_at(line, at).map(|(_, mm)| mm) }).flatten());
                model::with_mut(cx.id, |c| {
                    c.hover_tab = tab;
                    c.hover_link = link;
                    c.hover_row = if c.page == Page::Problems { hit.map(|h| h.0 as usize) } else { None };
                });
            }
            MouseKind::Down => {
                if let Some(t) = g.tabs.iter().position(|&r| common::inside(r, x, y)) {
                    Self::stop_search(cx);
                    super::send(cx, User::Console(model::User::Page(Page::ALL[t])));
                    return MouseOut::default();
                }
                if let Some(sr) = g.search.filter(|&r| common::inside(r, x, y)) {
                    Self::start_search(cx, &g, None);
                    list::editor_mouse(cx, m, editor_rect(sr));
                    return MouseOut::default();
                }
                Self::stop_search(cx);
                if let Some((at, ch)) = Self::hit(cx, &g, m.x, m.y) {
                    Self::move_caret(cx, at, m.mods.shift);
                    let link = model::with(cx.id, |c| if c.page == Page::Problems { Some(Match { line: at, start: 0, end: 0 }) } else { c.link_at(at, ch).map(|(_, mm)| mm) }).flatten();
                    model::with_mut(cx.id, |c| c.hover_link = link.filter(|_| c.page != Page::Problems));
                    PRESSED.with(|p| p.borrow_mut().insert(cx.id.to_string(), link));
                } else {
                    PRESSED.with(|p| p.borrow_mut().remove(cx.id));
                }
            }
            MouseKind::Up => {
                let pressed = PRESSED.with(|p| p.borrow_mut().remove(cx.id)).flatten();
                if let (Some(pm), Some((at, ch))) = (pressed, Self::hit(cx, &g, m.x, m.y)) {
                    let same = model::with(cx.id, |c| if c.page == Page::Problems { at == pm.line } else { c.link_at(at, ch).is_some_and(|(_, mm)| mm == pm) }).unwrap_or(false);
                    if same && m.inside {
                        Self::open_at(cx, pm.line, pm.start);
                    }
                }
            }
        }
        MouseOut::default()
    }

    fn wheel(&self, cx: &mut Cx, _dx: f64, dy: f64, _mods: crate::input::Mods) -> bool {
        let g = geo(cx);
        if !list::vscroll_wheel(cx.id, dy, g.body.2, g.body.3) {
            return false;
        }
        Self::follow_bar(cx, &g);
        true
    }

    fn tick(&self, cx: &mut Cx) {
        if bar_tick(cx) {
            let g = geo(cx);
            Self::follow_bar(cx, &g);
        }
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, clip: &mut dyn Clipboard) -> bool {
        let g = geo(cx);
        if editing(cx.id).is_some() {
            return Self::search_key(cx, &g, k, clip);
        }
        let ctrl = k.mods.ctrl || k.mods.command;
        let Some((page, first, count, caret, filter)) = model::with(cx.id, |c| (c.page, c.first(c.page), c.count(c.page), c.view().caret, c.filter.clone())) else { return false };
        let rh = row_height(&g, page);
        let page_rows = (g.body.3 / rh).max(1) as usize;
        match k.vk {
            // Ctrl+F: the search box
            70 if ctrl && !k.mods.alt => Self::start_search(cx, &g, None),
            // F3 / Shift+F3: the next / previous match
            114 => {
                model::with_mut(cx.id, |c| c.find_next(k.mods.shift));
            }
            vk::ESCAPE if !filter.is_empty() => model::with_mut(cx.id, |c| c.clear_filter()),
            // Ctrl+A: every line
            65 if ctrl && page != Page::Problems => model::with_mut(cx.id, |c| {
                if count > 0 {
                    let v = c.view_mut();
                    v.anchor = Some(first);
                    v.caret = Some(first + count as u64 - 1);
                }
            }),
            // Ctrl+C / Cmd+C (Ctrl+Insert): the selected lines
            67 | 45 if ctrl => {
                if let Some(t) = model::with(cx.id, |c| c.selected_text()).flatten() {
                    clip.set_text(&t);
                }
            }
            // Ctrl+Page Up / Down: the pages
            vk::PAGE_UP | vk::PAGE_DOWN if ctrl => {
                let i = page.index();
                let to = if k.vk == vk::PAGE_DOWN { (i + 1) % 3 } else { (i + 2) % 3 };
                super::send(cx, User::Console(model::User::Page(Page::ALL[to])));
            }
            vk::ENTER => {
                if let Some(at) = caret {
                    Self::open(cx, at);
                } else {
                    return false;
                }
            }
            vk::UP | vk::DOWN | vk::PAGE_UP | vk::PAGE_DOWN | vk::HOME | vk::END if !ctrl || matches!(k.vk, vk::HOME | vk::END) => {
                let at = caret.and_then(|c| c.checked_sub(first)).map(|c| c as usize);
                if let Some(to) = rows::nav(k.vk, at, count, page_rows) {
                    Self::move_caret(cx, first + to as u64, k.mods.shift && page != Page::Problems);
                }
            }
            _ if !k.text.is_empty() && !ctrl && !k.mods.alt && k.text.chars().all(|c| !c.is_control()) && g.search.is_some() => {
                Self::start_search(cx, &g, Some(k.text));
            }
            _ => return false,
        }
        true
    }

    fn ime(&self, cx: &mut Cx, ime: &Ime) -> bool {
        let g = geo(cx);
        let Some(sr) = g.search else { return false };
        let took = list::editor_ime(cx, ime, editor_rect(sr));
        Self::sync_filter(cx);
        took
    }

    fn ime_area(&self, cx: &mut Cx) -> Option<Rect> {
        let g = geo(cx);
        list::editor_ime_area(cx, editor_rect(g.search?))
    }

    fn wants_ime(&self, _store: &dyn Store, id: &str) -> bool {
        editing(id).is_some()
    }

    fn context_menu(&self, cx: &mut Cx) -> Option<crate::components::edit::MenuState> {
        let g = geo(cx);
        list::editor_menu(cx, editor_rect(g.search?))
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = crate::components::shared_describe(cx, self.name());
        n.role = Role::Group;
        n.value = None;
        n.states.multiline = false;
        n.states.read_only = false;
        if n.name.is_empty() {
            n.name = "Output".into();
        }
        let g = geo(cx);
        let (x0, y0) = (cx.rect.0, cx.rect.1);
        let at = |r: Rect| (x0 + r.0, y0 + r.1, r.2, r.3);
        let (page, show_tabs, filter, found, badge) = model::with_mut(cx.id, |c| (c.page, c.show_tabs, c.filter.clone(), c.found_text(), badge_of(&c.problems)));
        let editing_now = editing(cx.id).is_some();
        if show_tabs {
            let mut list = AccessNode::new(part_id(cx.id, PART_TAB, A11Y_TABLIST), Role::TabList);
            list.name = "Pages".into();
            if let Some(strip) = g.strip {
                list.bounds = at(strip);
            }
            for (i, p) in Page::ALL.iter().enumerate() {
                let mut t = AccessNode::new(part_id(cx.id, PART_TAB, i), Role::Tab);
                t.name = p.title().into();
                if *p == Page::Problems {
                    if let Some((count, sev)) = badge {
                        t.name = format!("Problems ({count} {}{})", sev.name(), if count == 1 { "" } else { "s" });
                    }
                }
                t.states.selected = Some(*p == page);
                t.actions = vec![Action::Click];
                if let Some(&r) = g.tabs.get(i) {
                    t.bounds = at(r);
                }
                list.children.push(t);
            }
            n.children.push(list);
            if let Some(sr) = g.search {
                let mut s = AccessNode::new(part_id(cx.id, PART_EDITOR, A11Y_SEARCH), Role::TextInput);
                s.name = "Search".into();
                s.value = Some(filter);
                s.description = found;
                s.states.focused = editing_now;
                s.actions = vec![Action::Focus, Action::SetValue];
                s.bounds = at(sr);
                n.children.push(s);
            }
        }
        // (the position as the model has it now — AutoScroll's end once lines
        // were written — not as the last paint left it: a hidden pane isn't
        // painted on every host)
        let (pos, _, _) = Self::scroll(cx, &g);
        if page == Page::Problems {
            let mut lb = AccessNode::new(part_id(cx.id, PART_ROW, A11Y_PROBLEMS), Role::ListBox);
            lb.name = "Problems".into();
            lb.bounds = at(g.body);
            lb.states.focused = !editing_now;
            let (problems, caret) = model::with(cx.id, |c| (c.problems.clone(), c.view().caret)).unwrap_or_default();
            // (a long list: the rows around the ones in view)
            let start = ((pos / rows::ROW) as usize).saturating_sub(A11Y_MAX_PROBLEMS / 2).min(problems.len().saturating_sub(A11Y_MAX_PROBLEMS));
            for (i, pr) in problems.iter().enumerate().skip(start).take(A11Y_MAX_PROBLEMS) {
                let mut o = AccessNode::new(part_id(cx.id, PART_ITEM, A11Y_PROBLEM + i), Role::ListBoxOption);
                o.name = pr.spoken();
                o.states.selected = Some(caret == Some(i as u64));
                o.actions = vec![Action::Click];
                o.bounds = at((g.body.0, g.body.1 + i as i64 * rows::ROW - pos, g.body.2, rows::ROW));
                lb.children.push(o);
            }
            n.children.push(lb);
        } else {
            let mut t = AccessNode::new(part_id(cx.id, PART_ROW, A11Y_TEXT), Role::MultilineTextInput);
            t.name = page.title().into();
            t.states.read_only = true;
            t.states.multiline = true;
            t.states.focused = !editing_now;
            t.bounds = at(g.body);
            // (what's in view)
            let rows_in_view = (g.body.3 / g.lh).max(1) as usize + 1;
            let start = (pos.max(0) / g.lh) as usize;
            t.value = model::with(cx.id, |c| (start..(start + rows_in_view).min(c.count(page))).filter_map(|i| c.line_text(page, i)).collect::<Vec<_>>().join("\n"));
            n.children.push(t);
        }
        n
    }

    fn access(&self, cx: &mut Cx, action: Action, part: Option<usize>, value: Option<&AccessValue>) -> bool {
        let Some(part) = part else { return false };
        match (part, action) {
            (i, Action::Click) if i < Page::ALL.len() => super::send(cx, User::Console(model::User::Page(Page::ALL[i]))),
            (A11Y_SEARCH, Action::Focus) => {
                let g = geo(cx);
                Self::start_search(cx, &g, None);
            }
            (A11Y_SEARCH, Action::SetValue) => {
                let text = match value {
                    Some(AccessValue::Text(t)) => t.clone(),
                    Some(AccessValue::Number(n)) => n.to_string(),
                    None => String::new(),
                };
                if editing(cx.id).is_some() {
                    list::set_edit_text(cx.id, &text);
                }
                model::with_mut(cx.id, |c| c.find(&text));
            }
            (i, Action::Click | Action::Focus) if i >= A11Y_PROBLEM => {
                let row = (i - A11Y_PROBLEM) as u64;
                Self::move_caret(cx, row, false);
                if action == Action::Click {
                    Self::open(cx, row);
                }
            }
            _ => return false,
        }
        true
    }
}

thread_local! {
    /// A press on a link (a problem): its release on the same one opens it.
    static PRESSED: std::cell::RefCell<std::collections::HashMap<String, Option<Match>>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

impl Console {
    /// The link pressed and let go on (line `line`, its start `start`; a
    /// problem's row): OnLinkClick.
    fn open_at(cx: &mut Cx, line: u64, start: usize) {
        let link = model::with(cx.id, |c| if c.page == Page::Problems { c.problems.get(line as usize).map(|p| (p.file.clone(), p.line)) } else { c.link_at(line, start).map(|(k, _)| (k.file, k.line)) }).flatten();
        if let Some((file, line)) = link {
            super::send(cx, User::Console(model::User::Link(file, line)));
        }
    }
}

/// The focus left it while its search box had the keyboard: the search
/// stays as typed.
pub fn focus_left(_id: &str, _ed: crate::components::list::InPlace) -> Vec<KernelEvent> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use rapidr_value::input::Button;
    use rapidr_value::panels::console::{self as model, Page, Problem, Severity};
    use rapidr_value::panels::User;
    use rapidr_value::v_int;

    use crate::components::form::Container;
    use crate::input::MemClipboard;
    use crate::store::Store;
    use crate::{FormUi, KernelEvent, MemStore, Mods, TextSystem};

    fn store(id: &str) -> MemStore {
        let mut s = MemStore::new();
        s.add("cform", "RFORM", None);
        s.add(id, "ROUTPUTCONSOLE", Some("cform")).set(id, "width", v_int(500)).set(id, "height", v_int(240));
        s
    }

    fn panel_events(f: &mut FormUi) -> Vec<User> {
        f.take_events()
            .into_iter()
            .filter_map(|e| match e {
                KernelEvent::Container(Container::Panel { action, .. }) => Some(action),
                _ => None,
            })
            .collect()
    }

    fn click(f: &mut FormUi, s: &MemStore, ts: &mut TextSystem, x: f64, y: f64, mods: Mods) {
        f.mouse_down(s, ts, x, y, Button::Left, mods);
        f.mouse_up(s, ts, x, y, Button::Left, mods);
    }

    /// AutoScroll: the lines written follow to the end whether or not the
    /// console was painted since (a hidden pane isn't, on every host) —
    /// what a screen reader hears is the last lines, as on screen.
    #[test]
    fn autoscroll_follows_the_end_without_a_paint() {
        let s = store("con_auto");
        let mut ts = TextSystem::new();
        model::remove("con_auto");
        model::with_mut("con_auto", |c| c.write("one\n"));
        let mut f = FormUi::build(&s, "cform", false);
        f.paint(&s, &mut ts, 1.0);
        // (a hundred lines more, no paint in between)
        model::with_mut("con_auto", |c| {
            for i in 0..100 {
                c.write(&format!("line {i}\n"));
            }
        });
        let tree = f.access_tree(&s, &mut ts).to_json();
        assert!(tree.contains("line 99") && !tree.contains("\"one"), "{tree}");
        // (the same once painted)
        f.paint(&s, &mut ts, 1.0);
        let painted = f.access_tree(&s, &mut ts).to_json();
        assert_eq!(painted, tree);
    }

    #[test]
    fn tabs_links_selection_search_and_copy() {
        // (the places below are the classic look's: named, as RapidR's is
        // every program's default)
        rapidr_value::theme::set(&rapidr_value::theme::CLASSIC);
        let s = store("con1");
        let mut ts = TextSystem::new();
        model::remove("con1");
        model::with_mut("con1", |c| {
            c.write("first\nsee Main.rr:12 here\nthird\n");
            c.add_problem(Problem { file: "a.bas".into(), line: 3, col: 1, severity: Severity::Warning, message: "w".into() });
        });
        let mut f = FormUi::build(&s, "cform", false);
        f.paint(&s, &mut ts, 1.0);
        // (the classic look's edge: 2 pixels; the strip 28; lines from 2 under it)
        let lh = rapidr_value::objects::text::text_size("Ag", &super::Console::mono_default()).1 + 2;
        let line_y = |i: i64| (2 + 28 + 2 + i * lh + lh / 2) as f64;
        // a link: OnLinkClick
        click(&mut f, &s, &mut ts, 2.0 + 6.0 + 8.0 * 6.0, line_y(1), Mods::NONE);
        assert_eq!(panel_events(&mut f), vec![User::Console(model::User::Link("Main.rr".into(), 12))]);
        // a line, Shift + another, Ctrl+C: the lines between
        click(&mut f, &s, &mut ts, 300.0, line_y(0), Mods::NONE);
        click(&mut f, &s, &mut ts, 300.0, line_y(2), Mods::SHIFT);
        let mut clip = MemClipboard::default();
        let ctrl = Mods { ctrl: true, ..Mods::NONE };
        f.key_down(&s, &mut ts, 67, "", ctrl, &mut clip);
        assert_eq!(clip.0.as_deref(), Some("first\nsee Main.rr:12 here\nthird"));
        // Ctrl+A
        model::with_mut("con1", |c| c.write("fourth"));
        f.key_down(&s, &mut ts, 65, "", ctrl, &mut clip);
        f.key_down(&s, &mut ts, 67, "", ctrl, &mut clip);
        assert_eq!(clip.0.as_deref(), Some("first\nsee Main.rr:12 here\nthird\nfourth"));
        // typing searches; Escape clears
        f.key_down(&s, &mut ts, 72, "h", Mods::NONE, &mut clip);
        assert_eq!(model::with("con1", |c| c.filter.clone()).as_deref(), Some("h"));
        f.key_down(&s, &mut ts, 73, "i", Mods::NONE, &mut clip);
        assert_eq!(model::with_mut("con1", |c| c.found_text()), "1 of 1");
        f.key_down(&s, &mut ts, 27, "", Mods::NONE, &mut clip);
        assert_eq!(model::with("con1", |c| c.filter.clone()).as_deref(), Some(""));
        // the Problems tab; screen readers see the tabs and the problems
        f.paint(&s, &mut ts, 1.0);
        let tabs = super::tab_rects((2, 2, 496, 28), &s.font("con1"));
        click(&mut f, &s, &mut ts, (tabs[2].0 + 5) as f64, 14.0, Mods::NONE);
        assert_eq!(panel_events(&mut f), vec![User::Console(model::User::Page(Page::Problems))]);
        model::with_mut("con1", |c| c.show(Page::Problems));
        let tree = f.access_tree(&s, &mut ts);
        let con = &tree.children[0];
        assert_eq!(con.children[0].children.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(), ["Output", "Build", "Problems (1 warning)"]);
        assert_eq!(con.children.last().unwrap().children[0].name, "Warning: w, a.bas line 3");
        model::remove("con1");
    }
}
