//! QTABCONTROL (RapidQ manual, Appendix A): a row of tabs over an area the
//! program fills itself — not pages: it shows and hides its components in
//! OnChange. Shared by the desktop and web runtimes, which draw the same
//! [`TabControl::ops`] and lay aligned children out in the same
//! [`TabControl::display`] rectangle.
//!
//! RapidQ's QTabControl is Windows' tab control (SysTabControl32, drawn by
//! its owner for TabInactiveColor / TabInactiveFont), with Windows' rules:
//! the first tab added is selected; a tab inserted before the selected one
//! keeps that tab selected; deleting the selected tab leaves none selected
//! (TabIndex -1, the manual: "change the QTabControl index after the
//! call"); the program setting TabIndex doesn't fire OnChange, the user
//! clicking a tab (or the arrow keys) does. Its sizes are the classic
//! Windows ones: a tab is its text's width plus 6 pixels each side and the
//! font's height plus 5 high, the selected tab 2 pixels larger, the area
//! for the components 4 pixels inside the frame (TCM_ADJUSTRECT).
//!
//! Every layout is computed for tabs along the top ("canonical"), then
//! turned for TabPosition (bottom) and VerticalTabs (left / right).

use super::font::Font;
use super::text::text_size;
use crate::{v_int, v_str, Value};

/// Most tabs a control keeps.
const MAX_TABS: usize = 10_000;
/// The selected tab's extra size on each side.
const SELECTED: i64 = 2;
/// Text padding inside a tab, each side (Windows' DEFAULT_PADDING_X).
const PAD_X: i64 = 6;
/// Gaps between buttons (ButtonStyle), between flat ones, between rows.
const BUTTON_GAP: i64 = 3;
const FLAT_GAP: i64 = 8;
/// The scroll buttons' width (a single line of tabs too long to show).
const SCROLL_W: i64 = 17;

/// The light and dark edges of 3D frames, and the hot-tracked text.
const LIGHT: u32 = 0xFFFFFF;
const SHADOW: u32 = 0x808080;
const DARK: u32 = 0x404040;
const HOT: u32 = 0x003CB4;
const GRAY_TEXT: u32 = 0x808080;
const FACE: u32 = 0xF0F0F0;

#[derive(Clone, Debug)]
pub struct TabControl {
    pub tabs: Vec<String>,
    /// The selected tab; -1 none.
    pub index: i64,
    pub multi_line: bool,
    pub button_style: bool,
    pub flat_buttons: bool,
    pub flat_separators: bool,
    pub focus_buttons: bool,
    pub hot_track: bool,
    pub scroll_opposite: bool,
    pub vertical: bool,
    /// 0: top (left for vertical tabs); anything else: bottom (right).
    pub tab_position: i64,
    /// A tab's width / height; 0 fits its text / font.
    pub tab_width: i64,
    pub tab_height: i64,
    /// Inactive tabs' background (&HBBGGRR); `None`: the button face.
    pub inactive_color: Option<i64>,
    /// Inactive tabs' font (TabInactiveFont); `None`: the control's.
    pub inactive_font: Option<Font>,
    /// The first tab shown (a single line scrolled with its buttons).
    pub first: usize,
    /// The tab under the mouse (HotTrack).
    pub hot: Option<usize>,
    /// Goes up whenever it must be drawn (and its area laid out) again.
    pub revision: u64,
}

impl Default for TabControl {
    fn default() -> Self {
        TabControl {
            tabs: Vec::new(),
            index: -1,
            multi_line: false,
            button_style: false,
            flat_buttons: false,
            flat_separators: false,
            focus_buttons: false,
            hot_track: false,
            scroll_opposite: false,
            vertical: false,
            tab_position: 0,
            tab_width: 0,
            tab_height: 0,
            inactive_color: None,
            inactive_font: None,
            first: 0,
            hot: None,
            revision: 0,
        }
    }
}

/// A rectangle (x, y, width, height); what to draw, in the control's
/// pixels: the shared op vocabulary's model subset (`super::ops`), which
/// converts into the UI kernel's `ops::Op` unchanged.
pub use super::ops::{ModelOp as Op, Rect};

/// Which side of the area a tab hangs on (canonical: tabs along the top
/// hang on the area's top edge; ScrollOpposite's other rows on its bottom).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Side {
    Near,
    Far,
}

/// A tab laid out (canonical coordinates).
#[derive(Clone, Debug)]
struct Item {
    index: usize,
    rect: Rect,
    side: Side,
    /// Shown (a single line scrolled past it hides it).
    shown: bool,
}

#[derive(Clone, Debug)]
struct Layout {
    /// Along the tabs, and across them.
    depth: i64,
    items: Vec<Item>,
    /// The frame around the area (canonical), tabs style only.
    frame: Option<Rect>,
    display: Rect,
    /// The scroll buttons (back, forward) and whether each can scroll.
    scroller: Option<(Rect, Rect, bool, bool)>,
    /// Where the line of tabs is cut off by the scroll buttons.
    clip: i64,
}

