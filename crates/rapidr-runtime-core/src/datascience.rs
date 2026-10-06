//! RNUM, RDATAFRAME and RPLOT on the desktop (native builds and interpreted
//! programs): the shared model in `rapidr_value::datascience` — the same
//! one the web runs — with what only the desktop does: `PRINT` to stdout,
//! filling a QSTRINGGRID, and drawing charts with plotters into PNGs
//! (`SaveFig`, `Image.LoadFromPlot`).

use plotters::prelude::*;
use rapidr_value::datascience::{self as ds, plot::Plot};

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

    fn save_plot(&self, plot: &str, file: &str) {
        let bytes = render_plot_png(plot, 1);
        if !bytes.is_empty() {
            if let Err(e) = std::fs::write(file, &bytes) {
                eprintln!("[ERROR] RPlot.savefig: can't write {file}: {e}");
            }
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

fn rgb_color(name: &str) -> RGBColor {
    let (r, g, b) = ds::plot::rgb(name);
    RGBColor(r, g, b)
}

/// Encode an RGB pixel buffer to PNG bytes in memory.
fn encode_rgb_to_png(buf: &[u8], w: u32, h: u32) -> Vec<u8> {
    let mut png_bytes: Vec<u8> = Vec::new();
    {
        let encoder = image::codecs::png::PngEncoder::new(std::io::Cursor::new(&mut png_bytes));
        use image::ImageEncoder;
        let _ = encoder.write_image(buf, w, h, image::ColorType::Rgb8);
    }
    png_bytes
}

/// What charts are drawn on: plotters' bitmap backend, with the text in the
/// built-in Liberation Sans (the program carries it already; metrically
/// Arial's and Helvetica's), the same on every system (Cargo.toml). Sizes and
/// placement are those plotters gives a font of its own: an em of
/// size / 1.24 pixels, the baseline 0.76 em below the text's top; the
/// edges' coverage goes through a square root (gamma 2), so the strokes
/// weigh what the systems' rasterizers gave them.
struct Chart<'a>(BitMapBackend<'a>);

type ChartError<'a> = plotters_backend::DrawingErrorKind<<BitMapBackend<'a> as DrawingBackend>::ErrorType>;

/// The charts' face: Liberation Sans.
fn chart_face() -> &'static ab_glyph::FontRef<'static> {
    static FACE: std::sync::OnceLock<ab_glyph::FontRef<'static>> = std::sync::OnceLock::new();
    FACE.get_or_init(|| ab_glyph::FontRef::try_from_slice(rapidr_value::objects::text::BUILTIN_FONTS[0]).expect("the built-in Liberation Sans"))
}

/// ab_glyph's scale (the face's height in pixels) for an em of `em` pixels.
fn chart_scale(em: f32) -> ab_glyph::PxScale {
    use ab_glyph::Font;
    let face = chart_face();
    ab_glyph::PxScale::from(em * face.height_unscaled() / face.units_per_em().unwrap_or(2048.0))
}

/// `text` laid out at plotters' `size`: each glyph (its id, its x), the
/// width, the em.
fn chart_layout(text: &str, size: f64) -> (Vec<(ab_glyph::GlyphId, f32)>, i32, f32) {
    use ab_glyph::{Font, ScaleFont};
    let em = (size / 1.24) as f32;
    let face = chart_face();
    let scaled = face.as_scaled(chart_scale(em));
    let mut x = 0f32;
    let mut prev = None;
    let mut glyphs = Vec::new();
    for c in text.chars() {
        let id = scaled.glyph_id(c);
        if let Some(p) = prev {
            x += scaled.kern(p, id);
        }
        glyphs.push((id, x));
        x += scaled.h_advance(id);
        prev = Some(id);
    }
    (glyphs, x as i32, em)
}

impl<'a> DrawingBackend for Chart<'a> {
    type ErrorType = <BitMapBackend<'a> as DrawingBackend>::ErrorType;

    fn get_size(&self) -> (u32, u32) {
        self.0.get_size()
    }
    fn ensure_prepared(&mut self) -> Result<(), ChartError<'a>> {
        self.0.ensure_prepared()
    }
    fn present(&mut self) -> Result<(), ChartError<'a>> {
        self.0.present()
    }
    fn draw_pixel(&mut self, point: plotters_backend::BackendCoord, color: plotters_backend::BackendColor) -> Result<(), ChartError<'a>> {
        self.0.draw_pixel(point, color)
    }
    fn draw_line<S: plotters_backend::BackendStyle>(&mut self, from: plotters_backend::BackendCoord, to: plotters_backend::BackendCoord, style: &S) -> Result<(), ChartError<'a>> {
        self.0.draw_line(from, to, style)
    }
    fn draw_rect<S: plotters_backend::BackendStyle>(&mut self, upper_left: plotters_backend::BackendCoord, bottom_right: plotters_backend::BackendCoord, style: &S, fill: bool) -> Result<(), ChartError<'a>> {
        self.0.draw_rect(upper_left, bottom_right, style, fill)
    }
    fn blit_bitmap(&mut self, pos: plotters_backend::BackendCoord, size: (u32, u32), src: &[u8]) -> Result<(), ChartError<'a>> {
        self.0.blit_bitmap(pos, size, src)
    }

    fn estimate_text_size<T: plotters_backend::BackendTextStyle>(&self, text: &str, style: &T) -> Result<(u32, u32), ChartError<'a>> {
        let (_, width, em) = chart_layout(text, style.size());
        Ok((width.max(0) as u32, em as u32))
    }

    // (plotters' own placement: the anchor against the layout box, then the
    // rotation)
    fn draw_text<T: plotters_backend::BackendTextStyle>(&mut self, text: &str, style: &T, pos: plotters_backend::BackendCoord) -> Result<(), ChartError<'a>> {
        use ab_glyph::Font;
        use plotters_backend::text_anchor::{HPos, VPos};
        let color = style.color();
        if color.alpha == 0.0 {
            return Ok(());
        }
        let (glyphs, width, em) = chart_layout(text, style.size());
        let height = em as i32;
        let dx = match style.anchor().h_pos {
            HPos::Left => 0,
            HPos::Right => -width,
            HPos::Center => -width / 2,
        };
        let dy = match style.anchor().v_pos {
            VPos::Top => 0,
            VPos::Center => -height / 2,
            VPos::Bottom => -height,
        };
        let trans = style.transform();
        let (w, h) = self.get_size();
        let face = chart_face();
        let scale = chart_scale(em);
        let baseline = 0.76 * em;
        for (id, x) in glyphs {
            let Some(outline) = face.outline_glyph(id.with_scale_and_position(scale, ab_glyph::point(x, baseline))) else { continue };
            let bounds = outline.px_bounds();
            let mut pixels = Vec::new();
            outline.draw(|gx, gy, coverage| pixels.push((bounds.min.x as i32 + gx as i32, bounds.min.y as i32 + gy as i32, coverage)));
            for (px, py, coverage) in pixels {
                let (tx, ty) = trans.transform(px + dx, py + dy);
                let (tx, ty) = (pos.0 + tx, pos.1 + ty);
                if tx >= 0 && tx < w as i32 && ty >= 0 && ty < h as i32 {
                    self.0.draw_pixel((tx, ty), plotters_backend::BackendColor { alpha: color.alpha * f64::from(coverage).sqrt(), rgb: color.rgb })?;
                }
            }
        }
        Ok(())
    }
}

