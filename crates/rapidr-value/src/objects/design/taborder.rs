//! The Tab-order editor (docs/studio-wow.md DES-13), Visual Studio's
//! View ▸ Tab Order on the canvas: with TabOrderMode on, every component
//! that takes the focus carries its place in the Tab order — `0`, `1` …
//! in its container, `2.0`, `2.1` inside the third one (Delphi's and VB's
//! TabIndex count from 0). Clicking components in the order wanted
//! renumbers them: the first clicked in a container becomes its 0, the next
//! its 1, the ones not clicked yet keep their order after them. Each click
//! is written at once as `TabOrder = n` in the CREATE blocks whose number
//! changed — the smallest edit, one undo step — by RapidR's rule (the
//! runtimes walk Tab by TabOrder, else creation order:
//! `objects::a11y::tab_walk`). A click on the form starts the count over;
//! Escape (or TabOrderMode off) ends it.

use super::{DesignSurface, NodeId};
use crate::designer::arrange;
use crate::objects::font::Font;
use crate::objects::ops::{Op, Place, Rect};
use crate::objects::text::text_size;

impl DesignSurface {
    /// Whether component `id` takes part in the Tab order (it is on the
    /// form and its type has a TabOrder).
    pub(super) fn tab_stop(&self, id: NodeId) -> bool {
        self.on_form(id) && self.designer.design.node(id).and_then(|n| n.component()).is_some_and(|c| c.property("TabOrder").is_some())
    }

    /// Container `parent`'s children in the Tab order, those that take
    /// part only.
    fn tab_children(&self, parent: NodeId) -> Vec<NodeId> {
        arrange::tab_order(&self.designer.design, parent).into_iter().filter(|&c| self.tab_stop(c)).collect()
    }

    /// Each component's place in the Tab order, as the badges show it
    /// (`"2.1"`), in creation order.
    pub fn tab_numbers(&self) -> Vec<(NodeId, String)> {
        let mut out = Vec::new();
        fn walk(s: &DesignSurface, parent: NodeId, prefix: &str, out: &mut Vec<(NodeId, String)>) {
            for (k, c) in s.tab_children(parent).into_iter().enumerate() {
                let label = format!("{prefix}{k}");
                out.push((c, label.clone()));
                walk(s, c, &format!("{label}."), out);
            }
        }
        walk(self, self.designer.design.root(), "", &mut out);
        out
    }

    /// TabOrderMode on or off.
    pub fn set_tab_order_mode(&mut self, on: bool) {
        if on == self.tab_order.is_some() {
            return;
        }
        self.end_edit(true);
        self.menu_open.clear();
        self.forget_gestures();
        self.tab_order = on.then(Vec::new);
        self.say(if on { "Tab order: click the components in the order Tab should take them; Escape ends" } else { "Tab order done" });
    }

    /// A click in Tab-order mode (client coordinates): the component there
    /// takes the next number in its container (one undo step).
    pub(super) fn tab_order_click(&mut self, x: i64, y: i64) {
        let hit = self.component_at(x, y).and_then(|i| self.ids().get(i).copied());
        // (a component that doesn't take part: its container that does)
        let mut at = hit;
        while let Some(id) = at.filter(|&id| !self.tab_stop(id)) {
            at = self.designer.design.parent(id).filter(|&p| p != self.designer.design.root());
        }
        let Some(id) = at else {
            if let Some(t) = &mut self.tab_order {
                t.clear();
            }
            self.say("Tab order: counting starts over");
            return;
        };
        let Some(parent) = self.designer.design.parent(id) else { return };
        let mut clicked = self.tab_order.take().unwrap_or_default();
        clicked.retain(|&c| c != id && self.designer.design.node(c).is_some());
        clicked.push(id);
        let mine: Vec<NodeId> = clicked.iter().copied().filter(|&c| self.designer.design.parent(c) == Some(parent)).collect();
        let rest: Vec<NodeId> = self.tab_children(parent).into_iter().filter(|c| !mine.contains(c)).collect();
        let order: Vec<NodeId> = mine.iter().copied().chain(rest).collect();
        let _ = self.designer.set_tab_order(parent, &order);
        self.commit();
        self.tab_order = Some(clicked);
        let number = self.tab_numbers().into_iter().find(|(c, _)| *c == id).map(|(_, n)| n).unwrap_or_default();
        let name = self.designer.design.node(id).map(|n| n.name.clone()).unwrap_or_default();
        self.say(format!("{name}: Tab order {number}"));
    }

    /// The badges (surface pixels from the client area's origin): each
    /// component's number at its top left, those clicked in the accent.
    pub(super) fn tab_order_ops(&self, out: &mut Vec<Op>) {
        let Some(clicked) = &self.tab_order else { return };
        let t = crate::theme::current();
        let font = Font { styles: 1, ..Font::default() };
        for (id, label) in self.tab_numbers() {
            let Some(r) = self.rect_of(id) else { continue };
            let (x, y, w, h) = self.view_rect((r.left, r.top, r.width, r.height));
            // (the component faintly framed, its badge over its corner)
            out.push(Op::Edge { rect: (x, y, w, h), light: vec![t.border_strong], dark: vec![t.border_strong] });
            let (tw, th) = text_size(&label, &font);
            let badge: Rect = (x - 2, y - 2, tw + 10, th + 4);
            let on = clicked.contains(&id);
            let (bg, fg) = if on { (t.accent, t.window) } else { (t.window, t.text) };
            out.push(Op::Round { rect: badge, radius: 4.0, fill: Some(bg), stroke: Some(if on { t.accent } else { t.border_strong }), width: 1.0 });
            out.push(Op::Text { rect: badge, text: label, font: font.clone(), color: fg, angle: 0, place: Place::Center });
        }
    }
}
