//! RNUM, RDATAFRAME and RPLOT in the browser — the same implementations the
//! desktop uses (decision D7, docs/ide-plan.md): RNUM and RPLOT are
//! rapidr-value's models (an RPLOT drawn by rapidr-ui-render, crisp at the
//! page's scale), RDATAFRAME is rapidr-frame's polars engine in a wasm
//! module of its own (frame_web.rs). What stays here is the page's side of
//! them: the page's files, PRINT, a QSTRINGGRID to fill, the console.

use rapidr_frame::{Arg, Call, Out, Reply};
use rapidr_value::datascience::{num, plot};
use wasm_bindgen::JsValue;

use crate::object_web;
use crate::value::{v_dbl, v_int, v_null, v_str, Value};

fn warn(msg: &str) {
    web_sys::console::warn_1(&JsValue::from_str(msg));
}

// ---------------------------------------------------------------- RNUM --

pub fn num_method(name: &str, method: &str, args: &[Value]) -> Value {
    match num::method(name, method, args) {
        Some(v) => {
            if num::prints(method) {
                crate::builtins::rp_print(std::slice::from_ref(&v), true);
            }
            v
        }
        None => {
            warn(&format!("[WARN] RNum.{method}() not implemented"));
            v_null()
        }
    }
}

pub fn num_get_prop(name: &str, prop: &str) -> Value {
    num::get_prop(name, prop).unwrap_or_else(v_null)
}

pub fn num_set_prop(name: &str, prop: &str, val: &Value) {
    num::set_prop(name, prop, val);
}

// ---------------------------------------------------------- RDATAFRAME --

/// A new RDATAFRAME (nothing to do: a frame no method filled is empty).
pub fn init_dataframe(_name: &str) {}

fn frame_arg(v: &Value) -> Arg {
    match v {
        Value::Integer(_) | Value::Double(_) | Value::Boolean(_) => Arg::num(v.to_f64(), v.to_string_val()),
        _ => Arg::text(&v.to_string_val()),
    }
}

fn out_value(out: Out) -> Value {
    match out {
        Out::Null => v_null(),
        Out::Int(i) => v_int(i),
        Out::Dbl(d) => v_dbl(d),
        Out::Str(s) => v_str(&s),
    }
}

/// What a reply asks of the page, done; its value.
fn apply(reply: Reply) -> Value {
    if let Some(e) = reply.error {
        web_sys::console::error_1(&JsValue::from_str(&format!("[ERROR] {e}")));
    }
    if let Some(text) = reply.print {
        crate::builtins::rp_print(&[v_str(&text)], true);
    }
    if let Some((path, bytes)) = reply.write {
        if let Err(e) = object_web::web_write_file(&path, &bytes) {
            web_sys::console::error_1(&JsValue::from_str(&format!("[ERROR] RDataFrame: {path}: {e}")));
        }
    }
    if let Some((grid, rows)) = reply.grid {
        let grid = grid.to_uppercase();
        let ncols = rows.first().map(Vec::len).unwrap_or(0);
        object_web::rp_comp_method(&grid, "setcolcount", &[v_int(ncols as i64)]);
        object_web::rp_comp_method(&grid, "setrowcount", &[v_int(rows.len() as i64)]);
        for (ri, row) in rows.iter().enumerate() {
            for (ci, cell) in row.iter().enumerate() {
                object_web::rp_comp_method(&grid, "setcell", &[v_int(ci as i64), v_int(ri as i64), v_str(cell)]);
            }
        }
    }
    out_value(reply.value)
}

pub fn dataframe_method(name: &str, method: &str, args: &[Value]) -> Value {
    let args: Vec<Arg> = args.iter().map(frame_arg).collect();
    // (a file: one the program saved this session — OPEN, EXTRACTRESOURCE,
    // SaveToCsv —, else one of the project's, as the desktop reads it from
    // disk)
    object_web::install_file_hooks();
    let file = rapidr_frame::file_arg(method, &args).map(|path| object_web::web_project_file(&path).ok_or_else(|| format!("{path}: no such file")));
    // (Sample draws with the program's random numbers: RANDOMIZE repeats it)
    let seed = if method == "sample" { (rapidr_value::builtins::random_unit() * 9_007_199_254_740_992.0) as u64 } else { 0 };
    let call = Call { name: name.to_string(), method: method.to_string(), args, file, seed };
    match crate::frame_web::call(&call) {
        Some(reply) => apply(reply),
        None => {
            warn(&format!("[WARN] RDataFrame.{method}() not implemented"));
            v_null()
        }
    }
}

pub fn dataframe_get_prop(name: &str, prop: &str) -> Value {
    crate::frame_web::get_prop(name, prop).map(out_value).unwrap_or_else(v_null)
}

// --------------------------------------------------------------- RPLOT --

pub fn plot_method(name: &str, method: &str, args: &[Value]) -> Value {
    if plot::is_output(method) {
        // SaveFig file [, scale]: a PNG among the page's files (the program
        // reads it back, offers it as a download …), `scale` times the
        // chart's size. Render / Show: nothing to do (a QIMAGE shows a
        // chart: LoadFromPlot).
        if matches!(method, "savefig" | "save") {
            let file = args.first().map(Value::to_string_val).filter(|f| !f.is_empty()).unwrap_or_else(|| "plot.png".to_string());
            let scale = args.get(1).map(Value::to_f64).filter(|s| *s > 0.0).unwrap_or(1.0).min(8.0);
            object_web::install_file_hooks();
            let saved = rapidr_ui_render::chart::png(name, scale).and_then(|png| object_web::web_write_file(&file, &png));
            if let Err(e) = saved {
                web_sys::console::error_1(&JsValue::from_str(&format!("[ERROR] RPlot.SaveFig: {file}: {e}")));
            }
        }
        return v_null();
    }
    plot::method(name, method, args).unwrap_or_else(|| {
        warn(&format!("[WARN] RPlot.{method}() not implemented"));
        v_null()
    })
}

pub fn plot_get_prop(name: &str, prop: &str) -> Value {
    plot::get_prop(name, prop).unwrap_or_else(v_null)
}

pub fn plot_set_prop(name: &str, prop: &str, val: &Value) {
    plot::set_prop(name, prop, val);
}

/// A QIMAGE's picture from chart `plot` (`Image.LoadFromPlot`): its pixels,
/// drawn again at the page's scale for the screen; the chart's size.
pub fn load_into_picture(picture: &str, plot: &str) -> Option<(i64, i64)> {
    rapidr_ui_render::chart::load_into_picture(picture, plot)
}
