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
//! Bound to a data set (`DataSource`, `Kind`, `X`, `Y`: `rapidr_value::
//! datascience::link::chart`) it draws the data set's rows, drawn again as
//! they change — in RapidR Studio's designer too.
//!
//! It never takes the focus. A screen reader hears an image named by the
//! chart's Title (or its AccessibleName, Hint).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use rapidr_value::datascience::{chart, link, plot};
use rapidr_value::i18n::tr_ui;
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
    model_ops(id, plot::state(id), size, theme)
}

/// (RapidR Studio's designer) What a chart with no data yet shows while its
/// form is designed — its data come when the program runs: sample bars
/// under its Title (or "Sample data"), so the designer shows a chart, as
/// other designers do.
fn design_sample(title: &str) -> plot::Plot {
    let mut p = plot::Plot { title: if title.trim().is_empty() { tr_ui("Sample data") } else { title.to_string() }, ..plot::Plot::default() };
    p.series.push(plot::Series {
        x: (0..5).map(f64::from).collect(),
        y: vec![4.0, 7.0, 3.0, 8.0, 5.0],
        label: String::new(),
        color: plot::PALETTE[0].into(),
        style: "bar".into(),
        categories: ["A", "B", "C", "D", "E"].map(String::from).to_vec(),
    });
    p
}

fn model_ops(id: &str, model: plot::Plot, size: (i64, i64), theme: &'static Theme) -> Rc<Vec<Op>> {
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
        let mut model = plot::state(cx.id);
        if let Some(d) = link::chart(cx.id) {
            // (bound to a data set: its rows, live — the designer's too;
            // the plot's own titles and labels kept, the columns' names
            // where it has none)
            model.series = d.series;
            if model.title.is_empty() {
                model.title = cx.store.get(cx.id, "title").to_string_val();
            }
            if model.xlabel.is_empty() {
                model.xlabel = d.xlabel;
            }
            if model.ylabel.is_empty() {
                model.ylabel = d.ylabel;
            }
            model.legend |= d.legend;
        } else if model.series.is_empty() && super::design::designed(cx.id) {
            model = design_sample(&cx.store.get(cx.id, "title").to_string_val());
        }
        let ops = model_ops(cx.id, model, (w, h), p.theme());
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

    /// A bar chart's category names are fitted in logical units: the same
    /// 8 bars in the same rectangle show the same names (and the same
    /// everything else) whatever the screen's scale — 1×, 1.5×, 2×, 3×.
    #[test]
    fn the_names_shown_are_the_same_at_every_scale() {
        plot_method("sw1", "bar", &[v_str("Alpha,Bravo,Charlie,Delta,Echo,Foxtrot,Golf,Hotel"), v_str("10,13,16,19,22,25,28,31"), v_str("v"), Value::Null], &Quiet);
        let mut ts = TextSystem::new();
        let mut shown = Vec::new();
        for w in (150..=500).step_by(10) {
            let mut s = form_with_plot("sw1");
            s.set("sw1", "width", v_int(w));
            let mut at_1 = Vec::new();
            for scale in [1.0, 1.5, 2.0, 3.0] {
                let mut f = FormUi::build(&s, "f", true);
                let t = texts(&f.paint(&s, &mut ts, scale));
                if at_1.is_empty() {
                    at_1 = t;
                } else {
                    assert_eq!(at_1, t, "width {w}: scale {scale}");
                }
            }
            shown.push((w, at_1.iter().filter(|t| t.len() > 3 && t.chars().all(char::is_alphabetic)).count()));
        }
        // (and it is not nothing: all eight when there is room, fewer in a narrow one)
        assert_eq!(shown.last().map(|s| s.1), Some(8), "{shown:?}");
        assert!(shown.first().is_some_and(|s| s.1 < 8), "{shown:?}");
    }

    #[test]
    fn sample_data_while_designed() {
        // (on a design surface — `surface:name` — a chart without data
        // shows sample bars under its Title; a running one stays empty)
        let mut s = form_with_plot("ds1:plot1");
        s.set("ds1:plot1", "title", v_str("Sales by month"));
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(&s, "f", true);
        let t = texts(&f.paint(&s, &mut ts, 1.0));
        for want in ["Sales by month", "A", "E"] {
            assert!(t.contains(&want.to_string()), "{t:?}");
        }
        let s = form_with_plot("kp3");
        let mut f = FormUi::build(&s, "f", true);
        assert!(!texts(&f.paint(&s, &mut ts, 1.0)).contains(&"A".to_string()));
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
