//! QTRACKBAR: its range, position, ticks and selection range (RapidQ
//! manual, Appendix A), how it's laid out and what the keys and the mouse do
//! to it, shared by the desktop and web runtimes, which draw the same
//! [`TrackBar::shapes`].

use crate::{v_int, Value};

#[derive(Clone, Debug)]
pub struct TrackBar {
    pub min: i64,
    pub max: i64,
    pub position: i64,
    /// The step between automatic tick marks.
    pub frequency: i64,
    pub line_size: i64,
    pub page_size: i64,
    /// 0 tbHorizontal, 1 tbVertical.
    pub orientation: i64,
    /// 0 tmBottomRight, 1 tmTopLeft, 2 tmBoth.
    pub tick_marks: i64,
    /// 0 tsNone, 1 tsAuto, 2 tsManual.
    pub tick_style: i64,
    pub sel_start: i64,
    pub sel_end: i64,
    /// Ticks added with SetTick (tsManual shows them).
    pub ticks: Vec<i64>,
    /// Goes up whenever it must be drawn again.
    pub revision: u64,
}

impl Default for TrackBar {
    fn default() -> Self {
        TrackBar {
            min: 0,
            max: 10,
            position: 0,
            frequency: 1,
            line_size: 1,
            page_size: 2,
            orientation: 0,
            tick_marks: 0,
            tick_style: 1,
            sel_start: 0,
            sel_end: 0,
            ticks: Vec::new(),
            revision: 0,
        }
    }
}

/// A shape to draw: a closed polygon (or a line with two points), its fill
/// and its outline as 0xRRGGBB.
#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    pub points: Vec<(f64, f64)>,
    pub fill: Option<u32>,
    pub stroke: Option<u32>,
}

/// A track bar's parts ([`TrackBar::parts`]), in its pixels: along the
/// bar (x for a horizontal one), across it (y).
#[derive(Clone, Debug, PartialEq)]
pub struct Parts {
    pub vertical: bool,
    /// The channel's ends along the bar, and its middle across it.
    pub channel: (f64, f64),
    pub middle: f64,
    /// Where Min is along the bar (a value is drawn from it).
    pub start: f64,
    /// The thumb's middle along the bar; its edges across it.
    pub thumb: f64,
    pub thumb_across: (f64, f64),
    /// The selection range's ends (SelStart, SelEnd).
    pub range: Option<(f64, f64)>,
    /// The ticks along the bar (a pixel's middle) and whether each is a
    /// long one (the first, the last).
    pub ticks: Vec<(f64, bool)>,
    /// Where the ticks before the thumb (above / left) start across the
    /// bar, going outward; and those after it.
    pub ticks_before: Option<f64>,
    pub ticks_after: Option<f64>,
}

/// Most ticks drawn (a huge range with Frequency 1 draws no more).
const MAX_TICKS: usize = 1000;
/// From the ends to the first and last positions (along the bar).
const INSET: f64 = 13.0;
/// The thumb's length along the bar and its thickness across it.
const THUMB_LEN: f64 = 11.0;
const THUMB_THICK: f64 = 20.0;
/// Room for the top / left ticks.
const TICK_ROOM: f64 = 7.0;

impl TrackBar {
    fn changed(&mut self) {
        self.revision += 1;
    }

    fn lo(&self) -> i64 {
        self.min.min(self.max)
    }

    fn hi(&self) -> i64 {
        self.max.max(self.min)
    }

    fn clamp(&self, p: i64) -> i64 {
        p.clamp(self.lo(), self.hi())
    }

    pub fn vertical(&self) -> bool {
        self.orientation == 1
    }

