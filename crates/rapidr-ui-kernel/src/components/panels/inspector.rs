//! RPROPERTYINSPECTOR drawn and driven by the kernel; its model is
//! `rapidr_value::panels::inspector` (the web draws the same).

use rapidr_value::objects::a11y::{AccessNode, Role};
use rapidr_value::objects::ops::Place;

use super::common::{self, look};
use crate::components::{ComponentKind, Cx};
use crate::paint::Painter;

pub struct Inspector;

impl ComponentKind for Inspector {
    fn name(&self) -> &'static str {
        "RPROPERTYINSPECTOR"
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let l = look(p.theme());
        let (w, h) = (cx.width(), cx.height());
        common::ground(p, w, h, &l);
        p.text((0, 0, w, h), "Properties", &cx.font, l.dim, Place::Center);
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = crate::components::shared_describe(cx, self.name());
        n.role = Role::Grid;
        if n.name.is_empty() {
            n.name = "Properties".into();
        }
        n
    }
}

/// The focus left it while its in-place editor was open (`super::focus_left`).
pub fn focus_left(_id: &str, _ed: crate::components::list::InPlace) -> Vec<crate::input::KernelEvent> {
    Vec::new()
}
