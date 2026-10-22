//! The debugger's values drawn by the UI kernel (docs/preview4-plan.md S10,
//! phase 1): a Variables / Watch / Call Stack pane of sample values, each
//! row a `value_view::ValueRow` — its kind's icon, its name, its value
//! styled by kind, colour swatches, the changed mark — in RapidR light,
//! dark, high contrast and the classic look side by side, rendered by the
//! CPU renderer (the web host's and the captures').
//!
//!     cargo run --release -p rapidr-ui-render --example value_view_demo -- OUTDIR
//!
//! Writes OUTDIR/debugger-values-1x.png and -2x.png, and prints each row's
//! accessible name (what a screen reader reads).

use rapidr_ui_kernel::components::{list::selected_row, tree::expander};
use rapidr_ui_kernel::paint::Painter;
use rapidr_ui_kernel::value_view::{Hint, RowStyle, ValueInfo, ValueKind, ValueRow};
use rapidr_ui_kernel::{DisplayList, FormUi, MemStore, Place, TextSystem};
use rapidr_ui_render::cpu::CpuRenderer;
use rapidr_value::ide_theme::chrome_font;
use rapidr_value::theme::{Theme, CLASSIC, RAPIDR, RAPIDR_DARK, RAPIDR_HIGH_CONTRAST};

/// A row of the sample tree: its depth, whether it has children (and is
/// open), the row.
struct Line {
    depth: i64,
    open: Option<bool>,
    selected: bool,
    row: ValueRow,
}

fn value(name: &str, kind: &str, text: &str) -> ValueInfo<'static> {
    // (the samples live for the whole run)
    let leak = |s: &str| -> &'static str { Box::leak(s.to_string().into_boxed_str()) };
    ValueInfo { name: leak(name), kind: leak(kind), text: leak(text), ..Default::default() }
}

fn sample() -> Vec<Line> {
    let l = |depth, open, row| Line { depth, open, selected: false, row };
    let mut v = vec![
        l(0, Some(true), ValueRow::section(ValueKind::Locals, "Locals")),
        l(1, None, ValueRow::new(&value("w", "Integer", "0"))),
        l(1, None, ValueRow::new(&ValueInfo { changed: true, ..value("Lives", "Integer", "2") })),
        l(1, None, ValueRow::new(&value("Ratio", "Double", "0.75"))),
        l(1, None, ValueRow::new(&value("Title", "String", "\"Hello, RapidR\""))),
        l(1, None, ValueRow::new(&value("AccessibleName", "String", "\"\""))),
        l(1, None, ValueRow::new(&value("Done", "Boolean", "True"))),
        l(1, Some(false), ValueRow::new(&ValueInfo { count: 65, ..value("ColName", "Array", "Array(0 TO 64)") })),
        l(1, Some(false), ValueRow::new(&ValueInfo { count: 2, ..value("p", "Point", "Point") })),
        l(1, None, ValueRow::new(&value("Pending", "Empty", ""))),
        l(1, None, ValueRow::new(&ValueInfo { result: true, ..value("Average", "Double", "9.5") })),
        l(0, Some(true), ValueRow::section(ValueKind::Globals, "Globals")),
        l(1, Some(true), ValueRow::new(&value("Stats", "RListView", "\"Stats\" (RListView)"))),
        l(2, None, ValueRow::new(&ValueInfo { hint: Hint::Color, ..value("FontColor", "Integer", "-2147483640") })),
        l(2, None, ValueRow::new(&ValueInfo { hint: Hint::Color, changed: true, ..value("Color", "Integer", "255") })),
        l(2, None, ValueRow::new(&ValueInfo { hint: Hint::Color, ..value("BackColor", "Integer", &0x0080FF.to_string()) })),
        l(2, None, ValueRow::new(&ValueInfo { hint: Hint::Constant("alClient"), ..value("Align", "Integer", "5") })),
        l(2, None, ValueRow::new(&ValueInfo { hint: Hint::Handle, ..value("Handle", "Integer", "133082") })),
        l(1, Some(false), ValueRow::new(&value("Sender", "RButton", "\"Button1\" (RButton)"))),
        l(1, Some(false), ValueRow::new(&value("$INCLUDE RAPIDQ.INC", "Include", "(412 names)"))),
        l(0, Some(true), ValueRow::section(ValueKind::Watch, "Watch")),
        l(1, None, ValueRow::new(&value("Score * 2", "Integer", "84"))),
        l(1, Some(false), ValueRow::new(&ValueInfo { count: 3, ..value("sales", "RDataFrame", "\"sales\" (RDataFrame: 3 rows × 2 columns)") })),
        l(0, Some(true), ValueRow::section(ValueKind::CallStack, "Call Stack")),
        l(1, None, ValueRow::section(ValueKind::CurrentFrame, "UpdateScore  Main.rr:42")),
        l(1, None, ValueRow::section(ValueKind::Frame, "Button1Click  Main.rr:17")),
    ];
    // (Sender picked: the selection's look)
    if let Some(s) = v.iter_mut().find(|l| l.row.text().starts_with("Sender")) {
        s.selected = true;
    }
    v
}

