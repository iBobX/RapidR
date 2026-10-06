//! RPLOT's charts as pixels — the one implementation every runtime uses:
//! the chart model (`rapidr_value::datascience::plot`) draws the kernel's
//! ops, and the CPU renderer turns them into pixels at any scale, with the
//! kernel's fonts. `SaveFig` writes [`png`]; a QIMAGE's `LoadFromPlot`
//! takes [`load_into_picture`]: the 1× pixels a program reads (`Pixel`)
//! and the chart drawn again at the screen's scale for what it shows.

use std::cell::RefCell;
use std::rc::Rc;

use rapidr_ui_kernel::TextSystem;
use rapidr_value::datascience::plot;
use rapidr_value::objects::bitmap::Redraw;
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

/// Chart `name` in the current theme: its ops and size.
pub fn ops(name: &str) -> (Vec<Op>, (i64, i64)) {
    plot::ops(name, rapidr_value::theme::current())
}

/// Chart `name` as a PNG, `scale` device pixels per pixel (1: its size).
pub fn png(name: &str, scale: f64) -> Result<Vec<u8>, String> {
    let (ops, size) = ops(name);
    encode_png(&render(&ops, size, scale))
}

/// Chart `plot` into picture `picture` (a QIMAGE's): its pixels at 1×, and
/// drawn again at the screen's scale for the screen. The chart's size.
pub fn load_into_picture(picture: &str, plot: &str) -> Option<(i64, i64)> {
    let (ops, size) = ops(plot);
    let lo = render(&ops, size, 1.0);
    let ops = Rc::new(ops);
    let redraw = Redraw(Rc::new(move |s: usize| Some(render(&ops, size, s as f64))));
    rapidr_value::objects::with_picture(picture, |b| b.load_drawn(lo, redraw))?;
    Some(size)
}
