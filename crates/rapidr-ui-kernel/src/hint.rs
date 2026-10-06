//! Hints (`rapidr_value::hints`): what the mouse is over tells the program
//! the application's hint — the long part of that component's Hint, or
//! its parent's, up to the form's ("" over a component without one, or
//! outside the window) — which the form OnHint was bound on last hears
//! when it changes ([`KernelEvent::Hint`]); and a component whose ShowHint
//! is on (its own, else its parent's, as the VCL's ParentShowHint) shows
//! the short part in a tooltip under the mouse once it rests there
//! (Application.HintPause, 500 ms), until HintHidePause (2.5 s) or the
//! mouse moves to another component; one after another, the next shows
//! after HintShortPause. A press or a key takes the tooltip away.

use std::time::Duration;

use rapidr_value::hints::{long_hint, short_hint};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::Place;
use rapidr_value::Value;

use crate::input::KernelEvent;
use crate::paint::Painter;
use crate::store::{self, Store};
use crate::text::TextSystem;
use crate::tick::{now, Instant};
use crate::tree::FormUi;

/// How far below the mouse's hot spot the tooltip goes (the arrow's
/// height: the VCL's GetCursorHeightMargin).
const BELOW_CURSOR: f64 = 21.0;
/// The tooltip's padding inside its 1-pixel border.
const PAD_X: i64 = 4;
const PAD_Y: i64 = 2;

/// A form's hint state.
#[derive(Default)]
pub struct HintUi {
    /// What the mouse is over, as last told: a component's id, the form's
    /// own id over its open area; `None` outside the window.
    over: Option<String>,
    /// The component whose tooltip is armed or shown, and its text.
    target: Option<(String, String)>,
    /// When the armed tooltip shows, or the shown one goes.
    pub(crate) wake: Option<Instant>,
    /// The tooltip shown: its text and where its top left is (logical,
    /// the window's inside).
    pub shown: Option<(String, (f64, f64))>,
    /// A tooltip showed on the component before: the next shows sooner.
    recent: bool,
}

fn millis(prop: &str) -> Duration {
    Duration::from_millis(rapidr_value::globals::hint_setting(prop).to_i64().clamp(0, 600_000) as u64)
}

impl FormUi {
    /// A component's Hint, else its parent's, … else the form's.
    fn hint_of(&self, store: &dyn Store, from: Option<usize>) -> String {
        let mut at = from;
        while let Some(i) = at {
            let h = store::string(store, &self.nodes[i].id, "hint");
            if !h.is_empty() {
                return h;
            }
            at = self.nodes[i].parent;
        }
        store::string(store, &self.form, "hint")
    }

    /// Whether node `i` (`None`: the form) shows its hint: its ShowHint,
    /// else its parent's (the VCL's ParentShowHint), the form's False when
    /// never set.
    fn shows_hint(&self, store: &dyn Store, i: Option<usize>) -> bool {
        let mut at = i;
        while let Some(n) = at {
            match store.get(&self.nodes[n].id, "showhint") {
                Value::Null => at = self.nodes[n].parent,
                _ => return store::flag(store, &self.nodes[n].id, "showhint", false),
            }
        }
        store::flag(store, &self.form, "showhint", false)
    }

