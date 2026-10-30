//! A value editor's list: a choice's constants (ticks for a flag set), a
//! link's components or SUBs (and "Create handler"), the project's files, the
//! fonts. Up / Down, Page Up / Down, Home / End move; typing filters (or,
//! without a filter field, jumps to the item it starts); Space ticks a flag;
//! a click picks.

use rapidr_value::i18n::tr_ui;
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::{Place, Rect};
use rapidr_value::objects::text::text_size;

use super::field::{Accept, Field};
use super::Look;
use crate::paint::Painter;

/// A row's height (a target of at least 24 pixels).
pub const ROW: i64 = 24;
/// Rows shown at once.
pub const ROWS: usize = 9;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Item {
    pub label: String,
    /// Dim, at the right (a file's folder, a constant's value).
    pub detail: String,
    /// What a screen reader says for it (the label when empty).
    pub said: String,
    /// An action, not a value ("Create handler"): drawn in the accent.
    pub action: bool,
    /// A file that isn't there: marked.
    pub missing: bool,
    /// A colour shown before it (0xRRGGBB).
    pub swatch: Option<u32>,
    /// The label drawn in this font (a font's own name).
    pub face: Option<String>,
}

impl Item {
    pub fn new(label: impl Into<String>) -> Item {
        Item { label: label.into(), ..Item::default() }
    }

    pub fn said(&self) -> String {
        let mut s = if self.said.is_empty() { self.label.clone() } else { self.said.clone() };
        if self.missing {
            s.push_str(", ");
            s.push_str(&tr_ui("missing"));
        }
        s
    }
}

