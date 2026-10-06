//! RPLOT on a form: its chart (`rapidr_value::datascience::plot`, the
//! model every runtime shares) drawn by the one chart renderer
//! (`rapidr_value::datascience::chart`) as the kernel's own vector ops, at
//! the component's size — so the host makes it pixels at the screen's
//! scale (crisp at 1×, 1.5×, 2×) and the desktop and the web show the
//! same pixels. The chart fills the component's rectangle (Left, Top,
//! Width, Height, Align, Anchors: the kernel's layout), in the current
//! theme's colours; it is drawn again whenever the form is (the runtimes
//! ask for that after every change to a chart: a series, a property,
//! Render / Show).
//!
//! It never takes the focus. A screen reader hears an image named by the
//! chart's Title (or its AccessibleName, Hint).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use rapidr_value::datascience::{chart, plot};
use rapidr_value::objects::a11y::AccessNode;
use rapidr_value::objects::ops::Op;
use rapidr_value::theme::Theme;

use super::{ComponentKind, Cx, MouseIn, MouseOut};
use crate::paint::Painter;
use crate::store::Store;

pub struct Plot;

/// A chart's ops as last made: kept while its model, size and theme stay
/// the same (a form is painted again for every caret blink and hover).
struct Drawn {
    plot: plot::Plot,
    size: (i64, i64),
    theme: &'static Theme,
    ops: Rc<Vec<Op>>,
}

thread_local! {
    static DRAWN: RefCell<HashMap<String, Drawn>> = RefCell::new(HashMap::new());
}

/// Chart `id` as ops `size` logical pixels big, in `theme`.
pub fn chart_ops(id: &str, size: (i64, i64), theme: &'static Theme) -> Rc<Vec<Op>> {
    let model = plot::state(id);
    DRAWN.with(|d| {
        let mut d = d.borrow_mut();
        if let Some(c) = d.get(id).filter(|c| c.size == size && std::ptr::eq(c.theme, theme) && c.plot == model) {
            return c.ops.clone();
        }
        let ops = Rc::new(chart::ops_sized(&model, theme, size));
        d.insert(id.to_string(), Drawn { plot: model, size, theme, ops: ops.clone() });
        ops
    })
}

impl ComponentKind for Plot {
    fn name(&self) -> &'static str {
        "RPLOT"
    }

    fn focusable(&self, _store: &dyn Store, _id: &str) -> bool {
        false
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        if w <= 0 || h <= 0 {
            return;
        }
        let ops = chart_ops(cx.id, (w, h), p.theme());
        p.ops(ops.iter().cloned());
    }

    fn mouse(&self, _cx: &mut Cx, _m: &MouseIn) -> MouseOut {
        MouseOut { press: false, focus: Some(false) }
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        super::shared_describe(cx, self.name())
    }
}

#[cfg(test)]
mod tests {
    use rapidr_value::datascience::plot::{plot_method, plot_set_prop};
    use rapidr_value::objects::a11y::Role;
    use rapidr_value::{v_int, v_str, Value};

    use crate::{FormUi, Item, MemStore, Op, TextSystem};

    struct Quiet;
    impl rapidr_value::datascience::Host for Quiet {
        fn print(&self, _: &str) {}
        fn warn(&self, _: &str) {}
        fn to_grid(&self, _: &str, _: &[Vec<String>]) {}
        fn save_plot(&self, _: &str, _: &str, _: f64) {}
        fn show_plot(&self, _: &str) {}
    }

    fn form_with_plot(id: &str) -> MemStore {
        let mut s = MemStore::new();
        s.add("f", "RFORM", None);
        s.set("f", "width", v_int(400)).set("f", "height", v_int(300));
        s.add(id, "RPLOT", Some("f"));
        s.set(id, "left", v_int(10)).set(id, "top", v_int(20)).set(id, "width", v_int(240)).set(id, "height", v_int(160));
        s
    }

    fn texts(list: &crate::DisplayList) -> Vec<String> {
        list.items.iter().filter_map(|i| if let Item::Op { op: Op::Text { text, .. }, .. } = i { Some(text.clone()) } else { None }).collect()
    }

    #[test]
    fn draws_its_chart_at_its_place_and_size() {
        plot_method("kp1", "bar", &[v_str("North,South"), v_str("3,5"), v_str("units"), Value::Null], &Quiet);
        plot_set_prop("kp1", "title", &v_str("Sales"));
        let s = form_with_plot("kp1");
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(&s, "f", true);
        let list = f.paint(&s, &mut ts, 2.0);
        // (its background fills its rectangle, at its place)
        let bg = list.items.iter().find_map(|i| match i {
            Item::Op { origin, op: Op::Fill { rect, .. } } if *origin == (10, 20) => Some(*rect),
            _ => None,
        });
        assert_eq!(bg, Some((0, 0, 240, 160)));
        let t = texts(&list);
        for want in ["Sales", "North", "South"] {
            assert!(t.contains(&want.to_string()), "{t:?}");
        }
        // (a series added: drawn again with it)
        plot_method("kp1", "plot", &[v_str("0,1"), v_str("1,4"), v_str("trend")], &Quiet);
        plot_method("kp1", "legend", &[], &Quiet);
        let list = f.paint(&s, &mut ts, 1.0);
        assert!(texts(&list).contains(&"trend".to_string()));
    }

    #[test]
    fn an_image_named_by_its_title() {
        plot_set_prop("kp2", "title", &v_str("Rainfall"));
        let s = form_with_plot("kp2");
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(&s, "f", true);
        let tree = f.access_tree(&s, &mut ts);
        let node = tree.children.iter().find(|n| n.role == Role::Image).expect("the chart's node");
        assert_eq!(node.name, "Rainfall");
        assert_eq!(node.bounds, (10, 20, 240, 160));
    }
}
