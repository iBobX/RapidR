//! A program's forms as RapidR Studio's designer draws them (docs/ide-plan.md
//! I4, lane L-DVIEW), saved as BMPs — what `tools/visual/designer_wysiwyg.py`
//! compares with the running program's own captures (`RAPIDR_CAPTURE`):
//!
//! ```text
//! cargo run -p rapidr-designer --example design_capture -- FILE.bas PREFIX [--scale 2] [--chrome]
//! ```
//!
//! Each form the file CREATEs is opened in the designer (`Document`, the
//! source read as RapidR Studio reads it), shown on an RDESIGNSURFACE in a
//! form of its own and drawn by the UI kernel and the CPU renderer — the
//! headless desktop host's capture path. Written per form: `PREFIX-NAME.bmp`
//! (the designed form's inside: its menu bar and client area, what a
//! running program's capture is) and `PREFIX-NAME-surface.bmp` (the whole
//! surface: backdrop, frame, tray). Without `--chrome` the selection and the
//! grid are hidden (the designed form alone); with it, the first component
//! is selected. The theme is `RAPIDR_THEME`'s, as for programs. One line per
//! form on stdout: `NAME<TAB>caption<TAB>inside WxH`.

use std::path::Path;

use rapidr_designer::Document;
use rapidr_preprocessor::PreprocessOptions;
use rapidr_ui_kernel::{FormUi, MemStore, TextSystem};
use rapidr_value::designer::Designer;
use rapidr_value::objects::codec::{encode_bmp, Pixels};
use rapidr_value::objects::design::{DesignSurface, MARGIN, TRAY_GAP, TRAY_H};
use rapidr_value::v_int;

/// The surface showing `designer`'s form, drawn at `scale`: the whole
/// surface, and the form's inside cut from it.
fn capture(designer: Designer, scale: f64, chrome: bool) -> (Pixels, Pixels, String) {
    let mut d = DesignSurface::with_designer(designer);
    d.show_grid = chrome;
    d.show_selection = chrome;
    if chrome && !d.ids().is_empty() {
        d.select(0);
    }
    let (fx, fy, fw, fh) = d.form_rect();
    let tray = if d.tray().is_empty() { 0 } else { TRAY_GAP + TRAY_H };
    let (sw, sh) = (fx + fw + MARGIN, fy + fh + tray + MARGIN);
    let (ox, oy) = d.client_origin();
    let menu = d.menu_height();
    let (cw, ch) = d.client_size();
    let title = d.title();
    let mut store = MemStore::new();
    store.add("wysiwyghost", "RFORM", None).set("wysiwyghost", "clientwidth", v_int(sw)).set("wysiwyghost", "clientheight", v_int(sh));
    store.add("wysiwygsurface", "RDESIGNSURFACE", Some("wysiwyghost"));
    for (p, v) in [("left", 0), ("top", 0), ("width", sw), ("height", sh)] {
        store.set("wysiwygsurface", p, v_int(v));
    }
    rapidr_value::objects::with_design_mut("wysiwygsurface", |s| *s = d);
    let mut ts = TextSystem::new();
    // (bitmaps a component draws — a list view's rows — at the screen's
    // scale, as the hosts set it for a form's)
    rapidr_value::objects::bitmap::set_display_scale(scale);
    let mut f = FormUi::build(&store, "wysiwyghost", false);
    let list = f.paint(&store, &mut ts, scale);
    let px = rapidr_ui_render::cpu::capture(&list, &mut ts, &f);
    // (the inside: from the client's top left, less the menu bar)
    let s = |v: i64| ((v as f64) * scale).round() as usize;
    let (x0, y0, w, h) = (s(ox), s(oy - menu), s(cw), s(ch + menu));
    let mut inside = Pixels { width: w, height: h, pixels: Vec::with_capacity(w * h) };
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            inside.pixels.push(if x < px.width && y < px.height { px.pixels[y * px.width + x] } else { 0 });
        }
    }
    store.clear();
    (px, inside, title)
}

