//! ROUTPUTCONSOLE drawn and driven by the kernel; its model is
//! `rapidr_value::panels::console` (the web draws the same).

use rapidr_value::objects::a11y::{AccessNode, Role};
use rapidr_value::objects::ops::Place;

use super::common::{self, look};
use crate::components::{ComponentKind, Cx};
use crate::paint::Painter;

pub struct Console;

impl ComponentKind for Console {
    fn name(&self) -> &'static str {
        "ROUTPUTCONSOLE"
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let l = look(p.theme());
        let (w, h) = (cx.width(), cx.height());
        common::ground(p, w, h, &l);
        p.text((0, 0, w, h), "Output", &cx.font, l.dim, Place::Center);
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = crate::components::shared_describe(cx, self.name());
        n.role = Role::MultilineTextInput;
        if n.name.is_empty() {
            n.name = "Output".into();
        }
        n
    }
}
