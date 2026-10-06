//! RNUM, RDATAFRAME and RPLOT in the browser: the shared model in
//! `rapidr_value::datascience` — the same one native builds and the
//! interpreter run — with what only the page does: `PRINT` to the page's
//! output, filling a QSTRINGGRID, and drawing charts on an HTML canvas.

use crate::object_web;
use crate::value::{v_int, v_str, Value};
use rapidr_value::datascience::{self as ds, plot::Plot};
use wasm_bindgen::JsCast;

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

    // (the chart is drawn on the page: a browser has no files to save it in)
    fn save_plot(&self, plot: &str, _file: &str) {
        render_plot(&plot.to_uppercase());
    }

    fn show_plot(&self, plot: &str) {
        render_plot(&plot.to_uppercase());
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

pub fn plot_method(name: &str, method: &str, args: &[Value]) -> Value {
    ds::plot::plot_method(name, method, args, &Web)
}

pub fn plot_get_prop(name: &str, prop: &str) -> Value {
    ds::plot::plot_get_prop(name, prop)
}

pub fn plot_set_prop(name: &str, prop: &str, val: &Value) {
    ds::plot::plot_set_prop(name, prop, val);
}

// ======================================================================
// Charts on an HTML canvas
// ======================================================================

/// The canvas chart `name` (uppercase) is drawn on: in its component's
/// element when the page has one, else a hidden one (what
/// `Image.LoadFromPlot` reads).
fn plot_canvas(name: &str) -> Option<web_sys::HtmlCanvasElement> {
    let doc = web_sys::window()?.document()?;
    let canvas_id = format!("rr-{}-canvas", name.to_lowercase());
    let el = match doc.get_element_by_id(&canvas_id) {
        Some(el) => el,
        None => {
            let c = doc.create_element("canvas").ok()?;
            c.set_id(&canvas_id);
            match doc.get_element_by_id(&format!("rr-{}", name.to_lowercase())) {
                Some(container) => {
                    // (the chart shows instead of the placeholder)
                    if let Some(ph) = container.query_selector(".rr-plot-placeholder").ok().flatten() {
                        if let Ok(ph) = ph.dyn_into::<web_sys::HtmlElement>() {
                            let _ = ph.style().set_property("display", "none");
                        }
                    }
                    c.set_class_name("rr-plot-container");
                    let _ = container.append_child(&c);
                }
                None => {
                    if let Ok(html) = c.clone().dyn_into::<web_sys::HtmlElement>() {
                        let _ = html.style().set_property("display", "none");
                    }
                    let _ = doc.body()?.append_child(&c);
                }
            }
            c
        }
    };
    el.dyn_into().ok()
}

/// Draws chart `name` (uppercase) on its canvas at the page's scale (sharp
/// on a high-DPI screen): the same model, axes and series as the desktop's
/// PNG.
pub fn render_plot(name: &str) {
    render_plot_at(name, rapidr_value::objects::bitmap::display_scale().max(1) as u32);
}

/// Draws chart `name` (uppercase) on its canvas, `scale` device pixels per
/// pixel (the canvas keeps the chart's size on the page); the canvas.
pub fn render_plot_at(name: &str, scale: u32) -> Option<web_sys::HtmlCanvasElement> {
    let state = ds::plot::state(name);
    let canvas = plot_canvas(name)?;
    let s = scale.clamp(1, 3);
    let (pw, ph) = state.pixel_size();
    canvas.set_width(pw * s);
    canvas.set_height(ph * s);
    let _ = canvas.style().set_property("width", &format!("{pw}px"));
    let _ = canvas.style().set_property("height", &format!("{ph}px"));
    let ctx: web_sys::CanvasRenderingContext2d = canvas.get_context("2d").ok().flatten()?.dyn_into().ok()?;
    let _ = ctx.set_transform(f64::from(s), 0.0, 0.0, f64::from(s), 0.0, 0.0);
    draw_plot(&ctx, &state, pw as f64, ph as f64);
    Some(canvas)
}

/// Chart `state` drawn off the page at `scale`, as a picture's pixels
/// (RapidQ's &HBBGGRR, over white where the chart is see-through).
pub fn plot_pixels(state: &Plot, scale: usize) -> Option<rapidr_value::objects::codec::Pixels> {
    let s = scale.clamp(1, 3) as u32;
    let (pw, ph) = state.pixel_size();
    let canvas: web_sys::HtmlCanvasElement = crate::page_web::document().create_element("canvas").ok()?.dyn_into().ok()?;
    canvas.set_width(pw * s);
    canvas.set_height(ph * s);
    let ctx: web_sys::CanvasRenderingContext2d = canvas.get_context("2d").ok().flatten()?.dyn_into().ok()?;
    let _ = ctx.set_transform(f64::from(s), 0.0, 0.0, f64::from(s), 0.0, 0.0);
    ctx.set_fill_style_str("white");
    ctx.fill_rect(0.0, 0.0, f64::from(pw), f64::from(ph));
    draw_plot(&ctx, state, f64::from(pw), f64::from(ph));
    let (w, h) = (pw * s, ph * s);
    let rgba = ctx.get_image_data(0.0, 0.0, f64::from(w), f64::from(h)).ok()?.data().0;
    let mix = |c: u8, a: u32| (u32::from(c) * a + 255 * (255 - a)) / 255;
    let pixels = rgba.as_chunks::<4>().0.iter().map(|p| {
        let a = u32::from(p[3]);
        mix(p[0], a) | mix(p[1], a) << 8 | mix(p[2], a) << 16
    });
    Some(rapidr_value::objects::codec::Pixels { width: w as usize, height: h as usize, pixels: pixels.collect() })
}

/// The chart on `ctx`, `w` × `h` pixels.
fn draw_plot(ctx: &web_sys::CanvasRenderingContext2d, state: &Plot, w: f64, h: f64) {
    ctx.set_fill_style_str("white");
    ctx.fill_rect(0.0, 0.0, w, h);
    if state.is_pie() {
        render_pie(ctx, state, w, h);
        return;
    }

    // (the shared model's ranges: the desktop draws the same axes)
    let ((x_min, x_max), (y_min, y_max)) = state.ranges();

    // The plot area (the desktop's margins).
    let ml = if state.ylabel.is_empty() { 62.0 } else { 78.0 };
    let mr = 14.0;
    let mt = if state.title.is_empty() { 14.0 } else { 46.0 };
    let mb = if state.xlabel.is_empty() { 42.0 } else { 60.0 };
    let aw = (w - ml - mr).max(1.0);
    let ah = (h - mt - mb).max(1.0);
    let px = |x: f64| ml + (x - x_min) / (x_max - x_min) * aw;
    let py = |y: f64| mt + (1.0 - (y - y_min) / (y_max - y_min)) * ah;

    // Grid, axes, ticks: round values, about six an axis.
    let xt = ds::plot::nice_ticks(x_min, x_max, 6);
    let yt = ds::plot::nice_ticks(y_min, y_max, 6);
    if state.grid {
        ctx.set_stroke_style_str("#e4e4e4");
        ctx.set_line_width(1.0);
        ctx.begin_path();
        for y in &yt {
            ctx.move_to(ml, py(*y));
            ctx.line_to(ml + aw, py(*y));
        }
        for x in &xt {
            ctx.move_to(px(*x), mt);
            ctx.line_to(px(*x), mt + ah);
        }
        ctx.stroke();
    }
    ctx.set_stroke_style_str("#505050");
    ctx.set_line_width(1.0);
    ctx.begin_path();
    ctx.move_to(ml, mt);
    ctx.line_to(ml, mt + ah);
    ctx.line_to(ml + aw, mt + ah);
    for y in &yt {
        ctx.move_to(ml - 4.0, py(*y));
        ctx.line_to(ml, py(*y));
    }
    for x in &xt {
        ctx.move_to(px(*x), mt + ah);
        ctx.line_to(px(*x), mt + ah + 4.0);
    }
    ctx.stroke();
    ctx.set_fill_style_str("#505050");
    ctx.set_font("12px sans-serif");
    ctx.set_text_align("right");
    for y in &yt {
        let _ = ctx.fill_text(&ds::plot::tick_text(*y), ml - 7.0, py(*y) + 4.0);
    }
    ctx.set_text_align("center");
    for x in &xt {
        let _ = ctx.fill_text(&ds::plot::tick_text(*x), px(*x), mt + ah + 18.0);
    }

    // The series.
    let path = |pts: &[(f64, f64)]| {
        ctx.begin_path();
        for (i, (x, y)) in pts.iter().enumerate() {
            if i == 0 {
                ctx.move_to(*x, *y);
            } else {
                ctx.line_to(*x, *y);
            }
        }
        ctx.stroke();
    };
    for s in &state.series {
        let color = ds::plot::css(&s.color);
        ctx.set_stroke_style_str(&color);
        ctx.set_fill_style_str(&color);
        ctx.set_line_width(2.0);
        let pts: Vec<(f64, f64)> = s.x.iter().zip(&s.y).map(|(x, y)| (px(*x), py(*y))).collect();
        match s.style.as_str() {
            "o" | "scatter" => {
                for (x, y) in &pts {
                    ctx.begin_path();
                    let _ = ctx.arc(*x, *y, 4.0, 0.0, std::f64::consts::TAU);
                    ctx.fill();
                }
            }
            "bar" => {
                let bw = ds::plot::bar_width(&s.x) / (x_max - x_min) * aw;
                let base = py(0f64.clamp(y_min, y_max));
                for (x, y) in &pts {
                    ctx.fill_rect(x - bw / 2.0, y.min(base), bw, (base - y).abs());
                }
            }
            "barh" => {
                let bh = ds::plot::bar_width(&s.y) / (y_max - y_min) * ah;
                let base = px(0f64.clamp(x_min, x_max));
                for (x, y) in &pts {
                    ctx.fill_rect(x.min(base), y - bh / 2.0, (x - base).abs(), bh);
                }
            }
            "step" => {
                let mut steps = Vec::with_capacity(pts.len() * 2);
                for (i, p) in pts.iter().enumerate() {
                    steps.push(*p);
                    if let Some(next) = pts.get(i + 1) {
                        steps.push((next.0, p.1));
                    }
                }
                path(&steps);
            }
            "area" => {
                if let (Some(first), Some(last)) = (pts.first(), pts.last()) {
                    let base = py(0f64.clamp(y_min, y_max));
                    ctx.set_global_alpha(0.3);
                    ctx.begin_path();
                    ctx.move_to(first.0, base);
                    for (x, y) in &pts {
                        ctx.line_to(*x, *y);
                    }
                    ctx.line_to(last.0, base);
                    ctx.close_path();
                    ctx.fill();
                    ctx.set_global_alpha(1.0);
                }
                path(&pts);
            }
            "hline" => {
                ctx.set_line_width(1.0);
                for y in &s.y {
                    path(&[(ml, py(*y)), (ml + aw, py(*y))]);
                }
            }
            "vline" => {
                ctx.set_line_width(1.0);
                for x in &s.x {
                    path(&[(px(*x), mt), (px(*x), mt + ah)]);
                }
            }
            "--" | "dashed" => {
                let dash = js_sys::Array::of2(&6.0.into(), &4.0.into());
                let _ = ctx.set_line_dash(&dash);
                path(&pts);
                let _ = ctx.set_line_dash(&js_sys::Array::new());
            }
            _ => path(&pts),
        }
    }

    for a in &state.annotations {
        ctx.set_fill_style_str(&ds::plot::css(&a.color));
        ctx.set_font("14px sans-serif");
        ctx.set_text_align("left");
        let _ = ctx.fill_text(&a.text, px(a.x), py(a.y) + 12.0);
    }
    if !state.title.is_empty() {
        title(ctx, &state.title, w);
    }
    ctx.set_fill_style_str("#333");
    ctx.set_font("14px sans-serif");
    if !state.xlabel.is_empty() {
        ctx.set_text_align("center");
        let _ = ctx.fill_text(&state.xlabel, ml + aw / 2.0, h - 12.0);
    }
    if !state.ylabel.is_empty() {
        ctx.save();
        ctx.set_text_align("center");
        let _ = ctx.translate(20.0, mt + ah / 2.0);
        let _ = ctx.rotate(-std::f64::consts::FRAC_PI_2);
        let _ = ctx.fill_text(&state.ylabel, 0.0, 0.0);
        ctx.restore();
    }
    if state.legend {
        // (a framed box in the upper right, a line, mark or swatch a series)
        let labeled: Vec<&ds::plot::Series> = state.series.iter().filter(|s| !s.label.is_empty() && s.style != "hline" && s.style != "vline").collect();
        if !labeled.is_empty() {
            ctx.set_font("13px sans-serif");
            let text_w = labeled.iter().filter_map(|s| ctx.measure_text(&s.label).ok()).map(|m| m.width()).fold(0.0, f64::max);
            let (lw, lh) = (text_w + 52.0, labeled.len() as f64 * 22.0 + 10.0);
            // (the corner with the fewest points, as the desktop's)
            let (right, top) = ds::plot::legend_corner(state, (x_min, x_max), (y_min, y_max), ((lw + 16.0) / aw, (lh + 16.0) / ah));
            let lx = if right { ml + aw - lw - 10.0 } else { ml + 10.0 };
            let ly = if top { mt + 10.0 } else { mt + ah - lh - 10.0 };
            ctx.set_fill_style_str("rgba(255,255,255,0.9)");
            ctx.fill_rect(lx, ly, lw, lh);
            ctx.set_stroke_style_str("#bebebe");
            ctx.set_line_width(1.0);
            ctx.stroke_rect(lx, ly, lw, lh);
            ctx.set_text_align("left");
            for (i, s) in labeled.iter().enumerate() {
                let ey = ly + 16.0 + i as f64 * 22.0;
                let c = ds::plot::css(&s.color);
                ctx.set_fill_style_str(&c);
                ctx.set_stroke_style_str(&c);
                ctx.set_line_width(2.0);
                match s.style.as_str() {
                    "o" | "scatter" => {
                        ctx.begin_path();
                        let _ = ctx.arc(lx + 19.0, ey, 4.0, 0.0, std::f64::consts::TAU);
                        ctx.fill();
                    }
                    "bar" | "barh" | "area" => ctx.fill_rect(lx + 8.0, ey - 5.0, 22.0, 10.0),
                    _ => {
                        ctx.begin_path();
                        ctx.move_to(lx + 8.0, ey);
                        ctx.line_to(lx + 30.0, ey);
                        ctx.stroke();
                    }
                }
                ctx.set_fill_style_str("#333");
                let _ = ctx.fill_text(&s.label, lx + 38.0, ey + 4.5);
            }
        }
    }
}

/// A chart's title, centred at its top.
fn title(ctx: &web_sys::CanvasRenderingContext2d, text: &str, w: f64) {
    ctx.set_fill_style_str("#222");
    ctx.set_font("22px sans-serif");
    ctx.set_text_align("center");
    let _ = ctx.fill_text(text, w / 2.0, 30.0);
}

/// A pie: the first pie series' values, labels and colours, each slice
/// labelled with its share outside it, white between slices (as the
/// desktop's).
fn render_pie(ctx: &web_sys::CanvasRenderingContext2d, state: &Plot, w: f64, h: f64) {
    let values: Vec<f64> = state.series.iter().filter(|s| s.style == "pie").flat_map(|s| s.x.iter().copied()).collect();
    let total: f64 = values.iter().filter(|v| **v > 0.0).sum();
    let top = if state.title.is_empty() { 0.0 } else { 34.0 };
    if !state.title.is_empty() {
        title(ctx, &state.title, w);
    }
    if total <= 0.0 {
        return;
    }
    let first = state.series.iter().find(|s| s.style == "pie");
    let labels: Vec<String> = match first.map(|s| s.label.trim()).filter(|l| !l.is_empty()) {
        Some(l) => l.split(',').map(|s| s.trim().to_string()).collect(),
        None => (0..values.len()).map(|i| format!("Slice {}", i + 1)).collect(),
    };
    let colors: Vec<String> = match first.map(|s| s.color.trim()).filter(|c| !c.is_empty()) {
        Some(c) => c.split(',').map(|s| s.trim().to_string()).collect(),
        None => ds::plot::PALETTE.iter().map(|s| s.to_string()).collect(),
    };
    let (cx, cy) = (w / 2.0, top + (h - top) / 2.0);
    let radius = (w.min(h - top) * 0.36).max(20.0);
    let mut start = -std::f64::consts::FRAC_PI_2;
    for (i, value) in values.iter().enumerate().filter(|(_, v)| **v > 0.0) {
        let sweep = value / total * std::f64::consts::TAU;
        ctx.set_fill_style_str(&ds::plot::css(colors.get(i).map(String::as_str).unwrap_or("gray")));
        ctx.begin_path();
        ctx.move_to(cx, cy);
        let _ = ctx.arc(cx, cy, radius, start, start + sweep);
        ctx.close_path();
        ctx.fill();
        ctx.set_stroke_style_str("white");
        ctx.set_line_width(2.0);
        ctx.stroke();
        let mid = start + sweep / 2.0;
        ctx.set_fill_style_str("#333");
        ctx.set_font("13px sans-serif");
        ctx.set_text_align(if mid.cos() >= 0.0 { "left" } else { "right" });
        let text = format!("{} ({:.1}%)", labels.get(i).map(String::as_str).unwrap_or(""), value / total * 100.0);
        let _ = ctx.fill_text(&text, cx + radius * 1.12 * mid.cos(), cy + radius * 1.12 * mid.sin() + 4.5);
        start += sweep;
    }
}