/// Chart `name` as a PNG, `scale` device pixels per pixel (1, or the
/// screen's 2 or 3 so a chart in a QIMAGE stays sharp): the same layout
/// as the web's canvas — light grid, 2-pixel lines, a framed legend.
fn render_plot_png(name: &str, scale: u32) -> Vec<u8> {
    render_png(&ds::plot::state(name), scale)
}

/// Chart `state` as a PNG, `scale` device pixels per pixel.
fn render_png(state: &Plot, scale: u32) -> Vec<u8> {
    // (drawn twice as fine and averaged down: smooth edges — plotters
    // fills shapes without anti-aliasing)
    let scale = scale.clamp(1, 3);
    let (w, h) = state.pixel_size();
    // (a huge chart: drawn once, at its size)
    let k = if u64::from(w * scale * 2) * u64::from(h * scale * 2) > 40_000_000 { 1 } else { 2 };
    let fine = if state.is_pie() { render_pie_rgb(state, w * scale * k, h * scale * k, scale * k) } else { render_rgb(state, scale * k) };
    if fine.is_empty() {
        return Vec::new();
    }
    let (w, h) = (w * scale, h * scale);
    encode_rgb_to_png(&if k == 2 { downsample(&fine, w as usize, h as usize) } else { fine }, w, h)
}

