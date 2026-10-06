//! RNUM, RDATAFRAME and RPLOT in desktop programs (native and interpreted).
//!
//! Each is one implementation shared with the web (decision D7,
//! docs/ide-plan.md): RNUM and RPLOT are rapidr-value's models (an RPLOT
//! drawn by rapidr-ui-render, crisp at any scale), RDATAFRAME is
//! rapidr-frame's (polars). What stays here is the desktop's side of them:
//! files on disk, PRINT, a QSTRINGGRID to fill, the warning stream.

use rapidr_frame::{Arg, Call, Out, Reply};
use rapidr_value::datascience::{num, plot};

use crate::value::{v_dbl, v_int, v_null, v_str, Value};

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
            eprintln!("[WARN] RNum.{method}() not implemented");
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

/// A file a program names: as it is, else beside the executable, else in
/// RAPIDR_HOME's examples (a program run from anywhere finds its data).
fn resolve_data_path(path: &str) -> std::path::PathBuf {
    let p = std::path::Path::new(path);
    if p.exists() {
        return p.to_path_buf();
    }
    let beside_exe = std::env::current_exe().ok().and_then(|exe| exe.parent().map(|d| d.join(path)));
    let in_home = std::env::var("RAPIDR_HOME").ok().map(|h| [std::path::Path::new(&h).join("examples").join(path), std::path::Path::new(&h).join(path)]);
    beside_exe.into_iter().chain(in_home.into_iter().flatten()).find(|c| c.exists()).unwrap_or_else(|| p.to_path_buf())
}

/// A BASIC value as the frame engine takes it.
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

/// What a reply asks of the desktop, done; its value.
fn apply(reply: Reply) -> Value {
    if let Some(e) = reply.error {
        eprintln!("[ERROR] {e}");
    }
    if let Some(text) = reply.print {
        crate::builtins::rp_print(&[v_str(&text)], true);
    }
    if let Some((path, bytes)) = reply.write {
        if let Err(e) = std::fs::write(&path, bytes) {
            eprintln!("[ERROR] RDataFrame: {path}: {e}");
        }
    }
    if let Some((grid, rows)) = reply.grid {
        populate_grid(&grid, &rows);
    }
    out_value(reply.value)
}

pub fn dataframe_method(name: &str, method: &str, args: &[Value]) -> Value {
    let args: Vec<Arg> = args.iter().map(frame_arg).collect();
    let file = rapidr_frame::file_arg(method, &args).map(|path| std::fs::read(resolve_data_path(&path)).map_err(|e| format!("{path}: {e}")));
    // (Sample draws with the program's random numbers: RANDOMIZE repeats it)
    let seed = if method == "sample" { (crate::value::builtins::random_unit() * 9_007_199_254_740_992.0) as u64 } else { 0 };
    match rapidr_frame::call(Call { name: name.to_string(), method: method.to_string(), args, file, seed }) {
        Some(reply) => apply(reply),
        None => {
            eprintln!("[WARN] RDataFrame.{method}() not implemented");
            v_null()
        }
    }
}

/// A QSTRINGGRID filled with `rows` (the column names first).
#[cfg(feature = "gui")]
fn populate_grid(grid: &str, rows: &[Vec<String>]) {
    use crate::object::{rp_comp_method, rp_comp_set};
    let ncols = rows.first().map(Vec::len).unwrap_or(0);
    rp_comp_set(grid, "cols", v_int(ncols as i64));
    rp_comp_set(grid, "rowcount", v_int(rows.len() as i64));
    for (ri, row) in rows.iter().enumerate() {
        for (ci, cell) in row.iter().enumerate() {
            rp_comp_method(grid, "setcell", &[v_int(ci as i64), v_int(ri as i64), v_str(cell)]);
        }
    }
}

#[cfg(not(feature = "gui"))]
fn populate_grid(_grid: &str, _rows: &[Vec<String>]) {}

pub fn dataframe_get_prop(name: &str, prop: &str) -> Value {
    rapidr_frame::get_prop(name, prop).map(out_value).unwrap_or_else(v_null)
}

// --------------------------------------------------------------- RPLOT --

pub fn plot_method(name: &str, method: &str, args: &[Value]) -> Value {
    if plot::is_output(method) {
        // SaveFig file [, scale]: a PNG, `scale` times the chart's size
        // (2: for a high-DPI screen or print). Render / Show: nothing to
        // do on the desktop (a QIMAGE shows a chart: LoadFromPlot).
        if matches!(method, "savefig" | "save") {
            let file = args.first().map(Value::to_string_val).filter(|f| !f.is_empty()).unwrap_or_else(|| "plot.png".to_string());
            let scale = args.get(1).map(Value::to_f64).filter(|s| *s > 0.0).unwrap_or(1.0).min(8.0);
            match rapidr_ui_render::chart::png(name, scale) {
                Ok(png) => {
                    if let Err(e) = std::fs::write(&file, png) {
                        eprintln!("[ERROR] RPlot.SaveFig: {file}: {e}");
                    }
                }
                Err(e) => eprintln!("[ERROR] RPlot.SaveFig: {e}"),
            }
        }
        return v_null();
    }
    plot::method(name, method, args).unwrap_or_else(|| {
        eprintln!("[WARN] RPlot.{method}() not implemented");
        v_null()
    })
}

pub fn plot_get_prop(name: &str, prop: &str) -> Value {
    plot::get_prop(name, prop).unwrap_or_else(v_null)
}

pub fn plot_set_prop(name: &str, prop: &str, val: &Value) {
    plot::set_prop(name, prop, val);
}