const PANE_W: i64 = 400;
const ROW_H: i64 = 20;
const INDENT: i64 = 16;
const TITLE_H: i64 = 26;
const GAP: i64 = 16;

fn pane(p: &mut Painter, lines: &[Line], label: &str) {
    let t: &Theme = p.theme();
    let font = chrome_font(t);
    let h = TITLE_H + lines.len() as i64 * ROW_H + 8;
    // the pane: its title on the face, the rows on the window
    p.fill((0, 0, PANE_W, h), t.face);
    p.text((10, 0, PANE_W - 20, TITLE_H), label, &font, t.text, Place::Left);
    p.fill((0, TITLE_H, PANE_W, h - TITLE_H), t.window);
    p.frame((0, 0, PANE_W, h), if t.fluent() { t.border } else { t.shadow });
    for (k, line) in lines.iter().enumerate() {
        let top = TITLE_H + 4 + k as i64 * ROW_H;
        let left = 6 + line.depth * INDENT;
        let ink = line.selected.then(|| selected_row(p, (2, top, PANE_W - 4, ROW_H), true));
        if let Some(open) = line.open {
            expander(p, left + 5, top + ROW_H / 2, 9, open);
        }
        let style = RowStyle { selected_ink: ink, ..RowStyle::new(&font) };
        line.row.paint(p, (left + 14, top, PANE_W - left - 20, ROW_H), &style);
    }
}

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    std::fs::create_dir_all(&out).expect("the output folder");
    let lines = sample();
    for l in &lines {
        println!("{:width$}{}", "", l.row.accessible_name(), width = (l.depth * 2) as usize);
    }
    let themes: [(&'static Theme, &str); 4] = [(&RAPIDR, "RapidR light"), (&RAPIDR_DARK, "RapidR dark"), (&RAPIDR_HIGH_CONTRAST, "High contrast"), (&CLASSIC, "Classic")];
    let pane_h = TITLE_H + lines.len() as i64 * ROW_H + 8;
    let size = (GAP + themes.len() as i64 * (PANE_W + GAP), pane_h + 2 * GAP);
    let mut s = MemStore::new();
    s.add("demo", "RFORM", None);
    let form = FormUi::build(&s, "demo", false);
    let mut ts = TextSystem::new();
    for scale in [1.0, 2.0] {
        let mut list = DisplayList { size, scale, ..Default::default() };
        {
            let mut p = Painter::new(&mut list);
            p.fill((0, 0, size.0, size.1), 0x9AA3B5);
            for (i, (theme, label)) in themes.iter().enumerate() {
                let x = GAP + i as i64 * (PANE_W + GAP);
                p.at((x, GAP), |p| p.in_theme(theme, |p| pane(p, &lines, &format!("Variables · {label}"))));
            }
        }
        let (w, h) = rapidr_ui_render::canvas::device_size(&list);
        let mut r = CpuRenderer::new(w, h);
        r.render(w, h, &list, &mut ts, &form);
        let png = rapidr_value::objects::codec::encode_png(&r.pixels()).expect("a PNG");
        let path = format!("{out}/debugger-values-{}x.png", scale as i64);
        std::fs::write(&path, png).expect("written");
        println!("{path}");
    }
}
