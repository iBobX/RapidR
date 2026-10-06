//! Progress meters, drawn from Min, Max and Position:
//!
//! - QGAUGE (RapidR's RPROGRESSBAR; QPROGRESSBAR too): RapidQ's gauge, as
//!   Delphi's TGauge draws it — BackColor (white) behind, ForeColor
//!   (black) done, BorderStyle bsSingle a black frame, ShowText the
//!   percentage in the middle (in BackColor over the done part); Kind
//!   gkText 0, gkHorizontalBar 1 (the default), gkVerticalBar 2, gkPie 3,
//!   gkNeedle 4.
//! - RPROGRESS (RapidR's): Windows' classic progress bar — a thin sunken
//!   frame, blocks in the highlight colour (Smooth: one bar).
//!
//! A fluent theme draws both rounded: the accent done (a QGAUGE whose
//! colours the program didn't set), on a quiet track.
//!
//! Neither takes the focus or the mouse.

use rapidr_value::objects::a11y::AccessNode;
use rapidr_value::objects::ops::{Op, Place};

use super::radio::oval;
use super::{ComponentKind, Cx};
use crate::paint::Painter;
use crate::store::{self, Store};
use crate::text::bgr_to_rgb;

/// Min, Max, Position (clamped).
pub fn range(store: &dyn Store, id: &str) -> (i64, i64, i64) {
    let min = store::int(store, id, "min", 0);
    let max = store::int(store, id, "max", 100).max(min);
    (min, max, store::int(store, id, "position", 0).clamp(min, max))
}

/// How much is done, in percent (TGauge's PercentDone: whole numbers).
pub fn percent(store: &dyn Store, id: &str) -> i64 {
    let (min, max, pos) = range(store, id);
    if max == min {
        return 0;
    }
    (pos - min) * 100 / (max - min)
}

pub struct Progress;

impl Progress {
    /// QGAUGE (TGauge's look).
    fn gauge(cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        let t = p.theme();
        // (BackColor / ForeColor the program didn't set: the theme's — white
        // and black in the classic look, a track and the accent otherwise)
        let set = |prop: &str| store::int(cx.store, cx.id, prop, -1);
        let back = match set("backcolor") {
            -1 if t.fluent() => t.unfocused,
            -1 => t.window,
            c => bgr_to_rgb(c),
        };
        let fore = match set("forecolor") {
            -1 if t.fluent() => t.accent,
            -1 => t.text,
            c => bgr_to_rgb(c),
        };
        let pct = percent(cx.store, cx.id);
        let mut r = (0, 0, w, h);
        let kind = store::int(cx.store, cx.id, "kind", 1);
        let bordered = store::int(cx.store, cx.id, "borderstyle", 1) != 0;
        if t.fluent() && matches!(kind, 1 | 2) {
            // (rounded: the track, the part done over it, a thin border)
            let radius = t.radius.min((w.min(h) / 2) as f64);
            p.round(r, radius, Some(back), bordered.then_some(t.border), 1.0);
            let done = if kind == 1 { (0, 0, w * pct / 100, h) } else { (0, h - h * pct / 100, w, h * pct / 100) };
            p.round(done, radius, Some(fore), None, 1.0);
            if store::flag(cx.store, cx.id, "showtext", true) {
                let text = format!("{pct}%");
                let ink = t.text_on(back);
                p.text(r, &text, &cx.font, ink, Place::Center);
                if done.2 > 0 && done.3 > 0 {
                    let over = t.text_on(fore);
                    p.clipped(done, |p| p.text(r, &text, &cx.font, over, Place::Center));
                }
            }
            return;
        }
        // (a pie's and a needle's corners show what's behind the gauge, as
        // TGauge leaves them; a bar is BackColor throughout)
        p.fill(r, if matches!(kind, 3 | 4) { crate::paint::behind(cx.store, cx.id) } else { back });
        // (TGauge's frame: black — clWindowFrame on Windows 98)
        let line = if t.fluent() { t.border } else { t.text };
        if bordered {
            p.frame(r, line);
            r = (1, 1, w - 2, h - 2);
        }
        let (x, y, iw, ih) = r;
        // (the done part, where the text is drawn in BackColor)
        let mut done = None;
        // (a pie's done part, for its percentage)
        let mut wedge: Option<Vec<(f64, f64)>> = None;
        match kind {
            1 => {
                let dw = iw * pct / 100;
                done = Some((x, y, dw, ih));
                p.fill((x, y, dw, ih), fore);
            }
            2 => {
                let dh = ih * pct / 100;
                done = Some((x, y + ih - dh, iw, dh));
                p.fill((x, y + ih - dh, iw, dh), fore);
            }
            3 => {
                let (cxp, cyp, rx, ry) = (x as f64 + (iw - 1) as f64 / 2.0, y as f64 + (ih - 1) as f64 / 2.0, (iw - 1) as f64 / 2.0, (ih - 1) as f64 / 2.0);
                p.shape(oval(cxp, cyp, rx, ry, 0.0, 360.0, back));
                let mut outline = oval(cxp, cyp, rx, ry, 0.0, 360.0, 0);
                outline.fill = None;
                outline.stroke = Some(line);
                p.shape(outline);
                if pct > 0 {
                    // (clockwise from twelve o'clock)
                    let part = oval(cxp, cyp, rx, ry, 90.0 - 360.0 * pct as f64 / 100.0, 90.0, fore);
                    wedge = Some(part.points.iter().map(|&(x, y)| (x + 0.5, y + 0.5)).collect());
                    p.shape(part);
                }
            }
            4 => {
                // (a half circle, its needle from the bottom middle)
                let (cxp, cyp, rx, ry) = (x as f64 + (iw - 1) as f64 / 2.0, (y + ih - 1) as f64, (iw - 1) as f64 / 2.0, (ih - 1) as f64);
                let mut arc = oval(cxp, cyp, rx, ry, 0.0, 180.0, back);
                arc.stroke = Some(line);
                p.shape(arc);
                let a = (180.0 - 180.0 * pct as f64 / 100.0).to_radians();
                p.op(Op::Line { from: (cxp + 0.5, cyp + 0.5), to: (cxp + 0.5 + rx * a.cos(), cyp + 0.5 - ry * a.sin()), color: fore });
            }
            _ => {}
        }
        // (the percentage inverts what it's drawn over, as TGauge's: black on
        // white, white on black, yellow on blue; a pie's and a needle's in
        // the middle of the whole gauge)
        if store::flag(cx.store, cx.id, "showtext", true) {
            let text = format!("{pct}%");
            // (fluent: what reads on each)
            let on = |c: u32| if t.fluent() { t.text_on(c) } else { c ^ 0xFFFFFF };
            p.clipped(r, |p| p.text(r, &text, &cx.font, on(back), Place::Center));
            if let Some(d) = done.filter(|d| d.2 > 0 && d.3 > 0) {
                p.clipped(d, |p| p.text(r, &text, &cx.font, on(fore), Place::Center));
            }
            if let Some(points) = wedge {
                p.clipped_polygon(points, |p| p.text(r, &text, &cx.font, on(fore), Place::Center));
            }
        }
    }

