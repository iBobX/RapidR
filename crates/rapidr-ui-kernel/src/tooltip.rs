//! Tooltips (RapidQ's hints): a component's `Hint` shown in a small box
//! when the mouse rests on it — after Windows' half second, for a few
//! seconds, gone as the mouse moves off, a button goes down or a key is
//! pressed — when its `ShowHint` is True, as in RapidQ (and Delphi). The
//! keyboard brings them too: a component Tab reaches shows its hint under
//! it. A component can give a tip of its own where it has no Hint
//! ([`ComponentKind::tip_at`](crate::components::ComponentKind::tip_at): a
//! dock's tab whose title is cut short, its header's buttons).
//!
//! Drawn by the kernel over everything, in the popup layer: the classic
//! look's is Windows' (the info colour, `Application.HintColor`'s default,
//! a black frame, MS Sans Serif); RapidR's own looks a rounded box in the
//! menus' colours and the chrome font. A screen reader hears a hint as its
//! component's description (`objects::a11y`), so the box itself is never
//! in the accessibility tree.

use std::time::Duration;

use rapidr_value::objects::ops::Place;
use rapidr_value::objects::text::text_size;

use crate::paint::Painter;
use crate::store::{self, Store};
use crate::tick::{now, Instant};
use crate::tree::FormUi;

/// Windows' hover time before a tooltip (Application.HintPause's default).
pub const PAUSE: Duration = Duration::from_millis(500);
/// How long one stays up (TTDT_AUTOPOP's default: ten times the pause).
pub const SHOWN: Duration = Duration::from_millis(5000);

/// A form's tooltip.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TipUi {
    /// The node the mouse (or the focus) rests on, and the text it shows.
    pub node: Option<usize>,
    pub text: String,
    /// Where the box goes: its top left (client pixels).
    pub at: (i64, i64),
    /// When it shows; when it goes.
    pub due: Option<Instant>,
    pub until: Option<Instant>,
    /// It is up.
    pub shown: bool,
}

/// The hint of node `i` at (x, y) of the client area: its own tip, or its
/// Hint when ShowHint says so.
fn tip_of(f: &mut FormUi, store: &dyn Store, i: usize, x: f64, y: f64) -> String {
    let (ax, ay, _, _) = f.nodes[i].abs;
    let id = f.nodes[i].id.clone();
    if let Some(kind) = f.nodes[i].kind {
        if let Some(t) = kind.tip_at(store, &id, x - ax as f64, y - ay as f64) {
            return t;
        }
    }
    if store::flag(store, &id, "showhint", false) {
        // (a hint with a "|": its short part, VCL's)
        let hint = store::string(store, &id, "hint");
        return hint.split('|').next().unwrap_or("").to_string();
    }
    String::new()
}

impl FormUi {
    /// The mouse moved to (x, y) over node `hit`: a tooltip waits, or
    /// follows, or goes.
    pub(crate) fn tip_mouse(&mut self, store: &dyn Store, hit: Option<usize>, x: f64, y: f64) {
        let text = match hit {
            Some(i) if self.nodes[i].shown => tip_of(self, store, i, x, y),
            _ => String::new(),
        };
        if hit == self.tip.node && text == self.tip.text {
            return;
        }
        let was_shown = self.tip.shown;
        self.tip_hide();
        if text.is_empty() {
            return;
        }
        // (moving from one tip to the next shows it at once, as Windows'
        // "reshow" delay does)
        let wait = if was_shown { Duration::ZERO } else { PAUSE };
        self.tip = TipUi { node: hit, text, at: (x.round() as i64 + 2, y.round() as i64 + 22), due: Some(now() + wait), until: None, shown: false };
    }

    /// The focus moved by the keyboard to the focused node: its hint shows
    /// under it.
    pub(crate) fn tip_focus(&mut self, store: &dyn Store) {
        self.tip_hide();
        let Some(i) = self.focus else { return };
        let (x, y, w, h) = self.nodes[i].abs;
        let text = tip_of(self, store, i, (x + w / 2) as f64, (y + h / 2) as f64);
        if !text.is_empty() {
            self.tip = TipUi { node: Some(i), text, at: (x, y + h + 4), due: Some(now() + PAUSE), until: None, shown: false };
        }
    }

    /// Any tooltip goes (a press, a key, the mouse gone).
    pub fn tip_hide(&mut self) {
        if self.tip.shown {
            self.dirty = true;
        }
        self.tip = TipUi::default();
    }

    /// The tooltip's deadline, if one waits.
    pub(crate) fn tip_wake(&self) -> Option<Instant> {
        if self.tip.shown { self.tip.until } else { self.tip.due }
    }

    /// Its deadline came: it shows, or goes.
    pub(crate) fn tip_tick(&mut self, at: Instant) {
        if !self.tip.shown {
            if self.tip.due.is_some_and(|d| d <= at) {
                self.tip.shown = true;
                self.tip.until = Some(at + SHOWN);
                self.dirty = true;
            }
        } else if self.tip.until.is_some_and(|u| u <= at) {
            self.tip_hide();
        }
    }

    /// The tooltip over everything (a form's popup layer).
    pub(crate) fn paint_tip(&self, p: &mut Painter) {
        if !self.tip.shown || self.tip.text.is_empty() {
            return;
        }
        let t = p.theme();
        let font = rapidr_value::ide_theme::chrome_font(t);
        let (tw, th) = text_size(&self.tip.text, &font);
        let (pad_x, pad_y) = if t.fluent() { (8, 5) } else { (4, 2) };
        let (w, h) = (tw + 2 * pad_x, th + 2 * pad_y);
        // (inside the window: moved left and up where it would leave it)
        let (cw, ch) = (self.client.0, self.client.1 + self.menu_offset);
        let x = self.tip.at.0.min(cw - w - 2).max(0);
        let mut y = self.tip.at.1;
        if y + h > ch {
            y = (self.tip.at.1 - h - 28).max(0);
        }
        let ink = if t.fluent() {
            if t.contrast {
                p.fill((x, y, w, h), 0x000000);
                p.frame((x, y, w, h), t.text);
            } else {
                p.elevate((x, y, w, h), t.radius, 6.0);
                p.round((x, y, w, h), t.radius, Some(t.menu), Some(t.border), 1.0);
            }
            t.menu_text
        } else {
            // (Windows' classic tooltip: the info colour, a black frame)
            p.fill((x, y, w, h), 0xFFFFE1);
            p.frame((x, y, w, h), 0x000000);
            0x000000
        };
        p.text((x + pad_x, y + pad_y, tw + 2, th), &self.tip.text, &font, ink, Place::Left);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadlines() {
        let t0 = now();
        let mut tip = TipUi { due: Some(t0 + PAUSE), ..TipUi::default() };
        assert!(!tip.shown);
        tip.shown = true;
        tip.until = Some(t0 + SHOWN);
        assert!(tip.until > tip.due);
    }
}
