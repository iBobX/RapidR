//! RPLOT's charts as pixels — the one renderer every runtime uses: the
//! chart (`rapidr_value::datascience::chart`) is the kernel's ops, and the
//! CPU renderer makes them pixels at any scale, in the kernel's fonts, so a
//! chart is the same pixels on the desktop and the web. `SaveFig` writes
//! [`png`]; a QIMAGE's `LoadFromPlot` takes [`load_into_picture`]: the 1×
//! pixels a program reads (`Pixel`), and the chart drawn again at the
//! screen's scale for what it shows.

use std::cell::RefCell;

use rapidr_ui_kernel::TextSystem;
use rapidr_value::datascience::{chart, plot};
use rapidr_value::objects::codec::{encode_png, Pixels};
use rapidr_value::objects::ops::Op;

thread_local! {
    /// The charts' own text system (the built-in fonts; made on first use).
    static TEXT: RefCell<Option<TextSystem>> = const { RefCell::new(None) };
}

/// `ops` (a chart's, `size` logical pixels) at `scale`.
pub fn render(ops: &[Op], size: (i64, i64), scale: f64) -> Pixels {
    TEXT.with(|t| {
        let mut t = t.borrow_mut();
        let text = t.get_or_insert_with(TextSystem::new);
        crate::cpu::render_ops(ops, size, scale, text)
    })
}

/// Chart `p` in the current theme, at `scale` (1: its size in pixels).
pub fn pixels(p: &plot::Plot, scale: f64) -> Pixels {
    let (ops, size) = chart::ops(p, rapidr_value::theme::current());
    render(&ops, size, scale)
}

/// Chart `p` as a PNG at `scale`.
pub fn png(p: &plot::Plot, scale: f64) -> Result<Vec<u8>, String> {
    encode_png(&pixels(p, scale))
}

/// Chart `plot` (as it is now) into picture `picture` (a QIMAGE's): its
/// pixels at 1×, and drawn again at the screen's scale for the screen. The
/// chart's size; `None` when `picture` isn't a picture.
pub fn load_into_picture(picture: &str, plot: &str) -> Option<(i64, i64)> {
    let state = plot::state(plot);
    let (ops, size) = chart::ops(&state, rapidr_value::theme::current());
    let lo = render(&ops, size, 1.0);
    rapidr_value::objects::with_picture(picture, move |b| {
        b.set_pixels(lo);
        b.set_redraw(move |s| Some(render(&ops, size, s as f64)));
    })?;
    Some(size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rapidr_value::objects::ops::Op;

    /// A chart's category names are drawn at every scale: the same 8 bars,
    /// the same rectangle, the same names — at 1×, 1.5× and 2× each name's
    /// place holds ink (a name fitted away at one scale only, as device
    /// pixels would do, leaves its place bare). The captures, as the
    /// desktop and the web make them.
    #[test]
    fn a_charts_names_are_drawn_at_every_scale() {
        let names = ["Alpha", "Bravo", "Charlie", "Delta", "Echo", "Foxtrot", "Golf", "Hotel"];
        let mut p = plot::Plot { width: 330, height: 220, ..plot::Plot::default() };
        p.series.push(plot::Series {
            x: (0..8).map(f64::from).collect(),
            y: (0..8).map(|i| 10.0 + 3.0 * f64::from(i)).collect(),
            label: String::new(),
            color: plot::PALETTE[0].into(),
            style: "bar".into(),
            categories: names.map(String::from).to_vec(),
        });
        let (ops, size) = chart::ops(&p, rapidr_value::theme::current());
        let shown: Vec<(String, (i64, i64, i64, i64))> = ops
            .iter()
            .filter_map(|o| match o {
                Op::Text { text, rect, .. } if names.contains(&text.as_str()) => Some((text.clone(), *rect)),
                _ => None,
            })
            .collect();
        // (the names it fits: all of them, or every other one — the same at each scale)
        assert!(shown.len() >= 4, "{shown:?}");
        for scale in [1.0, 1.5, 2.0] {
            let px = render(&ops, size, scale);
            let background = px.pixels[0];
            for (name, (x, y, w, h)) in &shown {
                let (x0, y0) = ((*x as f64 * scale) as usize, (*y as f64 * scale) as usize);
                let (x1, y1) = (((x + w) as f64 * scale).ceil() as usize, ((y + h) as f64 * scale).ceil() as usize);
                let ink = (y0..y1.min(px.height)).flat_map(|r| (x0..x1.min(px.width)).map(move |c| (r, c))).filter(|(r, c)| px.pixels[r * px.width + c] != background).count();
                assert!(ink > 10, "{name} at {scale}x has no ink ({ink} pixels in {x0},{y0} {x1},{y1})");
            }
        }
    }
}
