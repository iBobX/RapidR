//! QSTATUSBAR (docked at the bottom by its Align, as in RapidQ:
//! runtime-core's layout): its panels left to right, each `Panel(i).Width`
//! pixels (100 by default; the last one takes the rest), each a thin
//! sunken box with its Caption on one line — or its SimpleText in one box
//! when SimplePanel is set or it has no panels; as the FLTK and web
//! runtimes lay it out.

use rapidr_value::objects::a11y::{node_id, AccessNode, Role};
use rapidr_value::objects::ops::{Place, Rect};

use super::{ComponentKind, Cx};
use crate::paint::{Painter, FACE, GRAY_TEXT, LIGHT, SHADOW};
use crate::store::{self, Store};
use crate::text::bgr_to_rgb;

pub struct StatusBar;

/// Its boxes (in its own pixels) and their texts.
pub fn panels(store: &dyn Store, id: &str, w: i64, h: i64) -> Vec<(Rect, String)> {
    let count = store::int(store, id, "panelcount", 0).clamp(0, 256);
    // (a box 2 pixels narrower than its panel; 1 pixel in from the bar's
    // edges)
    let boxed = |x: i64, pw: i64| (x, 2, (pw - 2).max(0), (h - 3).max(0));
    if count == 0 || store::flag(store, id, "simplepanel", false) {
        return vec![(boxed(1, w), store::string(store, id, "simpletext"))];
    }
    let mut out = Vec::new();
    let mut x = 1;
    for i in 0..count {
        let width = store::int(store, id, &format!("panel({i}).width"), 0);
        let pw = if i == count - 1 { (w + 1 - x).max(0) } else if width > 0 { width.min(10_000) } else { 100 };
        out.push((boxed(x, pw), store::string(store, id, &format!("panel({i}).caption"))));
        x += pw;
    }
    out
}

impl ComponentKind for StatusBar {
    fn name(&self) -> &'static str {
        "RSTATUSBAR"
    }

    fn focusable(&self, _store: &dyn Store, _id: &str) -> bool {
        false
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        p.fill((0, 0, w, h), FACE);
        let color = if cx.state.enabled { bgr_to_rgb(cx.font.color) } else { GRAY_TEXT };
        for ((x, y, bw, bh), text) in panels(cx.store, cx.id, w, h) {
            if bw <= 0 || bh <= 0 {
                continue;
            }
            p.edge((x, y, bw, bh), &[SHADOW], &[LIGHT]);
            // (the caption 3 pixels in, cut at the box's inside)
            let inside = (x + 1, y + 1, (bw - 2).max(0), (bh - 2).max(0));
            p.clipped(inside, |p| p.text((x + 3, y, (bw - 6).max(0), bh), &text, &cx.font, color, Place::Left));
        }
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = AccessNode::new(node_id(cx.id), Role::Group);
        n.bounds = cx.rect;
        let (x0, y0, w, h) = cx.rect;
        for (k, ((x, y, bw, bh), text)) in panels(cx.store, cx.id, w, h).into_iter().enumerate() {
            let mut l = AccessNode::new(node_id(&format!("{}.panel({k})", cx.id)), Role::Label);
            l.name = text;
            l.bounds = (x0 + x, y0 + y, bw, bh);
            n.children.push(l);
        }
        n
    }
}