/// An RGB image of `2w` × `2h` pixels averaged down to `w` × `h`.
fn downsample(fine: &[u8], w: usize, h: usize) -> Vec<u8> {
    let fw = w * 2;
    let mut out = vec![0u8; w * h * 3];
    for y in 0..h {
        for x in 0..w {
            for c in 0..3 {
                let at = |dx: usize, dy: usize| u32::from(fine[((y * 2 + dy) * fw + x * 2 + dx) * 3 + c]);
                out[(y * w + x) * 3 + c] = ((at(0, 0) + at(1, 0) + at(0, 1) + at(1, 1) + 2) / 4) as u8;
            }
        }
    }
    out
}

/// About how much of the plot area the legend covers, its margin and
/// the marks' size included: (width, height) as fractions.
fn legend_size(state: &Plot) -> (f64, f64) {
    let labels: Vec<&ds::plot::Series> = state.series.iter().filter(|sr| !sr.label.is_empty() && sr.style != "hline" && sr.style != "vline").collect();
    let (w, h) = state.pixel_size();
    let aw = f64::from(w) - if state.ylabel.is_empty() { 74.0 } else { 90.0 };
    let ah = f64::from(h) - if state.title.is_empty() { 14.0 } else { 46.0 } - if state.xlabel.is_empty() { 40.0 } else { 58.0 };
    let longest = labels.iter().map(|sr| sr.label.chars().count()).max().unwrap_or(0) as f64;
    ((longest * 7.5 + 70.0) / aw.max(1.0), (labels.len() as f64 * 22.0 + 30.0) / ah.max(1.0))
}

