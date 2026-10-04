//! The spike's forms: three of `tests/fixtures`' GUI forms built in a
//! [`MemStore`], as the kernel's own tests build theirs — what the
//! program's `CREATE … END CREATE` blocks and its first statements leave
//! in the property store. The browser (`lib.rs`) and the desktop's capture
//! (`examples/desktop_capture.rs`) build them with this same code, so the
//! pixel comparison compares renderers, not forms.
//!
//! Ids differ between the forms (the shared models are one registry per
//! process, as in a program).
//!
//! A few lines of "program" ([`Program::dispatch`]) answer their events as
//! the fixtures' handlers do, so the page shows the kernel → program →
//! store → kernel loop the runtime will run.

use rapidr_ui_kernel::{KernelEvent, MemStore, Store};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::{with_list, with_tabcontrol, with_trackbar};
use rapidr_value::{v_int, v_str, Value};

/// The forms, by name: (name, the form's id).
pub const FORMS: [(&str, &str); 3] = [("trackbar", "tbform"), ("texts", "txform"), ("lists", "lsform")];

/// A form the size of an IDE's window (1200 × 760, 187 components), for
/// the per-frame cost of a big window; not on the page unless asked
/// (`?forms=big`).
pub const BIG: (&str, &str) = ("big", "bgform");

/// Form `name`'s id.
pub fn form_id(name: &str) -> Option<&'static str> {
    FORMS.iter().chain([&BIG]).find(|(n, _)| *n == name).map(|(_, id)| *id)
}

fn place(s: &mut MemStore, id: &str, (l, t, w, h): (i64, i64, i64, i64)) {
    s.set(id, "left", v_int(l)).set(id, "top", v_int(t));
    if w > 0 {
        s.set(id, "width", v_int(w));
    }
    if h > 0 {
        s.set(id, "height", v_int(h));
    }
}

/// A drawing method of a QCANVAS, its size read from the store (as the
/// runtimes call it).
fn draw(s: &MemStore, id: &str, method: &str, args: &[Value]) {
    let reader = |i: &str, p: &str| s.get(i, p);
    rapidr_value::objects::call(&id.to_lowercase(), method, args, &reader);
}