/// `&HBBGGRR` → 0xRRGGBB.
fn rgb(bgr: i64) -> u32 {
    let c = (bgr & 0xFFFFFF) as u32;
    (c & 0xFF) << 16 | (c & 0xFF00) | (c >> 16)
}

impl TabControl {
    fn changed(&mut self) {
        self.revision += 1;
    }

    fn count(&self) -> usize {
        self.tabs.len()
    }

    /// Tabs wrap into rows (VerticalTabs always does, as Windows').
    fn multi(&self) -> bool {
        self.multi_line || self.vertical
    }

    fn far_position(&self) -> bool {
        self.tab_position != 0
    }

    /// Lengths along / across the tabs for a `w` × `h` control.
    fn axes(&self, w: i64, h: i64) -> (i64, i64) {
        if self.vertical { (h, w) } else { (w, h) }
    }

    fn tab_height_for(&self, font: &Font) -> i64 {
        if self.tab_height > 0 {
            return self.tab_height;
        }
        let fh = text_size("Ag", font).1.max(1);
        fh + SELECTED + if self.button_style { 6 } else { 3 }
    }

    fn tab_width_for(&self, i: usize, font: &Font) -> i64 {
        if self.tab_width > 0 {
            return self.tab_width;
        }
        text_size(&self.tabs[i], font).0 + 2 * PAD_X
    }

    fn layout(&self, w: i64, h: i64, font: &Font) -> Layout {
        let (len, depth) = self.axes(w, h);
        let tab_h = self.tab_height_for(font);
        let buttons = self.button_style;
        let gap = if !buttons { 0 } else if self.flat_buttons { FLAT_GAP } else { BUTTON_GAP };
        let start = if buttons { 0 } else { SELECTED };
        let end = if buttons { len } else { len - SELECTED };
        let widths: Vec<i64> = (0..self.count()).map(|i| self.tab_width_for(i, font)).collect();

        // Rows of tabs, in order (a single line: one row).
        let mut rows: Vec<Vec<usize>> = vec![Vec::new()];
        let mut x = start;
        for (i, &tw) in widths.iter().enumerate() {
            if self.multi() && rows.last().is_some_and(|r| !r.is_empty()) && x + tw > end {
                rows.push(Vec::new());
                x = start;
            }
            rows.last_mut().unwrap().push(i);
            x += tw + gap;
        }
        if rows.last().is_some_and(Vec::is_empty) && rows.len() > 1 {
            rows.pop();
        }
        let selected_row = usize::try_from(self.index).ok().and_then(|s| rows.iter().position(|r| r.contains(&s))).unwrap_or(0);

        // Each row's place: (side, distance from the area, 0 nearest).
        let n = rows.len();
        let mut place = vec![(Side::Near, 0usize); n];
        if self.scroll_opposite && self.multi() {
            for (r, p) in place.iter_mut().enumerate() {
                *p = if r >= selected_row { (Side::Near, r - selected_row) } else { (Side::Far, selected_row - 1 - r) };
            }
        } else {
            // The selected row moves next to the area, the others keep
            // their order behind it.
            let mut order = vec![selected_row];
            order.extend((0..n).filter(|r| *r != selected_row));
            for (k, r) in order.into_iter().enumerate() {
                place[r] = (Side::Near, k);
            }
        }
        let near_rows = place.iter().filter(|p| p.0 == Side::Near).count() as i64;
        let far_rows = place.iter().filter(|p| p.0 == Side::Far).count() as i64;
        let step = if buttons { tab_h + BUTTON_GAP } else { tab_h };
        // (canonical y of the area's top and bottom edges)
        let (near_edge, far_edge) = if buttons {
            (near_rows * step, depth - far_rows * step)
        } else {
            (SELECTED + near_rows * tab_h, depth - far_rows * tab_h - if far_rows > 0 { SELECTED } else { 0 })
        };

        // A single line too long: the scroll buttons, from `first`.
        let total: i64 = widths.iter().sum::<i64>() + gap * (widths.len() as i64 - 1).max(0);
        let scrolling = !self.multi() && start + total > end;
        let first = if scrolling { self.first.min(self.count().saturating_sub(1)) } else { 0 };
        let clip = if scrolling { len - 2 * SCROLL_W - 1 } else { len };

        let mut items = Vec::new();
        for (r, row) in rows.iter().enumerate() {
            let (side, k) = place[r];
            let k = k as i64;
            // Widen the tabs of a full row to its end (not the last row
            // alone, not fixed widths, not buttons) — as Windows does.
            let used: i64 = row.iter().map(|&i| widths[i]).sum::<i64>() + gap * (row.len() as i64 - 1).max(0);
            let justify = self.multi() && n > 1 && self.tab_width <= 0 && !buttons;
            let extra = if justify { (end - start - used).max(0) } else { 0 };
            let mut x = start;
            for (j, &i) in row.iter().enumerate() {
                if scrolling && i < first {
                    items.push(Item { index: i, rect: (-1000, 0, widths[i], tab_h), side, shown: false });
                    continue;
                }
                let mut tw = widths[i];
                if extra > 0 {
                    tw += extra / row.len() as i64 + i64::from((j as i64) < extra % row.len() as i64);
                }
                let y = match side {
                    Side::Near if buttons => k * step,
                    Side::Near => near_edge - (k + 1) * tab_h,
                    Side::Far if buttons => depth - (k + 1) * step + BUTTON_GAP,
                    Side::Far => far_edge + k * tab_h,
                };
                items.push(Item { index: i, rect: (x, y, tw, tab_h), side, shown: x < clip });
                x += tw + gap;
            }
        }
        items.sort_by_key(|it| it.index);

        let (frame, display) = if buttons {
            (None, (0, near_edge, len, (far_edge - near_edge).max(0)))
        } else {
            let frame = (0, near_edge, len, (far_edge - near_edge).max(0));
            (Some(frame), (4, near_edge + 4, (len - 8).max(0), (far_edge - near_edge - 8).max(0)))
        };
        let scroller = scrolling.then(|| {
            let sh = SCROLL_W.min(tab_h);
            let y = if buttons { 0 } else { near_edge - sh - 1 };
            let back = (len - 2 * SCROLL_W, y.max(0), SCROLL_W, sh);
            let fwd = (len - SCROLL_W, y.max(0), SCROLL_W, sh);
            let last_end = items.last().map_or(0, |it| it.rect.0 + it.rect.2);
            (back, fwd, first > 0, last_end > clip)
        });
        Layout { depth, items, frame, display, scroller, clip }
    }