/// Chart `state` as RGB pixels, `scale` device pixels per pixel.
fn render_rgb(state: &Plot, scale: u32) -> Vec<u8> {
    let s = scale.clamp(1, 6);
    let (w, h) = state.pixel_size();
    let (w, h) = (w * s, h * s);
    let si = s as i32;
    let px = |v: u32| v * s;
    // (a font of `n` pixels: plotters' sizes are 1.24 ems, Chart above)
    let pt = |n: i32| (f64::from(n * si) * 1.24).round();
    let mut pixel_buf = vec![0u8; (w * h * 3) as usize];
    {
        let root = Chart(BitMapBackend::with_buffer(&mut pixel_buf, (w, h))).into_drawing_area();
        if root.fill(&WHITE).is_err() {
            return Vec::new();
        }

        // (the shared model's ranges: the web draws the same axes)
        let ((x_min, x_max), (y_min, y_max)) = state.ranges();

        let mut builder = ChartBuilder::on(&root);
        if !state.title.is_empty() {
            builder.caption(&state.title, ("sans-serif", pt(22)).into_font().color(&RGBColor(34, 34, 34)));
        }
        builder
            .margin(px(14))
            .x_label_area_size(px(if state.xlabel.is_empty() { 26 } else { 44 }))
            .y_label_area_size(px(if state.ylabel.is_empty() { 46 } else { 62 }));
        let mut chart = match builder.build_cartesian_2d(x_min..x_max, y_min..y_max) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[ERROR] RPlot: {e}");
                return Vec::new();
            }
        };
        let axis = RGBColor(80, 80, 80);
        let mut mesh = chart.configure_mesh();
        mesh.x_labels(6)
            .y_labels(6)
            .x_label_formatter(&|v| ds::plot::tick_text(*v))
            .y_label_formatter(&|v| ds::plot::tick_text(*v))
            .label_style(("sans-serif", pt(12)).into_font().color(&axis))
            .axis_desc_style(("sans-serif", pt(14)).into_font().color(&RGBColor(51, 51, 51)))
            .axis_style(ShapeStyle { color: axis.to_rgba(), filled: false, stroke_width: s })
            .set_all_tick_mark_size(px(4) as i32)
            .light_line_style(TRANSPARENT)
            .bold_line_style(ShapeStyle { color: RGBColor(228, 228, 228).to_rgba(), filled: false, stroke_width: s });
        if !state.xlabel.is_empty() {
            mesh.x_desc(&state.xlabel);
        }
        if !state.ylabel.is_empty() {
            mesh.y_desc(&state.ylabel);
        }
        if !state.grid {
            mesh.disable_mesh();
        }
        let _ = mesh.draw();

        let line = |c: RGBColor| ShapeStyle { color: c.to_rgba(), filled: false, stroke_width: 2 * s };
        for sr in &state.series {
            let color = rgb_color(&sr.color);
            let points: Vec<(f64, f64)> = sr.x.iter().zip(&sr.y).map(|(x, y)| (*x, *y)).collect();
            let key = px(22) as i32;
            let legend_line = move |(x, y): (i32, i32)| PathElement::new(vec![(x, y), (x + key, y)], line(color));
            let drawn = match sr.style.as_str() {
                "o" | "scatter" => chart
                    .draw_series(points.iter().map(|&p| Circle::new(p, 4 * si, color.filled())))
                    .map(|a| a.legend(move |(x, y)| Circle::new((x + key / 2, y), 4 * si, color.filled()))),
                "bar" => {
                    let bw = ds::plot::bar_width(&sr.x);
                    chart
                        .draw_series(points.iter().map(|&(x, y)| Rectangle::new([(x - bw / 2.0, 0.0), (x + bw / 2.0, y)], color.filled())))
                        .map(|a| a.legend(move |(x, y)| Rectangle::new([(x, y - 5 * si), (x + key, y + 5 * si)], color.filled())))
                }
                "barh" => {
                    let bh = ds::plot::bar_width(&sr.y);
                    chart
                        .draw_series(points.iter().map(|&(x, y)| Rectangle::new([(0.0, y - bh / 2.0), (x, y + bh / 2.0)], color.filled())))
                        .map(|a| a.legend(move |(x, y)| Rectangle::new([(x, y - 5 * si), (x + key, y + 5 * si)], color.filled())))
                }
                "step" => {
                    let mut steps = Vec::with_capacity(points.len() * 2);
                    for (i, p) in points.iter().enumerate() {
                        steps.push(*p);
                        if let Some(next) = points.get(i + 1) {
                            steps.push((next.0, p.1));
                        }
                    }
                    chart.draw_series(LineSeries::new(steps, line(color))).map(|a| a.legend(legend_line))
                }
                "area" => {
                    let _ = chart.draw_series(AreaSeries::new(points.clone(), 0.0, color.mix(0.3)));
                    chart.draw_series(LineSeries::new(points, line(color))).map(|a| a.legend(legend_line))
                }
                "hline" => chart.draw_series(sr.y.iter().map(|&y| PathElement::new(vec![(x_min, y), (x_max, y)], ShapeStyle { color: color.to_rgba(), filled: false, stroke_width: s }))),
                "vline" => chart.draw_series(sr.x.iter().map(|&x| PathElement::new(vec![(x, y_min), (x, y_max)], ShapeStyle { color: color.to_rgba(), filled: false, stroke_width: s }))),
                "--" | "dashed" => chart
                    .draw_series(DashedLineSeries::new(points, 7 * si, 5 * si, line(color)))
                    .map(|a| a.legend(move |(x, y)| PathElement::new(vec![(x, y), (x + key / 2 - si, y)], line(color)))),
                _ => chart.draw_series(LineSeries::new(points, line(color))).map(|a| a.legend(legend_line)),
            };
            if let Ok(a) = drawn {
                if !sr.label.is_empty() {
                    a.label(&sr.label);
                }
            }
        }

        for ann in &state.annotations {
            let color = rgb_color(&ann.color);
            let _ = chart.draw_series(std::iter::once(plotters::element::Text::new(ann.text.clone(), (ann.x, ann.y), ("sans-serif", pt(14)).into_font().color(&color))));
        }

        if state.legend && state.series.iter().any(|sr| !sr.label.is_empty()) {
            let _ = chart
                .configure_series_labels()
                .position(match ds::plot::legend_corner(state, (x_min, x_max), (y_min, y_max), legend_size(state)) {
                    (true, true) => SeriesLabelPosition::UpperRight,
                    (false, true) => SeriesLabelPosition::UpperLeft,
                    (true, false) => SeriesLabelPosition::LowerRight,
                    (false, false) => SeriesLabelPosition::LowerLeft,
                })
                .margin(px(10) as i32)
                .legend_area_size(px(30) as i32)
                .label_font(("sans-serif", pt(13)).into_font().color(&RGBColor(51, 51, 51)))
                .background_style(WHITE.mix(0.9))
                .border_style(ShapeStyle { color: RGBColor(190, 190, 190).to_rgba(), filled: false, stroke_width: s })
                .draw();
        }
        let _ = root.present();
    }
    pixel_buf
}

