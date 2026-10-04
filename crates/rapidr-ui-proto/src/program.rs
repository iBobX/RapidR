//! The spike's "program" (Stage 0 spike A): plain blocking Rust, as
//! generated native code is — what this RapidQ source would compile to:
//!
//! ```basic
//! Form1.Show                         ' the demo form (demo.rs)
//! r = Form2.ShowModal                ' a 50 ms QTIMER ticks lblTicks meanwhile
//! PRINT "form2 returned "; r         ' only after Form2 closed
//! ' Form2's btnNested.OnClick:  r3 = Form3.ShowModal  (a modal inside a handler)
//! ' Form2's btnOpen.OnClick:    file$ = OpenDialog (async, the windows keep painting)
//! ```
//!
//! Handlers run in `ui::step`, after the pump returned; a handler that
//! calls `show_modal` pumps again from there (nested), and returns the
//! nested form's ModalResult to its caller.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use crate::form::{Component, Form, Kind};
use crate::kernel::FormId;
use crate::{demo, menu, ui};

/// What `--script` does when no script file is given (the same script for
/// the windowed and the headless host).
pub const SCRIPT: &str = "\
wait 400
print form2 is modal: a click on form1's OK must be dropped
click form1 btnOK
expect form1 lblStatus Ready.
click form2 btnNested
wait 300
print form3 (a modal shown by btnNested's OnClick handler) is up; form2's timer still ticks
expect form2 lblTicks ticking
expect form3 lblMsg modal inside a handler
capture
key form3 Tab
click form3 btnOK
wait 60
expect form2 lblResult nested ShowModal returned 1
click form2 btnOpen
wait 100
expect form2 lblResult file:
menu file.ping
wait 60
expect form2 lblResult menu: file.ping
resize form2 420 260
wait 100
capture
click form2 btnDoneRight
wait 100
expect form1 lblStatus Ready.
close form1
";

fn label(name: &str, left: i64, top: i64, width: i64, text: &str) -> Component {
    let mut c = Component::new(name, "RLABEL", left, top, Kind::Label { caption: text.into() });
    c.width = width;
    c
}

fn button(name: &str, left: i64, top: i64, width: i64, text: &str) -> Component {
    let mut c = Component::new(name, "RBUTTON", left, top, Kind::Button { caption: text.into() });
    c.width = width;
    c
}

fn form2() -> Form {
    let mut f = Form::new("Form2 (ShowModal)");
    f.add(label("lblTicks", 8, 10, 200, "waiting"));
    f.add(button("btnNested", 8, 34, 90, "Nested..."));
    f.add(button("btnOpen", 104, 34, 90, "Open..."));
    f.add(button("btnDoneRight", 0, 34, 75, "Done"));
    f.add(label("lblResult", 8, 70, 300, ""));
    let mut log = label("lblLog", 8, 92, 300, "");
    log.height = 80;
    f.add(log);
    crate::kernel::layout(&mut f);
    f
}

fn form3() -> Form {
    let mut f = Form::new("Form3 (nested modal)");
    f.width = 260;
    f.height = 90;
    f.add(label("lblMsg", 8, 10, 240, "modal inside a handler"));
    f.add(button("btnOK", 8, 40, 75, "OK"));
    f.add(button("btnCancel", 92, 40, 75, "Cancel"));
    f
}

fn log(f: FormId, line: &str) {
    let mut lines: Vec<String> = ui::caption(f, "lblLog").lines().map(str::to_string).collect();
    lines.push(line.to_string());
    let keep = lines.len().saturating_sub(4);
    ui::set_caption(f, "lblLog", &lines[keep..].join("\n"));
    ui::set_caption(f, "lblResult", line);
    eprintln!("[program] {line}");
}

pub fn main() {
    // Form1: the demo form, its handlers the demo's.
    let f1 = ui::create_form("form1", demo::form());
    let clicks = Rc::new(Cell::new(0u32));
    {
        let clicks = clicks.clone();
        ui::on_any(f1, move |ev| {
            let mut n = clicks.get();
            ui::with_form(f1, |form| demo::handle(form, std::slice::from_ref(ev), &mut n));
            clicks.set(n);
        });
    }
    let f2 = ui::create_form("form2", form2());
    let f3 = ui::create_form("form3", form3());
    ui::set_button_result(f2, "btnDoneRight", 1);
    ui::set_button_result(f3, "btnOK", 1);
    ui::set_button_result(f3, "btnCancel", 2);

    // A 50 ms timer on form2.
    let ticks = Rc::new(Cell::new(0u64));
    {
        let ticks = ticks.clone();
        ui::add_timer(Duration::from_millis(50), move || {
            ticks.set(ticks.get() + 1);
            if ui::shown(f2) && ui::caption(f2, "lblTicks") != "ticking" {
                ui::set_caption(f2, "lblTicks", "ticking");
            }
        });
    }

    // btnNested.OnClick: a ShowModal inside a handler.
    let nested = Rc::new(RefCell::new(Vec::<(i64, u64)>::new()));
    {
        let (nested, ticks) = (nested.clone(), ticks.clone());
        ui::on(f2, "btnNested", "click", move || {
            let before = ticks.get();
            let r = ui::show_modal(f3);
            let during = ticks.get() - before;
            nested.borrow_mut().push((r, during));
            eprintln!("[program] (nested modal: {during} timer ticks meanwhile)");
            log(f2, &format!("nested ShowModal returned {r} (timer ticked meanwhile: {})", during > 0));
        });
    }
    // btnOpen.OnClick: an async file dialog, waited for by pumping.
    let files = Rc::new(RefCell::new(Vec::new()));
    {
        let files = files.clone();
        ui::on(f2, "btnOpen", "click", move || {
            let p = ui::open_file_dialog(f2);
            log(f2, &format!("file: {}", p.as_ref().map_or("(cancelled)".to_string(), |p| p.display().to_string())));
            files.borrow_mut().push(p);
        });
    }
    ui::on_resize(f2, move |w, h| log(f2, &format!("OnResize {w}x{h}")));
    let menus = Rc::new(RefCell::new(Vec::<String>::new()));
    {
        let menus = menus.clone();
        ui::on_menu(move |id| {
            menus.borrow_mut().push(id.to_string());
            let target = if ui::shown(f2) { f2 } else { f1 };
            log(target, &format!("menu: {id}"));
            match id {
                menu::OPEN => {
                    let p = ui::open_file_dialog(target);
                    log(target, &format!("file: {p:?}"));
                }
                menu::OPEN_BETWEEN => {
                    let p = ui::async_dialog_between_pumps();
                    log(target, &format!("async-between-pumps file: {p:?}"));
                }
                menu::BLOCKING => {
                    let p = ui::blocking_dialog_between_pumps();
                    log(target, &format!("blocking file: {p:?}"));
                }
                menu::BLOCKING_CB => ui::blocking_dialog_in_callback(),
                menu::EXIT => ui::end(0),
                _ => {}
            }
        });
    }

    // ---- the program's main, as written ----
    ui::show(f1);
    let r = ui::show_modal(f2);
    println!("main: form2.ShowModal returned {r}");
    for (k, (r3, during)) in nested.borrow().iter().enumerate() {
        println!("main: nested #{} form3.ShowModal (inside btnNested.OnClick) returned {r3}; timer ticked during it: {}", k + 1, *during > 0);
    }
    for p in files.borrow().iter() {
        println!("main: file dialog -> {}", p.as_ref().map_or("(cancelled)".to_string(), |p| p.display().to_string()));
    }
    println!("main: menu picks {:?}", menus.borrow());
    let (total, late, held, pumps, nesting, redraws) = ui::stats();
    println!("main: timer ticks > 0: {}; deepest ShowModal nesting {nesting}", total > 0);
    eprintln!("[stats] {total} ticks, worst lateness {:.1} ms, {pumps} pumps, {held} held pumps, {redraws} redraws (host {})", late.as_secs_f64() * 1e3, ui::host_name());
    // The main loop, until Form1 closes; then the process ends.
    ui::run();
    let (total, late, held, pumps, _, redraws) = ui::stats();
    eprintln!("[stats] end: {total} ticks, worst lateness {:.1} ms, {pumps} pumps, {held} held pumps, {redraws} redraws", late.as_secs_f64() * 1e3);
    eprintln!("[host] {}", ui::host_report());
    let (frames, time, live) = ui::frame_stats();
    if frames > 0 {
        eprintln!("[stats] {frames} window frames by vello_cpu + softbuffer, average {:.2} ms (render + copy + present)", time.as_secs_f64() * 1e3 / frames as f64);
    }
    if live > 0 {
        eprintln!("[stats] {live} Resized events arrived with the window in live resize");
    }
    for h in ui::held_pumps() {
        eprintln!("[stats] held pump: {:.0} ms over, {} Resized, {} redraws, sizes {:?}", h.over.as_secs_f64() * 1e3, h.resized, h.redraws, h.sizes);
    }
    if let Some(n) = ui::script_failures() {
        println!("script: {} ({} failed expectation(s))", if n == 0 { "PASS" } else { "FAIL" }, n);
        ui::end(if n == 0 { 0 } else { 1 });
    }
    ui::end(0);
}