    /// The positions with a tick mark.
    pub fn tick_positions(&self) -> Vec<i64> {
        let (lo, hi) = (self.lo(), self.hi());
        let mut out = match self.tick_style {
            0 => return Vec::new(),
            1 => {
                let step = self.frequency.max(1);
                let mut v: Vec<i64> = (0..).map(|i| lo.saturating_add(i * step)).take_while(|p| *p < hi).take(MAX_TICKS).collect();
                v.push(hi);
                v
            }
            _ => {
                let mut v = vec![lo, hi];
                v.extend(self.ticks.iter().copied().filter(|t| (lo..=hi).contains(t)));
                v
            }
        };
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Where a position is along a bar `len` long (its pixel).
    fn along(&self, p: i64, len: f64) -> f64 {
        let span = (self.hi() - self.lo()) as f64;
        let usable = (len - 2.0 * INSET).max(0.0);
        if span <= 0.0 {
            return INSET;
        }
        INSET + (p - self.lo()) as f64 * usable / span
    }

    /// The position nearest a pixel along the bar.
    fn position_at(&self, at: f64, len: f64) -> i64 {
        let span = (self.hi() - self.lo()) as f64;
        let usable = (len - 2.0 * INSET).max(1.0);
        self.clamp(self.lo() + ((at - INSET) * span / usable).round() as i64)
    }

    /// Where the thumb starts across the bar.
    fn thumb_across(&self) -> f64 {
        4.0 + if self.tick_style != 0 && matches!(self.tick_marks, 1 | 2) { TICK_ROOM } else { 0.0 }
    }

    /// The control's length along the bar and across it.
    fn axes(&self, w: f64, h: f64) -> (f64, f64) {
        if self.vertical() { (h, w) } else { (w, h) }
    }

    /// What to draw in a `w` × `h` control: the channel, the selection
    /// range, the tick marks and the thumb (pointing at the ticks, as
    /// Windows'), in the current theme's colours (`crate::theme`; a fluent
    /// theme draws its own from [`TrackBar::parts`]).
    pub fn shapes(&self, w: f64, h: f64, enabled: bool) -> Vec<Shape> {
        let th = crate::theme::current();
        let (len, _) = self.axes(w, h);
        let vertical = self.vertical();
        let xy = |a: f64, c: f64| if vertical { (c, a) } else { (a, c) };
        let rect = |a0: f64, c0: f64, a1: f64, c1: f64| vec![xy(a0, c0), xy(a1, c0), xy(a1, c1), xy(a0, c1)];
        let t0 = self.thumb_across();
        let mut out = Vec::new();
        // The channel.
        let mid = t0 + THUMB_THICK / 2.0;
        out.push(Shape { points: rect(INSET - 5.0, mid - 2.0, len - INSET + 5.0, mid + 2.0), fill: Some(th.channel), stroke: Some(th.channel_edge) });
        // The selection range.
        if self.sel_end > self.sel_start {
            let (a, b) = (self.along(self.clamp(self.sel_start), len), self.along(self.clamp(self.sel_end), len));
            out.push(Shape { points: rect(a, mid - 1.0, b, mid + 1.0), fill: Some(if enabled { th.highlight } else { th.channel_edge }), stroke: None });
        }
        // The ticks (the first and last a bit longer).
        let ticks = self.tick_positions();
        let tick = if enabled { th.ticks } else { th.ticks_disabled };
        for (i, p) in ticks.iter().enumerate() {
            let a = self.along(*p, len).round() + 0.5;
            let long = if i == 0 || i + 1 == ticks.len() { 1.0 } else { 0.0 };
            if matches!(self.tick_marks, 0 | 2) {
                let c = t0 + THUMB_THICK + 2.0;
                out.push(Shape { points: vec![xy(a, c), xy(a, c + 3.0 + long)], fill: None, stroke: Some(tick) });
            }
            if matches!(self.tick_marks, 1 | 2) {
                let c = t0 - 2.0;
                out.push(Shape { points: vec![xy(a, c), xy(a, c - 3.0 - long)], fill: None, stroke: Some(tick) });
            }
        }
        // The thumb.
        let c = self.along(self.clamp(self.position), len).round();
        let (a0, a1) = (c - THUMB_LEN / 2.0, c + THUMB_LEN / 2.0);
        let (c0, c1) = (t0, t0 + THUMB_THICK);
        let point = 5.0;
        let thumb = match self.tick_marks {
            0 => vec![xy(a0, c0), xy(a1, c0), xy(a1, c1 - point), xy(c, c1), xy(a0, c1 - point)],
            1 => vec![xy(c, c0), xy(a1, c0 + point), xy(a1, c1), xy(a0, c1), xy(a0, c0 + point)],
            _ => rect(a0, c0, a1, c1),
        };
        out.push(Shape { points: thumb, fill: Some(if enabled { th.slider } else { th.slider_disabled }), stroke: Some(if enabled { th.slider_edge } else { th.channel_edge }) });
        out
    }

    /// Where a `w` × `h` control's parts are, for a theme that draws its
    /// own (the fluent ones): the same places [`TrackBar::shapes`] draws
    /// them, which the mouse finds.
    pub fn parts(&self, w: f64, h: f64) -> Parts {
        let (len, _) = self.axes(w, h);
        let t0 = self.thumb_across();
        let ticks = self.tick_positions();
        let n = ticks.len();
        Parts {
            vertical: self.vertical(),
            channel: (INSET - 5.0, len - INSET + 5.0),
            middle: t0 + THUMB_THICK / 2.0,
            start: self.along(self.lo(), len),
            thumb: self.along(self.clamp(self.position), len).round(),
            thumb_across: (t0, t0 + THUMB_THICK),
            range: (self.sel_end > self.sel_start).then(|| (self.along(self.clamp(self.sel_start), len), self.along(self.clamp(self.sel_end), len))),
            ticks: ticks.iter().enumerate().map(|(i, p)| (self.along(*p, len).round() + 0.5, i == 0 || i + 1 == n)).collect(),
            ticks_before: matches!(self.tick_marks, 1 | 2).then_some(t0 - 2.0),
            ticks_after: matches!(self.tick_marks, 0 | 2).then_some(t0 + THUMB_THICK + 2.0),
        }
    }

    /// The shapes as an SVG document (the web shows it).
    pub fn svg(&self, w: f64, h: f64, enabled: bool) -> String {
        let mut s = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">");
        for shape in self.shapes(w, h, enabled) {
            let pts: Vec<String> = shape.points.iter().map(|(x, y)| format!("{x},{y}")).collect();
            let fill = shape.fill.map_or("none".to_string(), |c| format!("#{c:06x}"));
            let stroke = shape.stroke.map_or("none".to_string(), |c| format!("#{c:06x}"));
            let tag = if shape.points.len() == 2 { "polyline" } else { "polygon" };
            // (outlines on the pixel grid: half a pixel in)
            s.push_str(&format!("<{tag} points=\"{}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1\" shape-rendering=\"crispEdges\"/>", pts.join(" ")));
        }
        s.push_str("</svg>");
        s
    }