    /// A canonical rectangle in the control's pixels (TabPosition,
    /// VerticalTabs turn it).
    fn map(&self, l: &Layout, r: Rect) -> Rect {
        let (x, y, w, h) = r;
        let far = self.far_position();
        match (self.vertical, far) {
            (false, false) => (x, y, w, h),
            (false, true) => (x, l.depth - y - h, w, h),
            (true, false) => (y, x, h, w),
            (true, true) => (l.depth - y - h, x, h, w),
        }
    }

    /// A point of the control in canonical coordinates.
    fn unmap(&self, l: &Layout, x: i64, y: i64) -> (i64, i64) {
        match (self.vertical, self.far_position()) {
            (false, false) => (x, y),
            (false, true) => (x, l.depth - 1 - y),
            (true, false) => (y, x),
            (true, true) => (y, l.depth - 1 - x),
        }
    }

    /// The area the components fill (Align), in the control's pixels.
    pub fn display(&self, w: i64, h: i64, font: &Font) -> Rect {
        let l = self.layout(w, h, font);
        self.map(&l, l.display)
    }

    /// Tab `i`'s rectangle in a `w` × `h` control, in the control's pixels
    /// (the selected tab as it's drawn, grown over its neighbours): what a
    /// screen reader is told its bounds are. `None`: no such tab, or one
    /// scrolled out of view.
    pub fn tab_rect(&self, i: usize, w: i64, h: i64, font: &Font) -> Option<Rect> {
        let l = self.layout(w, h, font);
        let it = l.items.iter().find(|it| it.index == i && it.shown)?;
        let r = if self.index == i as i64 && !self.button_style { Self::selected_rect(it) } else { it.rect };
        Some(self.map(&l, r))
    }

    /// The selected tab's rectangle grown by SELECTED on its sides and
    /// outward (and 1 into the frame, which it covers).
    fn selected_rect(item: &Item) -> Rect {
        let (x, y, w, h) = item.rect;
        match item.side {
            Side::Near => (x - SELECTED, y - SELECTED, w + 2 * SELECTED, h + SELECTED + 1),
            Side::Far => (x - SELECTED, y - 1, w + 2 * SELECTED, h + SELECTED + 1),
        }
    }

    /// The tab at a point of a `w` × `h` control: `Some(Ok(i))` a tab,
    /// `Some(Err(forward))` a scroll button, `None` nothing.
    fn hit(&self, x: i64, y: i64, w: i64, h: i64, font: &Font) -> Option<Result<usize, bool>> {
        let l = self.layout(w, h, font);
        let (cx, cy) = self.unmap(&l, x, y);
        let inside = |r: Rect| cx >= r.0 && cx < r.0 + r.2 && cy >= r.1 && cy < r.1 + r.3;
        if let Some((back, fwd, _, _)) = l.scroller {
            if inside(back) {
                return Some(Err(false));
            }
            if inside(fwd) {
                return Some(Err(true));
            }
        }
        if cx >= l.clip {
            return None;
        }
        // (the selected tab first: it's drawn over its neighbours)
        let sel = usize::try_from(self.index).ok();
        if let Some(it) = sel.and_then(|s| l.items.get(s)).filter(|it| it.shown && !self.button_style) {
            if inside(Self::selected_rect(it)) {
                return Some(Ok(it.index));
            }
        }
        l.items.iter().find(|it| it.shown && inside(it.rect)).map(|it| Ok(it.index))
    }

