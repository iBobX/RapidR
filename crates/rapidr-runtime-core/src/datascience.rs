//! RNUM, RDATAFRAME and RPLOT on the desktop (native builds and interpreted
//! programs): the shared model in `rapidr_value::datascience` — the same
//! one the web runs — with what only the desktop does: `PRINT` to stdout,
//! filling a QSTRINGGRID, writing a chart's PNG (`SaveFig`). Charts are
//! drawn by the one renderer every runtime uses (rapidr-ui-render's
//! chart.rs: the UI kernel's ops), so they're the web's pixels too.

use rapidr_value::datascience as ds;

use crate::value::{v_str, Value};

/// What the desktop does for the shared model.
struct Desktop;

impl ds::Host for Desktop {
    fn print(&self, text: &str) {
        crate::builtins::rp_print(&[v_str(text)], true);
    }

    fn warn(&self, text: &str) {
        eprintln!("{text}");
    }

    #[allow(unused_variables)]
    fn to_grid(&self, grid: &str, rows: &[Vec<String>]) {
        #[cfg(feature = "gui")]
        {
            use crate::object::{rp_comp_method, rp_comp_set};
            use crate::value::v_int;
            rp_comp_set(grid, "cols", v_int(rows.first().map_or(0, Vec::len) as i64));
            rp_comp_set(grid, "rowcount", v_int(rows.len() as i64));
            // (the header the fixed row; every column the data's)
            rp_comp_set(grid, "fixedrows", v_int(rows.len().min(1) as i64));
            rp_comp_set(grid, "fixedcols", v_int(0));
            for (r, row) in rows.iter().enumerate() {
                for (c, cell) in row.iter().enumerate() {
                    rp_comp_method(grid, "setcell", &[v_int(c as i64), v_int(r as i64), v_str(cell)]);
                }
            }
        }
    }

    fn save_plot(&self, plot: &str, file: &str, scale: f64) {
        match rapidr_ui_render::chart::png(&ds::plot::state(plot), scale) {
            Ok(bytes) => {
                if let Err(e) = std::fs::write(file, &bytes) {
                    eprintln!("[ERROR] RPlot.savefig: can't write {file}: {e}");
                }
            }
            Err(e) => eprintln!("[ERROR] RPlot.savefig: {e}"),
        }
    }

    // (a desktop chart is shown by Image.LoadFromPlot)
    fn show_plot(&self, _plot: &str) {}
}

pub fn num_method(name: &str, method: &str, args: &[Value]) -> Value {
    ds::num::num_method(name, method, args, &Desktop)
}

pub fn num_get_prop(name: &str, prop: &str) -> Value {
    ds::num::num_get_prop(name, prop)
}

pub fn num_set_prop(name: &str, prop: &str, val: &Value) {
    ds::num::num_set_prop(name, prop, val);
}

pub fn dataframe_method(name: &str, method: &str, args: &[Value]) -> Value {
    ds::frame::dataframe_method(name, method, args, &Desktop)
}

pub fn dataframe_get_prop(name: &str, prop: &str) -> Value {
    ds::frame::dataframe_get_prop(name, prop)
}

pub fn plot_method(name: &str, method: &str, args: &[Value]) -> Value {
    ds::plot::plot_method(name, method, args, &Desktop)
}

pub fn plot_get_prop(name: &str, prop: &str) -> Value {
    ds::plot::plot_get_prop(name, prop)
}

pub fn plot_set_prop(name: &str, prop: &str, val: &Value) {
    ds::plot::plot_set_prop(name, prop, val);
}