/// Form `name` as the fixture's program leaves it when it shows the form;
/// `None` for a form the spike doesn't have. Canvases are drawn at the
/// display scale set before (`objects::bitmap::set_display_scale`), as a
/// program's OnPaint draws after the window shows.
pub fn build(name: &str) -> Option<MemStore> {
    let mut s = MemStore::new();
    match name {
        // tests/fixtures/trackbar.bas
        "trackbar" => {
            s.add("tbform", "RFORM", None).set("tbform", "caption", v_str("trackbar"));
            s.add("tb", "RTRACKBAR", Some("tbform")).set("tb", "width", v_int(200));
            s.add("vt", "RTRACKBAR", Some("tbform"));
            place(&mut s, "vt", (220, 0, 0, 150));
            s.set("vt", "orientation", v_int(1)).set("vt", "tickmarks", v_int(2)).set("vt", "tickstyle", v_int(2));
            s.call("vt", "settick", &[v_int(5)]);
            s.add("tblbl", "RLABEL", Some("tbform"));
            place(&mut s, "tblbl", (0, 60, 200, 0));
            s.add("tbbtn", "RBUTTON", Some("tbform")).set("tbbtn", "caption", v_str("&Report"));
            place(&mut s, "tbbtn", (0, 90, 0, 0));
            // (Lbl.Caption = STR$(Tb.Max) + … : what the program says first)
            let caption = with_trackbar("tb", |t| format!("{}{}{}{}{}", t.max, t.page_size, t.line_size, t.frequency, t.tick_style)).unwrap_or_default();
            s.set("tb", "position", v_int(25));
            let p = with_trackbar("tb", |t| t.position).unwrap_or(0);
            s.set("tblbl", "caption", v_str(&format!("{caption}|{p}")));
            s.set("tb", "position", v_int(3));
        }
        // tests/fixtures/tab_control.bas, with text_edits.bas' edit and memo
        "texts" => {
            s.add("txform", "RFORM", None).set("txform", "caption", v_str("tabs and text"));
            s.set("txform", "clientwidth", v_int(352)).set("txform", "clientheight", v_int(330));
            s.add("tabs", "RTABCONTROL", Some("txform"));
            place(&mut s, "tabs", (0, 0, 340, 200));
            s.call("tabs", "addtabs", &[v_str("Tab 1"), v_str("Tab 2"), v_str("Tab 3")]);
            s.call("tabs", "inserttab", &[v_int(0), v_str("New")]);
            s.call("tabs", "deltabs", &[v_int(3)]);
            s.call("tabs", "tab", &[v_int(0), v_str("First")]);
            s.add("panel0", "RPANEL", Some("tabs")).set("panel0", "caption", v_str("Panel 1"));
            place(&mut s, "panel0", (5, 40, 330, 110));
            // (Align = alBottom in the area under the tabs: TabControl::display)
            let (dx, dy, dw, dh) = with_tabcontrol("tabs", |t| t.display(340, 200, &Font::default())).unwrap_or((4, 26, 332, 170));
            s.add("ed", "REDIT", Some("tabs")).set("ed", "text", v_str("Grüße, ñandú ✓"));
            place(&mut s, "ed", (dx, dy + dh - 21, dw, 21));
            s.add("txlbl", "RLABEL", Some("txform")).set("txlbl", "caption", v_str("Name:"));
            place(&mut s, "txlbl", (0, 210, 40, 0));
            s.add("name", "REDIT", Some("txform")).set("name", "text", v_str("RapidR"));
            place(&mut s, "name", (44, 207, 150, 0));
            s.add("memo", "RMEMO", Some("txform"));
            place(&mut s, "memo", (0, 236, 230, 90));
            s.call("memo", "addstrings", &[v_str("First line of the memo"), v_str("Zweite Zeile — äöü"), v_str("第三行 (fallback font)"), v_str("4"), v_str("5"), v_str("6")]);
            s.add("txbtn", "RBUTTON", Some("txform")).set("txbtn", "caption", v_str("OK")).set("txbtn", "default", v_int(-1));
            place(&mut s, "txbtn", (240, 236, 0, 0));
            s.add("chk", "RCHECKBOX", Some("txform")).set("chk", "caption", v_str("&Check me"));
            place(&mut s, "chk", (240, 270, 100, 0));
        }
        // tests/fixtures/canvas_onpaint.bas, with list_items.bas' list box
        "lists" => {
            s.add("lsform", "RFORM", None).set("lsform", "caption", v_str("canvas and list"));
            s.set("lsform", "clientwidth", v_int(400)).set("lsform", "clientheight", v_int(170));
            s.add("again", "RBUTTON", Some("lsform")).set("again", "caption", v_str("Again"));
            place(&mut s, "again", (10, 100, 0, 0));
            s.add("grow", "RBUTTON", Some("lsform")).set("grow", "caption", v_str("Grow"));
            place(&mut s, "grow", (90, 100, 0, 0));
            s.add("lslbl", "RLABEL", Some("lsform")).set("lslbl", "caption", v_str("-"));
            place(&mut s, "lslbl", (10, 130, 200, 0));
            s.add("cv", "RCANVAS", Some("lsform")).set("cv", "color", v_int(0x00FF00));
            place(&mut s, "cv", (10, 10, 200, 80));
            s.add("lst", "RLISTBOX", Some("lsform"));
            place(&mut s, "lst", (220, 10, 170, 120));
            let fruit = ["Apple", "Banana", "Cherry", "Date", "Elderberry", "Fig", "Grape", "Honeydew", "Kiwi", "Lemon"];
            s.call("lst", "additems", &fruit.map(v_str));
            paint_canvas(&s, 1);
        }
        "big" => {
            s.add("bgform", "RFORM", None).set("bgform", "caption", v_str("a big window"));
            s.set("bgform", "clientwidth", v_int(1200)).set("bgform", "clientheight", v_int(760));
            // (a tool bar's worth of buttons, a grid of captioned edits, two
            // memos and a list: 187 components)
            for i in 0..16 {
                let id = format!("bgb{i}");
                s.add(&id, "RBUTTON", Some("bgform")).set(&id, "caption", v_str(&format!("Button {i}")));
                place(&mut s, &id, (8 + i * 74, 8, 70, 0));
            }
            for r in 0..14 {
                for c in 0..6 {
                    let (l, e) = (format!("bgl{r}_{c}"), format!("bge{r}_{c}"));
                    s.add(&l, "RLABEL", Some("bgform")).set(&l, "caption", v_str(&format!("Field {r}.{c}:")));
                    place(&mut s, &l, (8 + c * 196, 44 + r * 30, 70, 0));
                    s.add(&e, "REDIT", Some("bgform")).set(&e, "text", v_str(&format!("value {}", r * 6 + c)));
                    place(&mut s, &e, (80 + c * 196, 40 + r * 30, 116, 0));
                }
            }
            for (i, id) in ["bgm0", "bgm1"].iter().enumerate() {
                s.add(id, "RMEMO", Some("bgform"));
                place(&mut s, id, (8 + i as i64 * 400, 470, 392, 280));
                let lines: Vec<Value> = (0..40).map(|n| v_str(&format!("Line {n}: the quick brown fox jumps over the lazy dog"))).collect();
                s.call(id, "addstrings", &lines);
            }
            s.add("bglst", "RLISTBOX", Some("bgform"));
            place(&mut s, "bglst", (816, 470, 376, 280));
            let items: Vec<Value> = (0..60).map(|n| v_str(&format!("Item {n}"))).collect();
            s.call("bglst", "additems", &items);
        }
        _ => return None,
    }
    Some(s)
}