    /// RPROGRESS (Windows' classic progress bar).
    fn bar(cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        let t = p.theme();
        if t.fluent() {
            // (rounded and smooth: the accent done on a quiet track)
            let radius = t.radius.min((h / 2) as f64);
            p.round((0, 0, w, h), radius, Some(t.unfocused), Some(t.border), 1.0);
            let done = w * percent(cx.store, cx.id) / 100;
            p.round((0, 0, done, h), radius, Some(t.accent), None, 1.0);
            return;
        }
        p.fill((0, 0, w, h), t.face);
        p.thin_sunken((0, 0, w, h));
        let (iw, ih) = (w - 4, h - 4);
        if iw <= 0 || ih <= 0 {
            return;
        }
        let done = iw * percent(cx.store, cx.id) / 100;
        if store::flag(cx.store, cx.id, "smooth", false) {
            p.fill((2, 2, done, ih), t.highlight);
            return;
        }
        // (blocks two thirds of the height wide, two pixels apart)
        let block = (ih * 2 / 3).max(2);
        let mut x = 0;
        while x < done {
            p.fill((2 + x, 2, block.min(iw - x), ih), t.highlight);
            x += block + 2;
        }
    }
}

impl ComponentKind for Progress {
    fn name(&self) -> &'static str {
        "RPROGRESSBAR"
    }

    fn focusable(&self, _store: &dyn Store, _id: &str) -> bool {
        false
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        if cx.store.type_of(cx.id).eq_ignore_ascii_case("RPROGRESS") {
            Self::bar(cx, p);
        } else {
            Self::gauge(cx, p);
        }
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        super::shared_describe(cx, self.name())
    }
}

#[cfg(test)]
mod tests {
    use rapidr_value::{v_int, Value};

    use crate::{FormUi, MemStore, TextSystem};

    #[test]
    fn gauges_and_bars_show_their_position() {
        let mut s = MemStore::new();
        s.add("pgform", "RFORM", None);
        s.add("pgg", "RPROGRESSBAR", Some("pgform")).set("pgg", "position", v_int(25)).set("pgg", "width", v_int(102)).set("pgg", "height", v_int(20));
        s.add("pgb", "RPROGRESS", Some("pgform")).set("pgb", "position", v_int(50)).set("pgb", "top", v_int(40)).set("pgb", "width", v_int(104)).set("pgb", "height", v_int(16));
        assert_eq!(super::percent(&s, "pgg"), 25);
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(&s, "pgform", false);
        let list = f.paint(&s, &mut ts, 1.0).dump();
        // The gauge: a quarter done in black, "25%" over it.
        assert!(list.contains("fill 1,1 25x18 #000000 @0,0"), "{list}");
        assert!(list.contains("\"25%\""), "{list}");
        // The bar: blocks of 8 up to half its inside.
        assert!(list.contains("fill 2,2 8x12 #0078d7 @0,40"), "{list}");
        assert!(!list.contains("fill 52,2"), "{list}");
        let tree = f.access_tree(&s, &mut ts);
        assert_eq!(tree.children[0].value.as_deref(), Some("25%"));
        let _ = Value::Null;
    }
}