    /// The selected tab shown (a single line scrolls to it).
    fn scroll_to_selected(&mut self, w: i64, h: i64, font: &Font) {
        let Ok(sel) = usize::try_from(self.index) else { return };
        if sel < self.first {
            self.first = sel;
            return;
        }
        while self.first < sel {
            let l = self.layout(w, h, font);
            if l.scroller.is_none() || l.items.get(sel).is_some_and(|it| it.rect.0 + it.rect.2 <= l.clip) {
                break;
            }
            self.first += 1;
        }
    }

    /// The user selecting tab `i`: whether the selection changed (OnChange).
    fn user_select(&mut self, i: usize, w: i64, h: i64, font: &Font) -> bool {
        if i >= self.count() || self.index == i as i64 {
            return false;
        }
        self.index = i as i64;
        self.scroll_to_selected(w, h, font);
        self.changed();
        true
    }

    /// The mouse pressed at (x, y): `(changed, take_focus)` — the selection
    /// changed (OnChange), and whether the control takes the keyboard focus
    /// (always for tabs; buttons only with FocusButtons). `None`: not on a
    /// tab or a scroll button (the click is a component's).
    pub fn mouse_down(&mut self, x: i64, y: i64, w: i64, h: i64, font: &Font) -> Option<(bool, bool)> {
        let focus = !self.button_style || self.focus_buttons;
        match self.hit(x, y, w, h, font)? {
            Ok(i) => Some((self.user_select(i, w, h, font), focus)),
            Err(forward) => {
                let l = self.layout(w, h, font);
                let (_, _, can_back, can_fwd) = l.scroller.unwrap_or_default();
                if forward && can_fwd {
                    self.first += 1;
                    self.changed();
                } else if !forward && can_back {
                    self.first -= 1;
                    self.changed();
                }
                Some((false, false))
            }
        }
    }

    /// The mouse moved to (x, y) (or left: `None`): whether it must be
    /// drawn again (HotTrack's tab under the mouse).
    pub fn mouse_move(&mut self, at: Option<(i64, i64)>, w: i64, h: i64, font: &Font) -> bool {
        if !self.hot_track && !(self.button_style && self.flat_buttons) {
            return false;
        }
        let hot = at.and_then(|(x, y)| self.hit(x, y, w, h, font)).and_then(Result::ok);
        if hot == self.hot {
            return false;
        }
        self.hot = hot;
        self.changed();
        true
    }

    /// A key on the focused control (its virtual key code): the arrows pick
    /// the tab before / after. Whether the selection changed (OnChange).
    pub fn key(&mut self, vk: i64, w: i64, h: i64, font: &Font) -> bool {
        let to = match vk {
            37 | 38 => self.index - 1,
            39 | 40 => self.index + 1,
            _ => return false,
        };
        match usize::try_from(to) {
            Ok(i) => self.user_select(i, w, h, font),
            Err(_) => false,
        }
    }

