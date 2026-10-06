//! RTOOLBAR drawn and driven by the kernel; its model is
//! `rapidr_value::panels::toolbar` (the web draws the same).

use rapidr_value::objects::a11y::{AccessNode, Role};

use super::common::look;
use crate::components::{ComponentKind, Cx};
use crate::paint::Painter;

pub struct ToolBar;

impl ComponentKind for ToolBar {
    fn name(&self) -> &'static str {
        "RTOOLBAR"
    }

    fn focusable(&self, _store: &dyn crate::store::Store, _id: &str) -> bool {
        false
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        // (a strip of the form's colour, as the container it was)
        let back = crate::paint::color_of(cx.store, cx.id).unwrap_or(look(p.theme()).chrome);
        p.fill((0, 0, cx.width(), cx.height()), back);
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = crate::components::shared_describe(cx, self.name());
        n.role = Role::Group;
        if n.name.is_empty() {
            n.name = "Toolbar".into();
        }
        n
    }
}
