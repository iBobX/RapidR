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
