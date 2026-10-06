//! QTRACKBAR: the shared model (`rapidr_value::objects::trackbar`) does
//! everything — its shapes, a press beside the thumb (a page toward it) or
//! on it (a drag), the keys (arrows, Page Up / Down, Home / End) — as the
//! web runtime routes it; OnChange when the user moved it. A fluent theme
//! draws it from the model's parts ([`TrackBar::parts`]): a thin rounded
//! rail, the accent up to the thumb, a round thumb with the accent in it.
//!
//! [`TrackBar::parts`]: rapidr_value::objects::trackbar::TrackBar::parts

use rapidr_value::objects::a11y::{AccessNode, Action};
use rapidr_value::objects::{with_trackbar, with_trackbar_mut};
use rapidr_value::v_int;

use super::{ComponentKind, Cx, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::a11y::AccessValue;
use crate::input::Clipboard;
use crate::paint::Painter;

pub struct Trackbar;

/// A fluent theme's track bar from the model's parts.
fn paint_fluent(cx: &Cx, p: &mut Painter, parts: &rapidr_value::objects::trackbar::Parts) {
    let t = p.theme();
    let s = cx.state;
    let vertical = parts.vertical;
    // (along, across) → the control's (x, y)
    let rect = |a0: f64, c0: f64, a1: f64, c1: f64| {
        let (a0, a1) = (a0.min(a1).round() as i64, a0.max(a1).round() as i64);
        let (c0, c1) = (c0.round() as i64, c1.round() as i64);
        if vertical { (c0, a0, c1 - c0, a1 - a0) } else { (a0, c0, a1 - a0, c1 - c0) }
    };
    let mid = parts.middle;
    let (rail, value) = if s.enabled { (t.channel, t.accent) } else { (t.slider_disabled, t.gray_text) };
    p.round(rect(parts.channel.0, mid - 2.0, parts.channel.1, mid + 2.0), 2.0, Some(rail), None, 1.0);
    // (the value: from Min to the thumb; a selection range over it)
    let (from, to) = parts.range.unwrap_or((parts.start, parts.thumb));
    if to > from {
        p.round(rect(from, mid - 2.0, to, mid + 2.0), 2.0, Some(value), None, 1.0);
    }
    let tick = if s.enabled { t.ticks } else { t.ticks_disabled };
    for &(a, long) in &parts.ticks {
        let len = if long { 4.0 } else { 3.0 };
        for (from, dir) in [(parts.ticks_before, -1.0), (parts.ticks_after, 1.0)] {
            let Some(c) = from else { continue };
            let (c0, c1) = (c, c + dir * len);
            let pts = if vertical { [(c0, a), (c1, a)] } else { [(a, c0), (a, c1)] };
            p.stroke(&pts, tick, 1.0);
        }
    }
    // (the thumb: a circle on the rail, the accent's dot in it — bigger
    // under the mouse, smaller held)
    let d = 18i64;
    let (ax, ay) = if vertical { (mid, parts.thumb) } else { (parts.thumb, mid) };
    let (x0, y0) = ((ax - d as f64 / 2.0).round() as i64, (ay - d as f64 / 2.0).round() as i64);
    let fill = if t.dark { t.slider_edge } else { t.control };
    p.round((x0, y0, d, d), d as f64 / 2.0, Some(fill), Some(t.border), 1.0);
    let dot = if !s.enabled || cx.ui.dragging { 8 } else if s.hover { 12 } else { 10 };
    let k = (d - dot) / 2;
    p.round((x0 + k, y0 + k, dot, dot), dot as f64 / 2.0, Some(if s.enabled { t.slider } else { t.slider_disabled }), None, 1.0);
    if s.focused {
        let r = t.focus_width as i64 + 1;
        p.focus((x0 - r, y0 - r, d + 2 * r, d + 2 * r));
    }
}

/// The classic thumb pointing down (ticks below), 11 × 24, as Windows'
/// unthemed track bar draws it (RapidQ's capture, tests/visual/rapidq/
/// ranges): `w` white, `l` COLOR_3DLIGHT, `s` the shadow, `k` the dark
/// shadow, `.` the face (a checker of white and the face while disabled).
/// Its row 19 is the channel's lower white line showing through, as there.
const THUMB_DOWN: [&str; 24] = [
    "wwwwwwwwwwk",
    "wllllllllsk",
    "wl.......sk",
    "wl.......sk",
    "wl.......sk",
    "wl.......sk",
    "wl.......sk",
    "wl.......sk",
    "wl.......sk",
    "wl.......sk",
    "wl.......sk",
    "wl.......sk",
    "wl.......sk",
    "wl.......sk",
    "wl.......sk",
    "wl.......sk",
    "wl.......sk",
    "wl.......sk",
    "wl.......sk",
    "wwlwwwwwwsk",
    "  wl...sk  ",
    "   wl.sk   ",
    "    wsk    ",
    "     k     ",
];

/// The thumb's rows pointing up (ticks above) or square (ticks on both
/// sides, or none), from [`THUMB_DOWN`]: lit from the top left still.
fn thumb_rows(marks: i64, ticked: bool) -> Vec<String> {
    let body = |r: &str| r.to_string();
    if !ticked || marks == 2 {
        let mut rows: Vec<String> = THUMB_DOWN[..19].iter().map(|r| body(r)).collect();
        rows.extend(["wl.......sk".to_string(), "wl.......sk".to_string(), "wl.......sk".to_string(), "wsssssssssk".to_string(), "kkkkkkkkkkk".to_string()]);
        return rows;
    }
    if marks == 1 {
        // (upside down: the point's lit side stays lit, the flat end is the
        // shaded bottom)
        let mut rows: Vec<String> = THUMB_DOWN.iter().rev().map(|r| body(r)).collect();
        let n = rows.len();
        rows[n - 1] = "kkkkkkkkkkk".into();
        rows[n - 2] = "wsssssssssk".into();
        rows[4] = "wl.......sk".into();
        return rows;
    }
    THUMB_DOWN.iter().map(|r| body(r)).collect()
}

/// The classic look (Windows' unthemed msctls_trackbar32 with a selection
/// range, as RapidQ's TTrackBar makes it): a sunken white channel 18 pixels
/// across, 8 pixels in from the ends; the raised grey thumb pointing at the
/// ticks; the ticks 3 pixels long (4 the first and last); the focus a
/// dotted frame round it. Pixel for pixel at 1×; the same pixels, crisp,
/// at a high-DPI screen's scale.
fn paint_classic(cx: &Cx, p: &mut Painter, bar: &rapidr_value::objects::trackbar::TrackBar) {
    let (w, h) = (cx.width(), cx.height());
    let t = p.theme();
    let vertical = bar.vertical();
    let (len, _) = if vertical { (h, w) } else { (w, h) };
    // (along, across) → the control's (x, y)
    let at = |a: i64, c: i64| if vertical { (c, a) } else { (a, c) };
    let rect = |a: i64, c: i64, la: i64, lc: i64| if vertical { (c, a, lc, la) } else { (a, c, la, lc) };
    let ticked = bar.tick_style != 0;
    let marks = bar.tick_marks;
    // (the thumb's band: below the ticks when they're above it)
    let c0 = if ticked && matches!(marks, 1 | 2) { 8 } else { 2 };
    let (lo, hi) = (bar.min.min(bar.max), bar.max.max(bar.min));
    let span = (hi - lo).max(1) as f64;
    let usable = (len - 27).max(0) as f64;
    let along = |v: i64| ((v.clamp(lo, hi) - lo) as f64 * usable / span).round() as i64;
    // The channel.
    let (ca, cl) = (8, (len - 16).max(0));
    let ch = rect(ca, c0 + 2, cl, 18);
    p.fill(ch, t.window);
    p.sunken_edge(ch);
    if bar.sel_end > bar.sel_start {
        let (s0, s1) = (along(bar.sel_start) + 13, along(bar.sel_end) + 13);
        p.fill(rect(s0, c0 + 4, (s1 - s0 + 1).max(1), 14), if cx.state.enabled { t.highlight } else { t.shadow });
    }
    // The ticks.
    if ticked {
        let ticks = bar.tick_positions();
        let n = ticks.len();
        for (i, v) in ticks.iter().enumerate() {
            let a = 13 + along(*v);
            let long = i == 0 || i + 1 == n;
            let l = if long { 4 } else { 3 };
            if matches!(marks, 0 | 2) {
                p.fill(rect(a, c0 + 25, 1, l), t.text);
            }
            if matches!(marks, 1 | 2) {
                p.fill(rect(a, c0 - 2 - l + 1, 1, l), t.text);
            }
        }
    }
    // The thumb.
    let a0 = 8 + along(bar.position);
    let rows = thumb_rows(marks, ticked);
    let enabled = cx.state.enabled;
    for (j, row) in rows.iter().enumerate() {
        let c = c0 + j as i64;
        let colors: Vec<Option<u32>> = row
            .chars()
            .enumerate()
            .map(|(i, ch)| match ch {
                'w' => Some(t.light),
                'l' => Some(t.light3d),
                's' => Some(t.shadow),
                'k' => Some(t.dark_shadow),
                '.' if !enabled => {
                    let (x, y) = at(a0 + i as i64, c);
                    Some(if (x + y) % 2 == 1 { t.light } else { t.face })
                }
                '.' => Some(t.face),
                _ => None,
            })
            .collect();
        // (a row's runs of one colour as one fill)
        let mut i = 0;
        while i < colors.len() {
            let start = i;
            while i < colors.len() && colors[i] == colors[start] {
                i += 1;
            }
            if let Some(color) = colors[start] {
                p.fill(rect(a0 + start as i64, c, (i - start) as i64, 1), color);
            }
        }
    }
    if cx.state.focused {
        // (one pixel, as Windows at 100 %: the VM's 200 % screen doubles
        // every focus rectangle's border, buttons' too)
        p.focus((0, 0, w, h));
    }
}

impl ComponentKind for Trackbar {
    fn name(&self) -> &'static str {
        "RTRACKBAR"
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        if p.fluent() {
            if let Some(parts) = with_trackbar(cx.id, |t| t.parts(w as f64, h as f64)) {
                paint_fluent(cx, p, &parts);
            }
            return;
        }
        if let Some(bar) = with_trackbar(cx.id, Clone::clone) {
            paint_classic(cx, p, &bar);
        }
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let (w, h) = (cx.width() as f64, cx.height() as f64);
        match m.kind {
            MouseKind::Down => {
                let (drag, changed) = with_trackbar_mut(cx.id, |t| t.mouse_down(m.x, m.y, w, h)).unwrap_or_default();
                cx.ui.dragging = drag;
                if changed {
                    cx.change();
                }
            }
            MouseKind::Move if m.captured && cx.ui.dragging => {
                if with_trackbar_mut(cx.id, |t| t.drag(m.x, m.y, w, h)).unwrap_or(false) {
                    cx.change();
                }
            }
            MouseKind::Up => cx.ui.dragging = false,
            _ => {}
        }
        MouseOut::default()
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, _clip: &mut dyn Clipboard) -> bool {
        if !(33..=40).contains(&k.vk) || k.mods.alt || k.mods.command {
            return false;
        }
        if with_trackbar_mut(cx.id, |t| t.key(k.vk)).unwrap_or(false) {
            cx.change();
        }
        true
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        super::shared_describe(cx, self.name())
    }

    fn access(&self, cx: &mut Cx, action: Action, _part: Option<usize>, value: Option<&AccessValue>) -> bool {
        let changed = match (action, value) {
            // (as the arrow keys would)
            (Action::Increment, _) => with_trackbar_mut(cx.id, |t| t.key(39)),
            (Action::Decrement, _) => with_trackbar_mut(cx.id, |t| t.key(37)),
            (Action::SetValue, Some(AccessValue::Number(v))) => with_trackbar_mut(cx.id, |t| {
                let before = t.position;
                t.set("position", &v_int(v.round() as i64));
                t.position != before
            }),
            _ => return false,
        };
        if changed.unwrap_or(false) {
            cx.change();
        }
        true
    }
}