/// The program the designer's drawing of `designer`'s form is compared with:
/// the original program's directives and constants (`$INCLUDE`, `CONST` …:
/// the CREATE blocks read their constants from them), then the form's CREATE
/// blocks as the designer reads them — the assignments and statements it
/// replays, nothing it can't (an event handler's name, `Width = Screen.Width
/// / 2`, `Visible = False`: the designer shows a hidden component, as
/// Delphi's does) — then `Name.ShowModal`.
fn create_only(source: &str, designer: &Designer) -> String {
    use rapidr_value::designer::{SubItem, Subtree};
    fn keep(design: &rapidr_value::designer::FormDesign, t: &Subtree) -> Subtree {
        let body = t
            .body
            .iter()
            .filter_map(|i| match i {
                SubItem::Prop(p) => {
                    let key = rapidr_value::designer::model::prop_key(&p.name);
                    let readable = matches!(design.read_value(&p.value), rapidr_value::designer::value::PropValue::Number(_) | rapidr_value::designer::value::PropValue::Str(_));
                    (readable && key != "visible" && key != "parent").then(|| i.clone())
                }
                SubItem::Code(c) => rapidr_ui_kernel::components::design::statement(design, c).map(|_| i.clone()),
                SubItem::Child(c) => Some(SubItem::Child(keep(design, c))),
            })
            .collect();
        Subtree { body, ..t.clone() }
    }
    let design = &designer.design;
    let mut out = String::new();
    for line in source.lines() {
        let t = line.trim_start().to_ascii_uppercase();
        if ["$INCLUDE", "$DEFINE", "$ESCAPECHARS", "$OPTION", "CONST "].iter().any(|k| t.starts_with(k)) {
            out.push_str(line.trim_end());
            out.push('\n');
        }
    }
    let Some(tree) = design.subtree(design.root()) else { return out };
    out.push_str(&rapidr_value::designer::text::write_create(&keep(design, &tree), "", &rapidr_value::designer::text::Style::default()));
    out.push_str(&format!("\n{}.ShowModal\n", tree.name));
    out
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut scale = 1.0;
    let mut chrome = false;
    let mut plain = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--scale" => {
                i += 1;
                scale = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(1.0);
            }
            "--chrome" => chrome = true,
            a => plain.push(a.to_string()),
        }
        i += 1;
    }
    let [file, prefix] = <[String; 2]>::try_from(plain).unwrap_or_else(|_| {
        eprintln!("usage: design_capture FILE.bas PREFIX [--scale 2] [--chrome]");
        std::process::exit(2)
    });
    let path = Path::new(&file);
    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("{file}: {e}");
        std::process::exit(1)
    });
    let options = PreprocessOptions { include_dirs: path.parent().map(|p| vec![p.join("include")]).unwrap_or_default(), ..PreprocessOptions::default() };
    let mut doc = Document::open_bytes(&bytes, Some(path), options);
    for k in 0..doc.forms().len() {
        let name = doc.forms()[k].name().to_string();
        let Some(designer) = doc.designer(k).map(|d| d.clone()) else { continue };
        // (a form: a dialog or a QFONT CREATEd at the top isn't designed)
        if designer.design.node(designer.design.root()).is_none_or(|n| !n.is_form()) {
            continue;
        }
        let program = format!("{prefix}-{}.bas", name.to_lowercase());
        if let Err(e) = std::fs::write(&program, create_only(&String::from_utf8_lossy(&bytes), &designer)) {
            eprintln!("{program}: {e}");
        }
        let (surface, inside, title) = capture(designer, scale, chrome);
        for (suffix, px) in [("", &inside), ("-surface", &surface)] {
            let out = format!("{prefix}-{}{suffix}.bmp", name.to_lowercase());
            if let Err(e) = std::fs::write(&out, encode_bmp(px)) {
                eprintln!("{out}: {e}");
            }
        }
        println!("{}\t{}\t{}x{}", name.to_lowercase(), title, inside.width, inside.height);
    }
}