    /// The mouse moved onto `hit` (`inside`: within the window at all):
    /// the application's hint, and the tooltip armed for it.
    pub(crate) fn hint_hover(&mut self, store: &dyn Store, hit: Option<usize>, inside: bool) {
        let over = inside.then(|| hit.map_or_else(|| self.form.clone(), |i| self.nodes[i].id.clone()));
        if over == self.hint.over {
            return;
        }
        self.hint.over = over;
        let long = if inside { long_hint(&self.hint_of(store, hit)).to_string() } else { String::new() };
        self.events.push(KernelEvent::Hint(long));
        let was_shown = self.hint.shown.take().is_some();
        if was_shown {
            self.dirty = true;
        }
        // (the tooltip's component: the first up the chain that shows hints)
        let mut at = hit;
        while let Some(i) = at {
            if self.shows_hint(store, Some(i)) {
                break;
            }
            at = self.nodes[i].parent;
        }
        let shows = inside && rapidr_value::globals::hint_setting("showhint").to_bool() && (at.is_some() || self.shows_hint(store, None));
        let text = if shows { short_hint(&self.hint_of(store, at)).to_string() } else { String::new() };
        if text.is_empty() {
            self.hint.target = None;
            self.hint.wake = None;
            self.hint.recent = was_shown;
            return;
        }
        let id = at.map_or_else(|| self.form.clone(), |i| self.nodes[i].id.clone());
        let pause = if was_shown || self.hint.recent { millis("hintshortpause") } else { millis("hintpause") };
        self.hint.target = Some((id, text));
        self.hint.wake = Some(now() + pause);
    }

    /// A press or a key: the tooltip goes, and doesn't come back until the
    /// mouse is over another component.
    pub(crate) fn hint_cancel(&mut self) {
        if self.hint.shown.take().is_some() {
            self.dirty = true;
        }
        self.hint.target = None;
        self.hint.wake = None;
        self.hint.recent = false;
    }

    /// The hint's deadline came (tick.rs): the tooltip shows, or goes.
    pub(crate) fn hint_tick(&mut self, at: Instant) {
        if !self.hint.wake.is_some_and(|w| w <= at) {
            return;
        }
        self.hint.wake = None;
        self.dirty = true;
        if self.hint.shown.take().is_some() {
            // (shown long enough: gone until the mouse is over another one)
            self.hint.target = None;
            self.hint.recent = false;
            return;
        }
        let Some((_, text)) = self.hint.target.clone() else { return };
        let (x, y) = self.mouse_at;
        self.hint.shown = Some((text, (x, y + BELOW_CURSOR)));
        self.hint.recent = true;
        self.hint.wake = Some(at + millis("hinthidepause"));
    }

    /// The tooltip shown now, if any: its text.
    pub fn hint_shown(&self) -> Option<&str> {
        self.hint.shown.as_ref().map(|(t, _)| t.as_str())
    }

    /// The tooltip over everything (paint.rs): a pale box with a 1-pixel
    /// border, its text in the system's font, kept inside the window.
    pub(crate) fn paint_hint(&mut self, ts: &mut TextSystem, p: &mut Painter) {
        let Some((text, (x, y))) = self.hint.shown.clone() else { return };
        let font = Font::default();
        let lines: Vec<&str> = text.split(['\r', '\n']).filter(|l| !l.is_empty()).collect();
        let (mut tw, mut th) = (0.0f32, 0.0f32);
        let mut line_h = 0.0f32;
        for l in &lines {
            let (w, h) = ts.measure(l, &font);
            tw = tw.max(w);
            th += h;
            line_h = line_h.max(h);
        }
        let (w, h) = (tw.ceil() as i64 + 2 * PAD_X + 2, th.ceil() as i64 + 2 * PAD_Y + 2);
        let (cw, ch) = (self.client.0, self.client.1 + self.menu_offset);
        let mut left = (x as i64).min(cw - w).max(0);
        let mut top = y as i64;
        if top + h > ch {
            // (no room under the mouse: above it)
            top = (y - BELOW_CURSOR) as i64 - h - 2;
        }
        top = top.max(0);
        left = left.max(0);
        let t = p.theme();
        let fill = match rapidr_value::globals::hint_setting("hintcolor") {
            Value::Null => t.system_color(24),
            v => crate::text::bgr_to_rgb(v.to_i64()),
        };
        let (border, ink) = (t.system_color(23), t.system_color(23));
        p.fill((left, top, w, h), fill);
        p.frame((left, top, w, h), border);
        let mut ly = top + 1 + PAD_Y;
        for l in &lines {
            p.text((left + 1 + PAD_X, ly, w - 2 - PAD_X, line_h.ceil() as i64), l, &font, ink, Place::Left);
            ly += line_h.ceil() as i64;
        }
    }
}