    /// What to draw for a `w` × `h` control in `font`, its Color `color`
    /// (&HBBGGRR), enabled or not, with the keyboard focus or not.
    pub fn ops(&self, w: i64, h: i64, font: &Font, color: i64, enabled: bool, focused: bool) -> Vec<Op> {
        let l = self.layout(w, h, font);
        let mut out = Vec::new();
        let back = rgb(color);
        let inactive_back = self.inactive_color.map_or(FACE, rgb);
        out.push(Op::Fill { rect: (0, 0, w, h), color: back });
        let fill = |out: &mut Vec<Op>, r: Rect, color: u32| {
            let r = self.map(&l, r);
            if r.2 > 0 && r.3 > 0 {
                out.push(Op::Fill { rect: r, color });
            }
        };

        // The frame around the area: raised, its edge broken under the
        // selected tab.
        let sel = usize::try_from(self.index).ok().and_then(|s| l.items.get(s)).filter(|it| it.shown);
        if let Some((fx, fy, fw, fh)) = l.frame {
            let gap = |side: Side| sel.filter(|it| it.side == side).map(|it| {
                let r = Self::selected_rect(it);
                (r.0 + 1, (r.0 + r.2 - 1).min(l.clip))
            });
            let edge = |out: &mut Vec<Op>, y: i64, colors: &[u32], inward: i64, side: Side| {
                for (k, c) in colors.iter().enumerate() {
                    let yy = y + inward * k as i64;
                    match gap(side) {
                        Some((a, b)) => {
                            fill(out, (fx, yy, (a - fx).max(0), 1), *c);
                            fill(out, (b, yy, (fx + fw - b).max(0), 1), *c);
                        }
                        None => fill(out, (fx, yy, fw, 1), *c),
                    }
                }
            };
            // (which edges are light depends on where they end up)
            let (near_c, far_c): (&[u32], &[u32]) = self.edge_colors();
            edge(&mut out, fy, near_c, 1, Side::Near);
            edge(&mut out, fy + fh - 1, far_c, -1, Side::Far);
            let (start_c, end_c): (&[u32], &[u32]) = self.side_colors();
            for (k, c) in start_c.iter().enumerate() {
                fill(&mut out, (fx + k as i64, fy, 1, fh), *c);
            }
            for (k, c) in end_c.iter().enumerate() {
                fill(&mut out, (fx + fw - 1 - k as i64, fy, 1, fh), *c);
            }
        }

        // The tabs: the rows away from the area first, the selected last.
        let mut order: Vec<&Item> = l.items.iter().filter(|it| it.shown).collect();
        order.sort_by_key(|it| {
            let dist = match it.side {
                Side::Near => l.display.1 - (it.rect.1 + it.rect.3),
                Side::Far => it.rect.1 - (l.display.1 + l.display.3),
            };
            (Some(it.index) == sel.map(|s| s.index), std::cmp::Reverse(dist))
        });
        let inactive_font = self.inactive_font.as_ref().unwrap_or(font);
        for it in order {
            let selected = Some(it.index) == sel.map(|s| s.index);
            let hot = self.hot == Some(it.index);
            let r = if selected && !self.button_style { Self::selected_rect(it) } else { it.rect };
            if r.0 >= l.clip {
                continue;
            }
            let r = (r.0, r.1, r.2.min(l.clip - r.0), r.3);
            if self.button_style {
                self.draw_button(&mut out, &l, r, selected, hot, if selected { back } else { inactive_back });
            } else {
                self.draw_tab(&mut out, &l, r, it.side, if selected { back } else { inactive_back }, selected);
            }
            // Its text, centred (the selected tab's a pixel outward; a
            // pushed button's a pixel down and right).
            let (mut tx, mut ty) = (it.rect.0, it.rect.1);
            if selected && !self.button_style {
                ty += if it.side == Side::Near { -1 } else { 1 };
            }
            if selected && self.button_style {
                tx += 1;
                ty += 1;
            }
            let text_font = if selected { font } else { inactive_font };
            let color = if !enabled {
                GRAY_TEXT
            } else if hot && self.hot_track {
                HOT
            } else {
                rgb(text_font.color)
            };
            let tr = self.map(&l, (tx, ty, it.rect.2.min(l.clip - tx), it.rect.3));
            out.push(Op::Text { rect: tr, text: self.tabs[it.index].clone(), angle: self.text_angle(), font: text_font.clone(), color });
            if selected && focused {
                let (fx, fy, fw, fh) = (it.rect.0 + 3, ty + 3, it.rect.2 - 6, it.rect.3 - 6);
                if fw > 0 && fh > 0 {
                    out.push(Op::Focus { rect: self.map(&l, (fx, fy, fw, fh)) });
                }
            }
        }
        // FlatSeperators: an etched line between flat buttons.
        if self.button_style && self.flat_buttons && self.flat_separators {
            for it in l.items.iter().filter(|it| it.shown && it.index + 1 < self.count()) {
                let x = it.rect.0 + it.rect.2 + FLAT_GAP / 2 - 1;
                if x + 2 < l.clip {
                    fill(&mut out, (x, it.rect.1 + 2, 1, it.rect.3 - 4), SHADOW);
                    fill(&mut out, (x + 1, it.rect.1 + 2, 1, it.rect.3 - 4), LIGHT);
                }
            }
        }
        // The scroll buttons.
        if let Some((back_r, fwd_r, can_back, can_fwd)) = l.scroller {
            for (r, can, forward) in [(back_r, can_back, false), (fwd_r, can_fwd, true)] {
                self.draw_button(&mut out, &l, r, false, false, FACE);
                let (x, y, bw, bh) = self.map(&l, r);
                let (cx, cy) = (x as f64 + bw as f64 / 2.0, y as f64 + bh as f64 / 2.0);
                let s = 3.0;
                // (back: left / up; forward: right / down)
                let points = match (self.vertical, forward) {
                    (false, false) => [(cx + s / 2.0, cy - s), (cx + s / 2.0, cy + s), (cx - s / 2.0 - 1.0, cy)],
                    (false, true) => [(cx - s / 2.0, cy - s), (cx - s / 2.0, cy + s), (cx + s / 2.0 + 1.0, cy)],
                    (true, false) => [(cx - s, cy + s / 2.0), (cx + s, cy + s / 2.0), (cx, cy - s / 2.0 - 1.0)],
                    (true, true) => [(cx - s, cy - s / 2.0), (cx + s, cy - s / 2.0), (cx, cy + s / 2.0 + 1.0)],
                };
                out.push(Op::Arrow { points, color: if can && enabled { 0x000000 } else { GRAY_TEXT } });
            }
        }
        out
    }

    /// The text's angle: tabs on the left read upward, on the right
    /// downward (Windows' vertical tabs).
    fn text_angle(&self) -> i32 {
        match (self.vertical, self.far_position()) {
            (false, _) => 0,
            (true, false) => 90,
            (true, true) => -90,
        }
    }