/// A label without a constant's lower-case prefix (`alTop` → `Top`,
/// `mbYes` → `Yes`); any other as it is.
fn stem(label: &str) -> &str {
    let lead = label.chars().take_while(|c| c.is_ascii_lowercase()).count();
    if lead > 0 && lead < label.len() && label[lead..].starts_with(|c: char| c.is_ascii_uppercase()) {
        &label[lead..]
    } else {
        label
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ListEd {
    pub items: Vec<Item>,
    /// The items the filter leaves, in order.
    pub shown: Vec<usize>,
    /// The selected one (in `shown`).
    pub sel: usize,
    pub top: usize,
    /// A flag set's ticks, an item each.
    pub ticks: Option<Vec<bool>>,
    /// A filter field over the list (long lists: files, fonts).
    pub filter: Option<Field>,
}

impl ListEd {
    pub fn new(items: Vec<Item>, selected: Option<usize>, filter: bool) -> ListEd {
        let shown = (0..items.len()).collect();
        let mut l = ListEd { items, shown, sel: 0, top: 0, ticks: None, filter: filter.then(|| Field::new("Filter", "", Accept::Text)) };
        if let Some(s) = selected {
            l.select_item(s);
        }
        l
    }

    /// The selected item's index in `items`.
    pub fn selected(&self) -> Option<usize> {
        self.shown.get(self.sel).copied()
    }

    pub fn select_item(&mut self, item: usize) {
        if let Some(k) = self.shown.iter().position(|&i| i == item) {
            self.sel = k;
            self.keep_visible();
        }
    }

    fn keep_visible(&mut self) {
        if self.sel < self.top {
            self.top = self.sel;
        } else if self.sel >= self.top + ROWS {
            self.top = self.sel + 1 - ROWS;
        }
    }

    /// The filter's text applied (its words, any order, any case).
    pub fn refilter(&mut self) {
        let words: Vec<String> = self.filter.as_ref().map_or_else(Vec::new, |f| f.text.to_lowercase().split_whitespace().map(str::to_string).collect());
        let keep = self.selected();
        self.shown = (0..self.items.len()).filter(|&i| self.items[i].action || words.iter().all(|w| self.items[i].label.to_lowercase().contains(w.as_str()))).collect();
        self.sel = keep.and_then(|k| self.shown.iter().position(|&i| i == k)).unwrap_or(0);
        self.top = 0;
        self.keep_visible();
    }

    /// Moves the selection (a key's): whether it was the list's.
    pub fn key(&mut self, vk: i64, text: &str) -> bool {
        let n = self.shown.len();
        let to = match vk {
            38 => Some(self.sel.saturating_sub(1)),
            40 => Some((self.sel + 1).min(n.saturating_sub(1))),
            33 => Some(self.sel.saturating_sub(ROWS - 1)),
            34 => Some((self.sel + ROWS - 1).min(n.saturating_sub(1))),
            36 if self.filter.is_none() => Some(0),
            35 if self.filter.is_none() => Some(n.saturating_sub(1)),
            32 if self.ticks.is_some() => {
                self.toggle();
                return true;
            }
            _ => None,
        };
        if let Some(to) = to {
            self.sel = to;
            self.keep_visible();
            return true;
        }
        if let Some(f) = &mut self.filter {
            if f.key(vk, text) == Some(true) {
                self.refilter();
                return true;
            }
            return false;
        }
        // (type-ahead: the next item starting with the letter — a
        // constant's after its prefix: `t` finds alTop)
        let Some(c) = text.chars().next().filter(|c| c.is_alphanumeric()) else { return false };
        let c = c.to_ascii_lowercase();
        let start = self.sel + 1;
        let hit = (0..n).map(|k| (start + k) % n).find(|&k| stem(&self.items[self.shown[k]].label).chars().next().map(|f| f.to_ascii_lowercase()) == Some(c));
        if let Some(k) = hit {
            self.sel = k;
            self.keep_visible();
        }
        true
    }

    /// Ticks or unticks the selected flag.
    pub fn toggle(&mut self) {
        if let (Some(t), Some(i)) = (&mut self.ticks, self.shown.get(self.sel).copied()) {
            if let Some(v) = t.get_mut(i) {
                *v = !*v;
            }
        }
    }

    /// The ticked items' labels.
    pub fn ticked(&self) -> Vec<String> {
        self.ticks.as_ref().map_or_else(Vec::new, |t| t.iter().enumerate().filter(|(_, on)| **on).map(|(i, _)| self.items[i].label.clone()).collect())
    }

    /// The height it takes for `rows` rows (and its filter).
    pub fn height(&self) -> i64 {
        let rows = self.shown.len().clamp(1, ROWS) as i64;
        rows * ROW + 4 + if self.filter.is_some() { ROW + 6 } else { 0 }
    }

    /// The list's rows' area within `r` (below the filter).
    pub fn rows_rect(&self, r: Rect) -> Rect {
        let f = if self.filter.is_some() { ROW + 6 } else { 0 };
        (r.0, r.1 + f, r.2, r.3 - f)
    }

    /// The item at `(x, y)` in `r` (an index of `shown`).
    pub fn at(&self, r: Rect, x: i64, y: i64) -> Option<usize> {
        let rr = self.rows_rect(r);
        if x < rr.0 || x >= rr.0 + rr.2 || y < rr.1 + 2 || y >= rr.1 + rr.3 {
            return None;
        }
        let k = self.top + ((y - rr.1 - 2) / ROW) as usize;
        (k < self.shown.len()).then_some(k)
    }

    /// The wheel scrolls three rows a notch (down: `dy` < 0).
    pub fn wheel(&mut self, dy: f64) {
        let max = self.shown.len().saturating_sub(ROWS);
        if dy < 0.0 {
            self.top = (self.top + 3).min(max);
        } else {
            self.top = self.top.saturating_sub(3);
        }
    }

    /// Draws it in `r`: the filter (focused when `filter_focus`), the rows,
    /// the selected one lit (`focused`: in the accent, else quietly).
    pub fn paint(&self, p: &mut Painter, r: Rect, focused: bool, filter_focus: bool, look: &Look, font: &Font) {
        if let Some(f) = &self.filter {
            f.paint(p, (r.0, r.1, r.2, ROW + 2), 0, filter_focus, look, font);
            if f.text.is_empty() && !filter_focus {
                let th = text_size("Ag", font).1;
                p.text((r.0 + 6, r.1 + 1 + (ROW - th) / 2, r.2 - 12, th), &tr_ui("Type to filter"), font, look.dim, Place::TopLeft);
            }
        }
        let rr = self.rows_rect(r);
        let th = text_size("Ag", font).1;
        let clip = rr;
        p.clipped(clip, |p| {
            for (k, &i) in self.shown.iter().enumerate().skip(self.top).take(ROWS) {
                let it = &self.items[i];
                let y = rr.1 + 2 + (k - self.top) as i64 * ROW;
                let row = (rr.0 + 2, y, rr.2 - 4, ROW);
                let sel = k == self.sel;
                let ink = if sel && focused { look.selected_text } else if it.action { look.accent_text } else { look.text };
                if sel {
                    p.round(row, 3.0, Some(if focused { look.selected } else { look.unfocused }), None, 1.0);
                }
                let mut x = row.0 + 8;
                if let Some(t) = &self.ticks {
                    let on = t.get(i).copied().unwrap_or(false);
                    let b = (x, y + (ROW - 14) / 2, 14, 14);
                    p.round(b, 2.0, Some(if on { look.accent } else { look.field }), Some(if on { look.accent } else { look.border }), 1.0);
                    if on {
                        p.stroke(&[(b.0 as f64 + 3.0, b.1 as f64 + 7.5), (b.0 as f64 + 6.0, b.1 as f64 + 10.5), (b.0 as f64 + 11.0, b.1 as f64 + 4.0)], look.accent_on, 1.6);
                    }
                    x += 22;
                }
                if let Some(c) = it.swatch {
                    p.round((x, y + 5, 14, 14), 2.0, Some(c), Some(look.border_strong), 1.0);
                    x += 20;
                }
                if it.action {
                    p.icon("actions/add", (x, y + (ROW - 16) / 2, 16, 16), Some(ink), false);
                    x += 20;
                }
                if it.missing {
                    p.icon("actions/error", (x, y + (ROW - 16) / 2, 16, 16), Some(look.error), false);
                    x += 20;
                }
                let face = it.face.as_ref().map(|f| Font { name: f.clone(), ..font.clone() });
                let lf = face.as_ref().unwrap_or(font);
                let lw = text_size(&it.label, lf).0;
                p.text((x, y + (ROW - th) / 2, (row.0 + row.2 - x - 8).max(0), th), &it.label, lf, ink, Place::TopLeft);
                if !it.detail.is_empty() {
                    let dw = text_size(&it.detail, font).0;
                    let dx = row.0 + row.2 - 8 - dw;
                    if dx > x + lw + 12 {
                        p.text((dx, y + (ROW - th) / 2, dw + 2, th), &it.detail, font, if sel && focused { look.selected_text } else { look.dim }, Place::TopLeft);
                    }
                }
            }
        });
        if self.shown.is_empty() {
            p.text((rr.0 + 10, rr.1 + 6, rr.2 - 20, th), &tr_ui("Nothing matches"), font, look.dim, Place::TopLeft);
        }
        if self.shown.len() > ROWS {
            let track = rr.3 - 8;
            let h = ((ROWS as f64 / self.shown.len() as f64) * track as f64).max(16.0) as i64;
            let y = rr.1 + 4 + ((self.top as f64 / (self.shown.len() - ROWS) as f64) * (track - h) as f64) as i64;
            p.round((rr.0 + rr.2 - 5, y, 3, h), 1.5, Some(look.border_strong), None, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filtering_and_type_ahead() {
        let names = ["alNone", "alTop", "alBottom", "alLeft", "alRight", "alClient"];
        let mut l = ListEd::new(names.iter().map(|n| Item::new(*n)).collect(), Some(5), false);
        assert_eq!(l.selected(), Some(5));
        l.key(38, "");
        assert_eq!(l.selected(), Some(4));
        l.key(0, "t");
        assert_eq!(l.items[l.selected().unwrap()].label, "alTop");
        let mut f = ListEd::new(["logo.png", "icons/save.png", "bg.jpg"].iter().map(|n| Item::new(*n)).collect(), None, true);
        f.filter.as_mut().unwrap().focus();
        f.key(0, "png");
        assert_eq!(f.shown, [0, 1]);
        let mut flags = ListEd::new(["mbYes", "mbNo", "mbOK"].iter().map(|n| Item::new(*n)).collect(), None, false);
        flags.ticks = Some(vec![true, false, false]);
        flags.key(40, "");
        flags.key(32, "");
        assert_eq!(flags.ticked(), ["mbYes", "mbNo"]);
    }
}