    /// A key pressed on it (its virtual key code): the arrows move by
    /// LineSize, Page Up / Page Down by PageSize, Home / End to the ends.
    /// Whether the position changed.
    pub fn key(&mut self, vk: i64) -> bool {
        let p = self.position;
        let to = match vk {
            37 | 38 => p - self.line_size,
            39 | 40 => p + self.line_size,
            33 => p - self.page_size,
            34 => p + self.page_size,
            36 => self.lo(),
            35 => self.hi(),
            _ => return false,
        };
        self.move_to(to)
    }

    fn move_to(&mut self, p: i64) -> bool {
        let p = self.clamp(p);
        if p == self.position {
            return false;
        }
        self.position = p;
        self.changed();
        true
    }

    /// The mouse pressed at (x, y) in a `w` × `h` control: on the thumb it
    /// starts dragging it (true); elsewhere the position moves a page
    /// toward the click (the second value: whether it changed).
    pub fn mouse_down(&mut self, x: f64, y: f64, w: f64, h: f64) -> (bool, bool) {
        let (len, _) = self.axes(w, h);
        let at = if self.vertical() { y } else { x };
        let c = self.along(self.clamp(self.position), len);
        if (at - c).abs() <= THUMB_LEN / 2.0 + 1.0 {
            return (true, false);
        }
        let p = self.position_at(at, len);
        let to = if p > self.position { (self.position + self.page_size).min(p) } else { (self.position - self.page_size).max(p) };
        (false, self.move_to(to))
    }