/// The canvas fixture's OnPaint (`PaintIt`): a red square, "n" and the
/// paint count on the canvas's green, a line and a circle.
fn paint_canvas(s: &MemStore, paints: i64) {
    draw(s, "cv", "fillrect", &[v_int(0), v_int(0), v_int(20), v_int(20), v_int(0xFF)]);
    draw(s, "cv", "textout", &[v_int(30), v_int(5), v_str(&format!("n {paints}")), v_int(0), v_int(0x00FF00)]);
    draw(s, "cv", "line", &[v_int(0), v_int(79), v_int(199), v_int(30), v_int(0xFF0000)]);
    draw(s, "cv", "circle", &[v_int(150), v_int(40), v_int(25), v_int(0x800080)]);
}

/// The fixtures' handlers, in a few lines: what a program does with the
/// kernel's events (runtime-core's `dispatch` runs the program's SUBs).
#[derive(Default)]
pub struct Program {
    /// The track bar's OnChange log (trackbar.bas' `Log`).
    log: String,
    paints: i64,
}

impl Program {
    /// Event `e` of form `form`, as the program answers it.
    pub fn dispatch(&mut self, s: &mut MemStore, e: &KernelEvent) {
        match e {
            // (the user changed a plain property: runtime-core stores it)
            KernelEvent::Set { id, prop, value } => {
                s.set(id, prop, v_int(*value));
            }
            KernelEvent::Change(id) if id == "tb" => {
                let p = with_trackbar("tb", |t| t.position).unwrap_or(0);
                self.log.push_str(&format!("{p},"));
            }
            KernelEvent::Click(id) if id == "tbbtn" => {
                s.set("tb", "max", v_int(4));
                let p = with_trackbar("tb", |t| t.position).unwrap_or(0);
                let caption = rapidr_ui_kernel::store::string(s, "tblbl", "caption");
                s.set("tblbl", "caption", v_str(&format!("{caption}|{}|{p}", self.log)));
            }
            KernelEvent::Click(id) if id == "again" => {
                self.paints += 1;
                paint_canvas(s, self.paints + 1);
            }
            KernelEvent::Click(id) if id == "grow" => {
                s.set("cv", "width", v_int(205));
            }
            KernelEvent::Click(id) if id == "lst" => {
                let item = with_list("lst", |l| l.item_index).unwrap_or(-1);
                s.set("lslbl", "caption", v_str(&format!("ItemIndex {item}")));
            }
            KernelEvent::Click(id) if id == "txbtn" => {
                let t = rapidr_value::objects::with_textedit("name", |t| t.text()).unwrap_or_default();
                s.set("txlbl", "caption", v_str(&format!("Hi {t}")));
            }
            KernelEvent::Change(id) if id == "tabs" => {
                let i = with_tabcontrol("tabs", |t| t.index).unwrap_or(0);
                s.set("panel0", "caption", v_str(&format!("Panel {}", i + 1)));
            }
            _ => {}
        }
    }
}