/// A pie: the first pie series' values, labels and colours, each slice
/// labelled with its share, white between slices.
fn render_pie_rgb(state: &Plot, w: u32, h: u32, s: u32) -> Vec<u8> {
    let si = s as i32;
    let pt = |n: i32| (f64::from(n * si) * 1.24).round();
    let values: Vec<f64> = state.series.iter().filter(|sr| sr.style == "pie").flat_map(|sr| sr.x.iter().copied()).collect();
    let total: f64 = values.iter().filter(|v| **v > 0.0).sum();
    let mut pixel_buf = vec![0u8; (w * h * 3) as usize];
    {
        let root = Chart(BitMapBackend::with_buffer(&mut pixel_buf, (w, h))).into_drawing_area();
        if root.fill(&WHITE).is_err() {
            return Vec::new();
        }
        let top = if state.title.is_empty() {
            0.0
        } else {
            let _ = root.draw(&plotters::element::Text::new(
                state.title.clone(),
                (w as i32 / 2, 12 * si),
                ("sans-serif", pt(22)).into_font().color(&RGBColor(34, 34, 34)).pos(plotters::style::text_anchor::Pos::new(plotters::style::text_anchor::HPos::Center, plotters::style::text_anchor::VPos::Top)),
            ));
            34.0 * s as f64
        };
        if total > 0.0 {
            let first = state.series.iter().find(|sr| sr.style == "pie");
            let labels: Vec<String> = match first.map(|sr| sr.label.trim()).filter(|l| !l.is_empty()) {
                Some(l) => l.split(',').map(|x| x.trim().to_string()).collect(),
                None => (0..values.len()).map(|i| format!("Slice {}", i + 1)).collect(),
            };
            let colors: Vec<&str> = match first.map(|sr| sr.color.trim()).filter(|c| !c.is_empty()) {
                Some(c) => c.split(',').map(str::trim).collect(),
                None => ds::plot::PALETTE.to_vec(),
            };
            let cx = w as f64 / 2.0;
            let cy = top + (h as f64 - top) / 2.0;
            let radius = ((w as f64).min(h as f64 - top) * 0.36).max(20.0);
            let mut start = -std::f64::consts::FRAC_PI_2;
            for (i, &value) in values.iter().enumerate().filter(|(_, v)| **v > 0.0) {
                let sweep = value / total * std::f64::consts::TAU;
                let color = rgb_color(colors.get(i).copied().unwrap_or("gray"));
                let steps = ((sweep * radius / 2.0) as usize).max(4);
                let mut pts: Vec<(i32, i32)> = vec![(cx as i32, cy as i32)];
                for k in 0..=steps {
                    let a = start + sweep * k as f64 / steps as f64;
                    pts.push(((cx + radius * a.cos()).round() as i32, (cy + radius * a.sin()).round() as i32));
                }
                let _ = root.draw(&plotters::element::Polygon::new(pts.clone(), color.filled()));
                let _ = root.draw(&PathElement::new(pts.into_iter().chain(std::iter::once((cx as i32, cy as i32))).collect::<Vec<_>>(), ShapeStyle { color: WHITE.to_rgba(), filled: false, stroke_width: 2 * s }));
                let mid = start + sweep / 2.0;
                let text = format!("{} ({:.1}%)", labels.get(i).map(String::as_str).unwrap_or(""), value / total * 100.0);
                let (lx, ly) = (cx + radius * 1.12 * mid.cos(), cy + radius * 1.12 * mid.sin());
                let hpos = if mid.cos() >= 0.0 { plotters::style::text_anchor::HPos::Left } else { plotters::style::text_anchor::HPos::Right };
                let _ = root.draw(&plotters::element::Text::new(
                    text,
                    (lx as i32, ly as i32),
                    ("sans-serif", pt(13)).into_font().color(&RGBColor(51, 51, 51)).pos(plotters::style::text_anchor::Pos::new(hpos, plotters::style::text_anchor::VPos::Center)),
                ));
                start += sweep;
            }
        }
        let _ = root.present();
    }
    pixel_buf
}

/// Chart `name` as a PNG (SaveFig; Image.LoadFromPlot's pixels).
pub fn plot_render_to_bytes(name: &str) -> Vec<u8> {
    render_plot_png(name, 1)
}

/// What draws chart `name` — as it is now — again at a screen scale: the
/// sharp picture a high-DPI screen shows of `Image.LoadFromPlot`.
pub fn plot_redraw(name: &str) -> impl Fn(usize) -> Option<rapidr_value::objects::codec::Pixels> + 'static {
    let state = ds::plot::state(name);
    move |scale| rapidr_value::objects::codec::decode_raster(&render_png(&state, scale as u32)).ok().map(|(p, _)| p)
}
