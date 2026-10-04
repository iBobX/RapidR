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
//! Neither takes the focus or the mouse.

use rapidr_value::objects::a11y::AccessNode;
use rapidr_value::objects::ops::{Op, Place};

use super::radio::oval;
use super::{ComponentKind, Cx};
use crate::paint::{Painter, FACE, HIGHLIGHT, LIGHT, SHADOW};
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
        let back = bgr_to_rgb(store::int(cx.store, cx.id, "backcolor", 0xFFFFFF));
        let fore = bgr_to_rgb(store::int(cx.store, cx.id, "forecolor", 0));
        let pct = percent(cx.store, cx.id);
        let mut r = (0, 0, w, h);
        p.fill(r, back);
        if store::int(cx.store, cx.id, "borderstyle", 1) != 0 {
            p.edge(r, &[0x000000], &[0x000000]);
            r = (1, 1, w - 2, h - 2);
        }
        let (x, y, iw, ih) = r;
        let kind = store::int(cx.store, cx.id, "kind", 1);
        // (the done part, where the text is drawn in BackColor)
        let mut done = None;
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
                outline.stroke = Some(0x000000);
                p.shape(outline);
                if pct > 0 {
                    // (clockwise from twelve o'clock)
                    p.shape(oval(cxp, cyp, rx, ry, 90.0 - 360.0 * pct as f64 / 100.0, 90.0, fore));
                }
            }
            4 => {
                // (a half circle, its needle from the bottom middle)
                let (cxp, cyp, rx, ry) = (x as f64 + (iw - 1) as f64 / 2.0, (y + ih - 1) as f64, (iw - 1) as f64 / 2.0, (ih - 1) as f64);
                let mut arc = oval(cxp, cyp, rx, ry, 0.0, 180.0, back);
                arc.stroke = Some(0x000000);
                p.shape(arc);
                let a = (180.0 - 180.0 * pct as f64 / 100.0).to_radians();
                p.op(Op::Line { from: (cxp + 0.5, cyp + 0.5), to: (cxp + 0.5 + rx * a.cos(), cyp + 0.5 - ry * a.sin()), color: fore });
            }
            _ => {}
        }
        if store::flag(cx.store, cx.id, "showtext", true) && matches!(kind, 0..=2) {
            let text = format!("{pct}%");
            p.clipped(r, |p| p.text(r, &text, &cx.font, fore, Place::Center));
            if let Some(d) = done.filter(|d| d.2 > 0 && d.3 > 0) {
                p.clipped(d, |p| p.text(r, &text, &cx.font, back, Place::Center));
            }
        }
    }

    /// RPROGRESS (Windows' classic progress bar).
    fn bar(cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        p.fill((0, 0, w, h), FACE);
        p.edge((0, 0, w, h), &[SHADOW], &[LIGHT]);
        let (iw, ih) = (w - 4, h - 4);
        if iw <= 0 || ih <= 0 {
            return;
        }
        let done = iw * percent(cx.store, cx.id) / 100;
        if store::flag(cx.store, cx.id, "smooth", false) {
            p.fill((2, 2, done, ih), HIGHLIGHT);
            return;
        }
        // (blocks two thirds of the height wide, two pixels apart)
        let block = (ih * 2 / 3).max(2);
        let mut x = 0;
        while x < done {
            p.fill((2 + x, 2, block.min(iw - x), ih), HIGHLIGHT);
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
