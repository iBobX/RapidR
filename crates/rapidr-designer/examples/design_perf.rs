//! The designer's frame time while dragging on a form of 300 components
//! (docs/ide-plan.md §6.2: 60 frames per second — 16.7 ms a frame): each
//! step moves the mouse, then the surface is drawn again by the UI kernel
//! (the display list a host renders) and, with `--render`, rasterized by the
//! CPU renderer too (the headless host's; a window's GPU renders faster).
//!
//! ```text
//! cargo run --release -p rapidr-designer --example design_perf [-- --render] [--scale 2]
//! ```

use std::time::Instant;

use rapidr_ui_kernel::{FormUi, MemStore, Mods, TextSystem};
use rapidr_value::designer::{Designer, FormDesign, Prop, SubItem, Subtree};
use rapidr_value::objects::design::DesignSurface;
use rapidr_value::v_int;

/// A form of `n` components of many kinds in a grid, a few with items.
fn big_form(n: usize) -> FormDesign {
    let kinds = ["QBUTTON", "QLABEL", "QEDIT", "QCHECKBOX", "QRADIOBUTTON", "QCOMBOBOX", "QLISTBOX", "QPANEL", "QGROUPBOX", "QTRACKBAR", "QGAUGE", "QCOOLBTN"];
    let p = |n: &str, v: String| SubItem::Prop(Prop { name: n.into(), value: v });
    let mut body = vec![p("Caption", "\"300 components\"".into()), p("Width", "1210".into()), p("Height", "760".into())];
    for i in 0..n {
        let (col, row) = (i % 15, i / 15);
        let ty = kinds[i % kinds.len()];
        let mut b = vec![p("Left", (8 + col as i64 * 80).to_string()), p("Top", (8 + row as i64 * 36).to_string()), p("Width", "72".into()), p("Height", "28".into())];
        if !matches!(ty, "QEDIT" | "QCOMBOBOX" | "QLISTBOX" | "QTRACKBAR" | "QGAUGE") {
            b.push(p("Caption", format!("\"C{i}\"")));
        }
        if matches!(ty, "QCOMBOBOX" | "QLISTBOX") {
            b.push(SubItem::Code("AddItems \"one\", \"two\", \"three\"".into()));
        }
        body.push(SubItem::Child(Subtree { id: 0, name: format!("C{i}"), type_written: ty.into(), body: b }));
    }
    FormDesign::from_subtree(Subtree { id: 0, name: "Big".into(), type_written: "QFORM".into(), body })
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let render = args.iter().any(|a| a == "--render");
    let scale: f64 = args.iter().position(|a| a == "--scale").and_then(|i| args.get(i + 1)).and_then(|s| s.parse().ok()).unwrap_or(1.0);
    rapidr_value::objects::bitmap::set_display_scale(scale);
    let mut store = MemStore::new();
    let (sw, sh) = (1240, 820);
    store.add("perfhost", "RFORM", None).set("perfhost", "clientwidth", v_int(sw)).set("perfhost", "clientheight", v_int(sh));
    store.add("perfds", "RDESIGNSURFACE", Some("perfhost"));
    for (p, v) in [("left", 0), ("top", 0), ("width", sw), ("height", sh)] {
        store.set("perfds", p, v_int(v));
    }
    let t0 = Instant::now();
    rapidr_value::objects::with_design_mut("perfds", |s| *s = DesignSurface::with_designer(Designer::new(big_form(300))));
    let mut ts = TextSystem::new();
    let mut f = FormUi::build(&store, "perfhost", false);
    let first = f.paint(&store, &mut ts, scale);
    let mut cpu = rapidr_ui_render::cpu::CpuRenderer::new(1, 1);
    let (dw, dh) = rapidr_ui_render::canvas::device_size(&first);
    if render {
        cpu = rapidr_ui_render::cpu::CpuRenderer::new(dw, dh);
        cpu.render(dw, dh, &first, &mut ts, &f);
    }
    println!("first frame (store built, laid out, drawn): {:.1} ms, {} items", t0.elapsed().as_secs_f64() * 1000.0, first.items.len());
    // a press on the first button (client 8, 8 → its surface place), then 120 moves
    let (ox, oy) = rapidr_value::objects::with_design("perfds", |d| d.client_origin()).unwrap();
    let (x0, y0) = ((ox + 20) as f64 + 0.5, (oy + 20) as f64 + 0.5);
    f.mouse_down(&store, &mut ts, x0, y0, rapidr_value::input::Button::Left, Mods::NONE);
    let mut frames = Vec::new();
    let mut renders = Vec::new();
    for k in 1..=120 {
        let t = Instant::now();
        f.mouse_move(&store, &mut ts, x0 + k as f64 * 3.0, y0 + k as f64 * 2.0, Mods::NONE);
        let list = f.paint(&store, &mut ts, scale);
        frames.push(t.elapsed().as_secs_f64() * 1000.0);
        if render {
            let t = Instant::now();
            cpu.render(dw, dh, &list, &mut ts, &f);
            renders.push(t.elapsed().as_secs_f64() * 1000.0);
        }
    }
    f.mouse_up(&store, &mut ts, x0 + 363.0, y0 + 242.0, rapidr_value::input::Button::Left, Mods::NONE);
    let stats = |v: &mut Vec<f64>| {
        v.sort_by(f64::total_cmp);
        (v[v.len() / 2], v[v.len() * 95 / 100], v[v.len() - 1])
    };
    let (p50, p95, max) = stats(&mut frames);
    println!("drag frame (move + kernel paint), 120 steps: p50 {p50:.2} ms, p95 {p95:.2} ms, max {max:.2} ms");
    if render {
        let (p50, p95, max) = stats(&mut renders);
        println!("CPU render of the frame ({dw}x{dh} px): p50 {p50:.2} ms, p95 {p95:.2} ms, max {max:.2} ms");
    }
}