    /// Whether a canonical direction ends up facing up or left (a light
    /// edge) — `outward_y`: the canonical edge faces -y (else +y).
    fn faces_light_y(&self, outward_y_negative: bool) -> bool {
        // (canonical -y is up for top tabs, down for bottom tabs, left for
        // left tabs, right for right tabs)
        outward_y_negative != self.far_position()
    }

    /// Lines (outermost first) of a raised edge facing light or dark.
    fn raised(light: bool) -> &'static [u32] {
        if light { &[LIGHT] } else { &[DARK, SHADOW] }
    }

    /// The frame's edge next to the near tabs (canonical top) and the far
    /// one, outermost line first.
    fn edge_colors(&self) -> (&'static [u32], &'static [u32]) {
        (Self::raised(self.faces_light_y(true)), Self::raised(self.faces_light_y(false)))
    }

    /// The frame's start (canonical left) and end edges — canonical -x is
    /// left for horizontal tabs, up for vertical ones: always light.
    fn side_colors(&self) -> (&'static [u32], &'static [u32]) {
        (Self::raised(true), Self::raised(false))
    }

    /// A tab with rounded outer corners, its edge on the area open.
    fn draw_tab(&self, out: &mut Vec<Op>, l: &Layout, r: Rect, side: Side, back: u32, _selected: bool) {
        let (x, y, w, h) = r;
        let mut fill = |r: Rect, c: u32| {
            let r = self.map(l, r);
            if r.2 > 0 && r.3 > 0 {
                out.push(Op::Fill { rect: r, color: c });
            }
        };
        // (canonical rows: the outer edge, toward -y for near tabs)
        let near = side == Side::Near;
        let (outer, inner_from, inner_to) = if near { (y, y + 2, y + h) } else { (y + h - 1, y, y + h - 2) };
        fill(if near { (x + 1, y + 1, w - 2, h - 1) } else { (x + 1, y, w - 2, h - 1) }, back);
        // outer edge and its corners
        let outer_light = self.faces_light_y(near);
        let oc = if outer_light { LIGHT } else { DARK };
        fill((x + 2, outer, w - 4, 1), oc);
        if !outer_light {
            fill((x + 2, if near { outer + 1 } else { outer - 1 }, w - 4, 1), SHADOW);
        }
        let corner = if near { outer + 1 } else { outer - 1 };
        fill((x + 1, corner, 1, 1), LIGHT);
        fill((x + w - 2, corner, 1, 1), DARK);
        // the sides: start light, end dark (shadow inside)
        fill((x, inner_from, 1, inner_to - inner_from), LIGHT);
        fill((x + w - 1, inner_from, 1, inner_to - inner_from), DARK);
        fill((x + w - 2, inner_from, 1, inner_to - inner_from), SHADOW);
    }

    /// A button (ButtonStyle): raised, or pushed in when selected; flat
    /// buttons show no edges but when pushed or under the mouse.
    fn draw_button(&self, out: &mut Vec<Op>, l: &Layout, r: Rect, pushed: bool, hot: bool, back: u32) {
        let mr = self.map(l, r);
        let (x, y, w, h) = mr;
        out.push(Op::Fill { rect: mr, color: back });
        let flat = self.button_style && self.flat_buttons;
        let lines: (&[u32], &[u32]) = match (flat, pushed, hot) {
            (true, false, false) => return,
            (true, false, true) => (&[LIGHT], &[SHADOW]),
            (true, true, _) => (&[SHADOW], &[LIGHT]),
            (false, false, _) => (&[LIGHT], &[DARK, SHADOW]),
            (false, true, _) => (&[DARK, SHADOW], &[LIGHT]),
        };
        // (in the control's pixels: top and left, then bottom and right)
        for (k, c) in lines.0.iter().enumerate() {
            let k = k as i64;
            out.push(Op::Fill { rect: (x + k, y + k, (w - 2 * k).max(0), 1), color: *c });
            out.push(Op::Fill { rect: (x + k, y + k, 1, (h - 2 * k).max(0)), color: *c });
        }
        for (k, c) in lines.1.iter().enumerate() {
            let k = k as i64;
            out.push(Op::Fill { rect: (x + k, y + h - 1 - k, (w - 2 * k).max(0), 1), color: *c });
            out.push(Op::Fill { rect: (x + w - 1 - k, y + k, 1, (h - 2 * k).max(0)), color: *c });
        }
    }

    // ---- the program's side ----

    /// AddTabs: tabs at the end (the first one added is selected).
    fn add(&mut self, caption: &str) {
        if self.count() >= MAX_TABS {
            return;
        }
        self.tabs.push(caption.to_string());
        if self.index < 0 {
            self.index = 0;
        }
    }

    /// InsertTab(Index, Caption): the selected tab stays selected.
    fn insert(&mut self, i: i64, caption: &str) {
        if self.count() >= MAX_TABS {
            return;
        }
        let i = i.clamp(0, self.count() as i64) as usize;
        self.tabs.insert(i, caption.to_string());
        if self.index < 0 {
            self.index = 0;
        } else if i as i64 <= self.index {
            self.index += 1;
        }
    }

    /// DelTabs(Index): deleting the selected tab leaves none selected.
    fn delete(&mut self, i: i64) {
        let Ok(i) = usize::try_from(i) else { return };
        if i >= self.count() {
            return;
        }
        self.tabs.remove(i);
        if i as i64 == self.index {
            self.index = -1;
        } else if (i as i64) < self.index {
            self.index -= 1;
        }
        self.first = self.first.min(self.count().saturating_sub(1));
        if self.hot.is_some_and(|h| h >= self.count()) {
            self.hot = None;
        }
    }

    pub fn get(&self, prop: &str) -> Option<Value> {
        let flag = |b: bool| v_int(if b { -1 } else { 0 });
        Some(match prop {
            "tabindex" => v_int(self.index),
            "multiline" => flag(self.multi_line),
            "buttonstyle" => flag(self.button_style),
            "flatbuttons" => flag(self.flat_buttons),
            "flatseperators" | "flatseparators" => flag(self.flat_separators),
            "focusbuttons" => flag(self.focus_buttons),
            "hottrack" => flag(self.hot_track),
            "scrollopposite" => flag(self.scroll_opposite),
            "verticaltabs" => flag(self.vertical),
            "tabposition" => v_int(self.tab_position),
            "tabwidth" => v_int(self.tab_width),
            "tabheight" => v_int(self.tab_height),
            "tabinactivecolor" => v_int(self.inactive_color.unwrap_or(0xF0F0F0)),
            _ => return None,
        })
    }

    pub fn set(&mut self, prop: &str, val: &Value) -> bool {
        let on = val.to_bool();
        match prop {
            // (the program's TabIndex: no OnChange; one past the tabs is
            // ignored, -1 selects none)
            "tabindex" => {
                let i = val.to_i64();
                if i < -1 || i >= self.count() as i64 {
                    return true;
                }
                self.index = i;
            }
            "multiline" => self.multi_line = on,
            "buttonstyle" => self.button_style = on,
            "flatbuttons" => self.flat_buttons = on,
            "flatseperators" | "flatseparators" => self.flat_separators = on,
            "focusbuttons" => self.focus_buttons = on,
            "hottrack" => {
                self.hot_track = on;
                self.hot = None;
            }
            "scrollopposite" => self.scroll_opposite = on,
            "verticaltabs" => self.vertical = on,
            "tabposition" => self.tab_position = val.to_i64(),
            "tabwidth" => self.tab_width = val.to_i64().clamp(0, 10_000),
            "tabheight" => self.tab_height = val.to_i64().clamp(0, 10_000),
            "tabinactivecolor" => self.inactive_color = Some(val.to_i64() & 0xFFFFFF),
            _ => return false,
        }
        self.changed();
        true
    }

    /// AddTabs, InsertTab, DelTabs, Tab(i) (read; with a second argument,
    /// set); `None` for other methods.
    pub fn call(&mut self, method: &str, args: &[Value]) -> Option<Value> {
        let arg = |i: usize| args.get(i).map(Value::to_string_val).unwrap_or_default();
        match method {
            "addtabs" | "addtab" => args.iter().for_each(|a| self.add(&a.to_string_val())),
            "inserttab" => self.insert(args.first().map_or(0, Value::to_i64), &arg(1)),
            // (RemoveTab: RapidR's own examples' name for it)
            "deltabs" | "deltab" | "removetab" => args.iter().for_each(|a| self.delete(a.to_i64())),
            "tab" | "tabs" | "tab=" | "tabs=" => {
                let i = usize::try_from(args.first().map_or(-1, Value::to_i64)).ok().filter(|i| *i < self.count());
                if args.len() >= 2 {
                    if let Some(i) = i {
                        self.tabs[i] = arg(1);
                    }
                } else {
                    return Some(v_str(i.map_or("", |i| self.tabs[i].as_str())));
                }
            }
            _ => return None,
        }
        self.changed();
        Some(Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tc(tabs: &[&str]) -> TabControl {
        let mut t = TabControl::default();
        t.call("addtabs", &tabs.iter().map(|s| v_str(s)).collect::<Vec<_>>());
        t
    }

    #[test]
    fn windows_selection_rules() {
        let mut t = TabControl::default();
        assert_eq!(t.index, -1);
        t.call("addtabs", &[v_str("A"), v_str("B"), v_str("C")]);
        assert_eq!(t.index, 0, "the first tab added is selected");
        t.set("tabindex", &v_int(2));
        t.call("inserttab", &[v_int(1), v_str("X")]);
        assert_eq!((t.index, t.call("tab", &[v_int(3)]).unwrap().to_string_val()), (3, "C".into()));
        t.call("deltabs", &[v_int(0)]);
        assert_eq!(t.index, 2);
        t.call("deltabs", &[v_int(2)]);
        assert_eq!(t.index, -1, "deleting the selected tab selects none");
        t.set("tabindex", &v_int(9));
        assert_eq!(t.index, -1, "past the tabs: ignored");
        t.call("tab", &[v_int(0), v_str("Z")]);
        assert_eq!(t.tabs, vec!["Z", "B"]);
    }

    #[test]
    fn layout_metrics_and_clicks() {
        let font = Font::default();
        let mut t = tc(&["One", "Two", "Three"]);
        let fh = text_size("Ag", &font).1;
        let (dx, dy, dw, dh) = t.display(300, 200, &font);
        // the area: 4 inside the frame, under one row of tabs
        assert_eq!((dx, dy, dw, dh), (4, 2 + fh + 5 + 4, 292, 200 - (2 + fh + 5) - 8));
        // a click on the second tab selects it (OnChange)
        let two_x = 2 + text_size("One", &font).0 + 12 + 5;
        assert_eq!(t.mouse_down(two_x, 10, 300, 200, &font), Some((true, true)));
        assert_eq!(t.index, 1);
        assert_eq!(t.mouse_down(two_x, 10, 300, 200, &font), Some((false, true)));
        assert_eq!(t.mouse_down(150, 150, 300, 200, &font), None, "the area is the components'");
        assert!(t.key(39, 300, 200, &font) && t.index == 2);
        assert!(!t.key(39, 300, 200, &font));
        // at the bottom: the area moves up
        t.set("tabposition", &v_int(1));
        assert_eq!(t.display(300, 200, &font).1, 4);
        // vertical tabs: on the left, the area to their right
        t.set("tabposition", &v_int(0));
        t.set("verticaltabs", &v_int(1));
        let (vx, vy, _, _) = t.display(300, 200, &font);
        assert_eq!((vx, vy), (2 + fh + 5 + 4, 4));
        assert!(t.ops(300, 200, &font, 0xF0F0F0, true, true).iter().any(|o| matches!(o, Op::Text { angle: 90, .. })));
    }

    #[test]
    fn tab_rects_are_where_clicks_select() {
        let font = Font::default();
        let mut t = tc(&["One", "Two", "Three"]);
        let fh = text_size("Ag", &font).1;
        // the selected tab grown by 2 each side and up, 1 into the frame
        assert_eq!(t.tab_rect(0, 300, 200, &font), Some((0, 0, text_size("One", &font).0 + 12 + 4, fh + 5 + 3)));
        let two = t.tab_rect(1, 300, 200, &font).unwrap();
        assert_eq!(two.1, 2, "unselected tabs sit 2 lower");
        assert_eq!(t.mouse_down(two.0 + two.2 / 2, two.1 + two.3 / 2, 300, 200, &font), Some((true, true)));
        assert_eq!(t.index, 1);
        assert_eq!(t.tab_rect(3, 300, 200, &font), None);
        // turned with the tabs
        t.set("verticaltabs", &v_int(1));
        let (x, y, w, h) = t.tab_rect(1, 300, 200, &font).unwrap();
        assert!(h > w && x == 0 && y > 0);
    }

    #[test]
    fn rows_and_scrolling() {
        let font = Font::default();
        let names: Vec<String> = (1..=9).map(|i| format!("Tab number {i}")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        // a single line too long: scroll buttons, and selecting scrolls
        let mut t = tc(&refs);
        let l = t.layout(200, 100, &font);
        assert!(l.scroller.is_some());
        t.set("tabindex", &v_int(0));
        let fwd = l.scroller.unwrap().1;
        assert_eq!(t.mouse_down(fwd.0 + 3, fwd.1 + 3, 200, 100, &font), Some((false, false)));
        assert_eq!(t.first, 1);
        assert!(t.key(39, 200, 100, &font) && t.index == 1);
        // multi-line: rows, the selected one next to the area
        let mut m = tc(&refs);
        m.set("multiline", &v_int(1));
        let rows = |m: &TabControl| m.layout(200, 300, &font).items.iter().map(|it| it.rect.1).collect::<std::collections::BTreeSet<_>>().len();
        assert!(rows(&m) > 2);
        m.set("tabindex", &v_int(8));
        let l = m.layout(200, 300, &font);
        let sel_bottom = l.items[8].rect.1 + l.items[8].rect.3;
        assert_eq!(sel_bottom, l.frame.unwrap().1, "the selected row touches the area");
        // ScrollOpposite: rows before the selected one go under the area
        m.set("scrollopposite", &v_int(1));
        let l = m.layout(200, 300, &font);
        assert!(l.items[0].side == Side::Far && l.items[0].rect.1 > l.display.1);
        // buttons: no frame
        let mut b = tc(&["A", "B"]);
        b.set("buttonstyle", &v_int(1));
        assert!(b.layout(200, 100, &font).frame.is_none());
    }
}