    /// The thumb dragged to (x, y): whether the position changed.
    pub fn drag(&mut self, x: f64, y: f64, w: f64, h: f64) -> bool {
        let (len, _) = self.axes(w, h);
        let at = if self.vertical() { y } else { x };
        let p = self.position_at(at, len);
        self.move_to(p)
    }

    pub fn get(&self, prop: &str) -> Option<Value> {
        Some(v_int(match prop {
            "min" => self.min,
            "max" => self.max,
            "position" => self.clamp(self.position),
            "frequency" => self.frequency,
            "linesize" => self.line_size,
            "pagesize" => self.page_size,
            "orientation" => self.orientation,
            "tickmarks" => self.tick_marks,
            "tickstyle" => self.tick_style,
            "selstart" => self.sel_start,
            "selend" => self.sel_end,
            _ => return None,
        }))
    }

    pub fn set(&mut self, prop: &str, val: &Value) -> bool {
        let n = val.to_i64();
        match prop {
            "min" => self.min = n,
            "max" => self.max = n,
            "position" => self.position = n,
            "frequency" => self.frequency = n.max(1),
            "linesize" => self.line_size = n,
            "pagesize" => self.page_size = n,
            "orientation" => self.orientation = n.clamp(0, 1),
            "tickmarks" => self.tick_marks = n.clamp(0, 2),
            "tickstyle" => self.tick_style = n.clamp(0, 2),
            "selstart" => self.sel_start = n,
            "selend" => self.sel_end = n,
            _ => return false,
        }
        // (the range changed: the position stays inside it)
        self.position = self.clamp(self.position);
        self.changed();
        true
    }

    /// SetTick(Pos) (and ClearTicks); `None` for other methods.
    pub fn call(&mut self, method: &str, args: &[Value]) -> Option<Value> {
        match method {
            "settick" => {
                let p = args.first().map_or(0, Value::to_i64);
                if !self.ticks.contains(&p) && self.ticks.len() < MAX_TICKS {
                    self.ticks.push(p);
                }
            }
            "clearticks" => self.ticks.clear(),
            _ => return None,
        }
        self.changed();
        Some(Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rapidq_defaults_ticks_and_keys() {
        let mut t = TrackBar::default();
        assert_eq!((t.min, t.max, t.page_size, t.line_size), (0, 10, 2, 1));
        assert_eq!(t.tick_positions().len(), 11);
        t.set("frequency", &v_int(4));
        assert_eq!(t.tick_positions(), vec![0, 4, 8, 10]);
        t.set("tickstyle", &v_int(2));
        t.call("settick", &[v_int(3)]);
        assert_eq!(t.tick_positions(), vec![0, 3, 10]);
        assert!(t.key(34));
        assert_eq!(t.position, 2);
        assert!(t.key(35) && t.position == 10);
        assert!(!t.key(39));
        t.set("max", &v_int(5));
        assert_eq!(t.get("position").unwrap().to_i64(), 5);
        // a click right of the thumb moves a page toward it; on it, a drag
        t.set("position", &v_int(0));
        let (drag, moved) = t.mouse_down(150.0, 10.0, 200.0, 45.0);
        assert!(!drag && moved && t.position == 2);
        let x = t.along(2, 200.0);
        assert_eq!(t.mouse_down(x, 10.0, 200.0, 45.0), (true, false));
        assert!(t.drag(200.0 - INSET, 10.0, 200.0, 45.0) && t.position == 5);
        assert!(t.svg(200.0, 45.0, true).starts_with("<svg"));
    }
}
