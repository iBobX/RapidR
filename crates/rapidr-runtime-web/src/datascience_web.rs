//! RNUM, RDATAFRAME and RPLOT in the browser: the shared model in
//! `rapidr_value::datascience` — the same one native builds and the
//! interpreter run — with what only the page does: `PRINT` to the page's
//! output, filling a QSTRINGGRID, keeping a chart's PNG among the page's
//! files (`SaveFig`), drawing an RPLOT's form again when its chart
//! changes. Charts are drawn by the one renderer every runtime uses (the UI
//! kernel's ops: an RPLOT on a form is a kernel component, as on the
//! desktop) — the desktop's pixels.

use crate::object_web;
use crate::value::{v_int, v_str, Value};
use rapidr_value::datascience as ds;

/// What the page does for the shared model.
struct Web;

impl ds::Host for Web {
    fn print(&self, text: &str) {
        crate::builtins::rp_print(&[v_str(text)], true);
    }

    fn warn(&self, text: &str) {
        web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(text));
    }

    fn to_grid(&self, grid: &str, rows: &[Vec<String>]) {
        let grid = grid.to_uppercase();
        object_web::rp_comp_set(&grid, "cols", v_int(rows.first().map_or(0, Vec::len) as i64));
        object_web::rp_comp_set(&grid, "rowcount", v_int(rows.len() as i64));
        // (the header the fixed row; every column the data's)
        object_web::rp_comp_set(&grid, "fixedrows", v_int(rows.len().min(1) as i64));
        object_web::rp_comp_set(&grid, "fixedcols", v_int(0));
        for (r, row) in rows.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                object_web::rp_comp_method(&grid, "setcell", &[v_int(c as i64), v_int(r as i64), v_str(cell)]);
            }
        }
    }

    // (a PNG among the page's files: the program reads it back, offers it
    // as a download …)
    fn save_plot(&self, plot: &str, file: &str, scale: f64) {
        object_web::install_file_hooks();
        let saved = rapidr_ui_render::chart::png(&ds::plot::state(plot), scale).and_then(|png| object_web::web_write_file(file, &png));
        if let Err(e) = saved {
            web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!("[ERROR] RPlot.savefig: {file}: {e}")));
        }
    }

    // (an RPLOT on a form is the UI kernel's: its form drawn again)
    fn show_plot(&self, _plot: &str) {
        crate::kernel_web::redraw();
    }
}

pub fn num_method(name: &str, method: &str, args: &[Value]) -> Value {
    ds::num::num_method(name, method, args, &Web)
}

pub fn num_get_prop(name: &str, prop: &str) -> Value {
    ds::num::num_get_prop(name, prop)
}

pub fn num_set_prop(name: &str, prop: &str, val: &Value) {
    ds::num::num_set_prop(name, prop, val);
}

pub fn dataframe_method(name: &str, method: &str, args: &[Value]) -> Value {
    // (its files are the page's: those the program wrote, the project's)
    object_web::install_file_hooks();
    ds::frame::dataframe_method(name, method, args, &Web)
}

pub fn dataframe_get_prop(name: &str, prop: &str) -> Value {
    ds::frame::dataframe_get_prop(name, prop)
}

/// (whatever a chart's member changed, an RPLOT on a form shows: drawn
/// again)
pub fn plot_method(name: &str, method: &str, args: &[Value]) -> Value {
    let v = ds::plot::plot_method(name, method, args, &Web);
    ds::Host::show_plot(&Web, name);
    v
}

pub fn plot_get_prop(name: &str, prop: &str) -> Value {
    ds::plot::plot_get_prop(name, prop)
}

pub fn plot_set_prop(name: &str, prop: &str, val: &Value) {
    if ds::plot::plot_set_prop(name, prop, val) {
        ds::Host::show_plot(&Web, name);
    }
}

/// A QIMAGE's picture from chart `plot` (`Image.LoadFromPlot`): its pixels,
/// drawn again at the page's scale for the screen — the desktop's pixels
/// (rapidr-ui-render's chart.rs); the chart's size.
pub fn load_into_picture(picture: &str, plot: &str) -> Option<(i64, i64)> {
    rapidr_ui_render::chart::load_into_picture(picture, plot)
}
