//! The code's helpers in the editor (docs/code-helpers-plan.md): marks and
//! chips take fixed columns and never move what's before them or the rows;
//! the caret stops beside the text; word wrap counts them; Quiet hides the
//! chips and the note; a value editor writes back as one undo step in the
//! code's own form; the line note; the values while paused; Ctrl held.

use rapidr_editor::service::{HelperKind, HelperSite};
use rapidr_value::objects::codehelp::MARK_COLS;
use rapidr_value::objects::{with_code, with_code_mut};
use rapidr_value::{v_int, v_str};

use crate::display::Item;
use crate::{FormUi, MemClipboard, MemStore, Mods, TextSystem};

fn form(id: &str, text: &str) -> (MemStore, FormUi, TextSystem) {
    let mut s = MemStore::new();
    let form = format!("{id}f");
    s.add(&form, "RFORM", None).set(&form, "clientwidth", v_int(900)).set(&form, "clientheight", v_int(600));
    s.add(id, "RCODEEDITOR", Some(&form)).set(id, "left", v_int(8)).set(id, "top", v_int(8)).set(id, "width", v_int(860)).set(id, "height", v_int(400));
    s.set(id, "text", v_str(text));
    let f = FormUi::build(&s, &form, false);
    (s, f, TextSystem::new())
}

fn key(f: &mut FormUi, s: &MemStore, ts: &mut TextSystem, vk: i64, text: &str, mods: Mods) {
    let mut clip = MemClipboard::default();
    f.key_down(s, ts, vk, text, mods, &mut clip);
}

const CMD: Mods = Mods { command: true, ctrl: true, shift: false, alt: false, word: false };

/// A site over the first `what` in `text`.
fn site(text: &str, what: &str, kind: HelperKind, value: &str, form: &str) -> HelperSite {
    let start = text.find(what).unwrap();
    let mut h = HelperSite::new(kind, start, start + what.len());
    h.value = value.into();
    h.form = form.into();
    h
}

fn set_sites(id: &str, sites: Vec<HelperSite>) {
    with_code_mut(id, |c| c.helpers.set_sites(sites, None));
}

/// Where byte `at` is drawn: (x, the row's top), logical pixels.
fn caret_at(f: &FormUi, id: &str, at: usize) -> (f64, f64) {
    let n = f.nodes.iter().find(|n| n.id == id).unwrap();
    let ui = n.ui.code.as_ref().unwrap();
    with_code(id, |c| {
        let sh = super::paint::shown_at(ui, c, at).unwrap();
        (super::paint::caret_x(ui, sh, at) / f.scale, sh.top)
    })
    .unwrap()
}

fn ch(f: &FormUi, id: &str) -> f64 {
    f.nodes.iter().find(|n| n.id == id).unwrap().ui.code.as_ref().unwrap().metrics.ch
}

const CODE: &str = "Button1.Color = RGB(255, 130, 20) ' orange\nButton1.Caption = \"Hi\"\nDrawDot 40, 12\n";

#[test]
fn a_mark_never_moves_what_is_before_it_or_the_rows() {
    let (s, mut f, mut ts) = form("hl1", CODE);
    f.paint(&s, &mut ts, 1.0);
    let value_end = CODE.find(" ' orange").unwrap();
    let before: Vec<(f64, f64)> = [0, CODE.find('=').unwrap(), value_end, CODE.find("Caption").unwrap()].iter().map(|&b| caret_at(&f, "hl1", b)).collect();
    let after_end = caret_at(&f, "hl1", value_end + 1);
    set_sites("hl1", vec![site(CODE, "RGB(255, 130, 20)", HelperKind::Color, &0x1482FF.to_string(), "rgb")]);
    f.paint(&s, &mut ts, 1.0);
    let now: Vec<(f64, f64)> = [0, CODE.find('=').unwrap(), value_end, CODE.find("Caption").unwrap()].iter().map(|&b| caret_at(&f, "hl1", b)).collect();
    assert_eq!(before, now, "the text up to the value's end, and the next line, stay where they were");
    // (after the value: exactly the mark's columns further on)
    let moved = caret_at(&f, "hl1", value_end + 1).0 - after_end.0;
    assert!((moved - MARK_COLS as f64 * ch(&f, "hl1")).abs() < 1.0, "moved {moved}");
    // (another colour: the same width — nothing flickers)
    set_sites("hl1", vec![site(CODE, "RGB(255, 130, 20)", HelperKind::Color, "255", "rgb")]);
    f.paint(&s, &mut ts, 1.0);
    assert_eq!(caret_at(&f, "hl1", value_end + 1), (after_end.0 + moved, after_end.1));
    // (the swatch is drawn, in the theme's own frame)
    let list = f.paint(&s, &mut ts, 1.0);
    assert!(list.items.iter().any(|i| matches!(i, Item::Op { op: rapidr_value::objects::ops::Op::Round { fill: Some(0xFF0000), .. }, .. })), "a red swatch");
}

#[test]
fn the_caret_stops_beside_the_text_never_inside_a_decoration() {
    let (s, mut f, mut ts) = form("hl2", CODE);
    f.focus_id(&s, "hl2");
    let arg = CODE.find("40").unwrap();
    let mut chip = HelperSite::new(HelperKind::ArgName, arg, arg + 2);
    chip.label = "y".into();
    chip.flag = true;
    let value_end = CODE.find(" ' orange").unwrap();
    set_sites("hl2", vec![site(CODE, "RGB(255, 130, 20)", HelperKind::Color, "255", "rgb"), chip]);
    f.paint(&s, &mut ts, 1.0);
    // (the value's end: the caret before the mark, by the `)`)
    let close = caret_at(&f, "hl2", value_end - 1).0;
    let end = caret_at(&f, "hl2", value_end).0;
    assert!((end - close - ch(&f, "hl2")).abs() < 1.0, "right after the `)`");
    // (an argument's start: the caret after its chip, by the `4`)
    let four = caret_at(&f, "hl2", arg).0;
    let comma = caret_at(&f, "hl2", arg - 1).0;
    assert!(four - comma > 3.0 * ch(&f, "hl2"), "the chip is between the space and the 4");
    let next = caret_at(&f, "hl2", arg + 1).0;
    assert!((next - four - ch(&f, "hl2")).abs() < 1.0, "the 4 right after the caret");
    // (Right from the value's end goes one character: past the mark and the space)
    with_code_mut("hl2", |c| c.doc.set_selections(rapidr_editor::Selections::caret(value_end)));
    key(&mut f, &s, &mut ts, 39, "", Mods::NONE);
    assert_eq!(with_code("hl2", |c| c.doc.selections().primary().head).unwrap(), value_end + 1);
}

#[test]
fn word_wrap_counts_the_marks_and_chips() {
    let line = "x = RGB(1, 2, 3) + RGB(4, 5, 6) + RGB(7, 8, 9) + 1 + 2 + 3 + 4 + 5 + 6";
    let (mut s, mut f, mut ts) = form("hl3", line);
    s.set("hl3", "wordwrap", v_int(-1));
    s.set("hl3", "width", v_int(560));
    let mut f2 = FormUi::build(&s, "hl3f", false);
    f2.paint(&s, &mut ts, 1.0);
    let rows = |f: &FormUi| f.nodes.iter().find(|n| n.id == "hl3").unwrap().ui.code.as_ref().unwrap().rows.total();
    let without = rows(&f2);
    let sites: Vec<HelperSite> = ["RGB(1, 2, 3)", "RGB(4, 5, 6)", "RGB(7, 8, 9)"].iter().map(|w| site(line, w, HelperKind::Color, "255", "rgb")).collect();
    set_sites("hl3", sites);
    // (a mark takes room on its row: a line that just fitted wraps sooner)
    let cols = with_code("hl3", |c| super::view::line_inserts(c, 0)).unwrap();
    assert_eq!(cols.len(), 3);
    f2.paint(&s, &mut ts, 1.0);
    assert!(rows(&f2) >= without, "{} rows, {without} before", rows(&f2));
    let pieces = super::view::wrap_line_with(line, 40, 4, &cols);
    let plain = super::view::wrap_line(line, 40, 4);
    assert!(pieces.len() >= plain.len());
    assert!(pieces[0].end <= plain[0].end, "the first row ends sooner: {pieces:?} / {plain:?}");
    let _ = f.paint(&s, &mut ts, 1.0);
}

#[test]
fn quiet_hides_the_chips_and_the_note() {
    let (mut s, mut f, mut ts) = form("hl4", CODE);
    f.focus_id(&s, "hl4");
    let arg = CODE.find("40").unwrap();
    let mut chip = HelperSite::new(HelperKind::ArgName, arg, arg + 2);
    chip.label = "y".into();
    chip.flag = true;
    let mut note = HelperSite::new(HelperKind::Note, 0, CODE.find('\n').unwrap());
    note.label = "The colour behind the button".into();
    set_sites("hl4", vec![note, site(CODE, "RGB(255, 130, 20)", HelperKind::Color, "255", "rgb"), chip]);
    let texts = |list: &crate::display::DisplayList| -> Vec<String> {
        list.items.iter().filter_map(|i| match i {
            Item::Op { op: rapidr_value::objects::ops::Op::Text { text, .. }, .. } => Some(text.clone()),
            _ => None,
        }).collect()
    };
    let list = f.paint(&s, &mut ts, 1.0);
    let t = texts(&list);
    assert!(t.iter().any(|x| x == "The colour behind the button"), "the caret line's note: {t:?}");
    assert!(t.iter().any(|x| x == "y:"), "the chip: {t:?}");
    s.set("hl4", "helpers", v_str("quiet"));
    with_code_mut("hl4", |c| {
        c.set("helpers", &v_str("quiet"));
    });
    let list = f.paint(&s, &mut ts, 1.0);
    let t = texts(&list);
    assert!(!t.iter().any(|x| x == "y:" || x.starts_with("The colour")), "quiet: marks only: {t:?}");
    assert_eq!(with_code("hl4", |c| c.helpers.decos(0, CODE.len()).len()).unwrap(), 1, "the colour's mark stays");
    // (Ctrl held a moment shows every name, even at Quiet; let go, they go)
    let t0 = std::time::Instant::now();
    crate::tick::set_test_now(Some(t0));
    key(&mut f, &s, &mut ts, 17, "", Mods { ctrl: true, ..Mods::NONE });
    crate::tick::set_test_now(Some(t0 + super::helpers::HOLD + std::time::Duration::from_millis(10)));
    f.tick(&s, &mut ts, t0 + super::helpers::HOLD + std::time::Duration::from_millis(10));
    assert!(with_code("hl4", |c| c.helpers.held).unwrap());
    f.key_up(17, Mods::NONE);
    assert!(!with_code("hl4", |c| c.helpers.held).unwrap());
    crate::tick::set_test_now(None);
}

#[test]
fn the_colour_editor_writes_back_in_the_codes_own_form_one_undo_step() {
    let (s, mut f, mut ts) = form("hl5", CODE);
    f.focus_id(&s, "hl5");
    set_sites("hl5", vec![site(CODE, "RGB(255, 130, 20)", HelperKind::Color, &0x1482FF.to_string(), "rgb")]);
    f.paint(&s, &mut ts, 1.0);
    // (the caret on the value; Ctrl+. opens its editor)
    with_code_mut("hl5", |c| c.doc.set_selections(rapidr_editor::Selections::caret(CODE.find("130").unwrap())));
    key(&mut f, &s, &mut ts, 190, ".", CMD);
    assert!(crate::value_edit::is_open("hl5"));
    assert!(with_code("hl5", |c| c.get("helpereditortext").unwrap().to_string_val()).unwrap().starts_with("orange, 255 130 20"));
    let list = f.paint(&s, &mut ts, 1.0);
    assert!(list.items.iter().any(|i| matches!(i, Item::Op { op: rapidr_value::objects::ops::Op::Text { text, .. }, .. } if text == "Colour")), "the editor shows");
    // (Tab to Hex: the wheel, the square, Red, Green, Blue, then Hex)
    for _ in 0..5 {
        key(&mut f, &s, &mut ts, 9, "", Mods::NONE);
    }
    for c in "#0000FF".chars() {
        key(&mut f, &s, &mut ts, 0, &c.to_string(), Mods::NONE);
    }
    key(&mut f, &s, &mut ts, 13, "\r", Mods::NONE);
    assert!(!crate::value_edit::is_open("hl5"));
    let text = with_code("hl5", |c| c.text()).unwrap();
    assert!(text.starts_with("Button1.Color = RGB(0, 0, 255) ' orange\n"), "{text:?}");
    // (one undo step back)
    key(&mut f, &s, &mut ts, 90, "", CMD);
    assert_eq!(with_code("hl5", |c| c.text()).unwrap(), CODE);
}

#[test]
fn esc_drops_and_hex_stays_hex() {
    let code = "Form1.Color = &H0000FF\n";
    let (s, mut f, mut ts) = form("hl6", code);
    f.focus_id(&s, "hl6");
    set_sites("hl6", vec![site(code, "&H0000FF", HelperKind::Color, "255", "hex")]);
    f.paint(&s, &mut ts, 1.0);
    with_code_mut("hl6", |c| c.doc.set_selections(rapidr_editor::Selections::caret(code.find("&H").unwrap())));
    key(&mut f, &s, &mut ts, 190, ".", CMD);
    key(&mut f, &s, &mut ts, 39, "", Mods::SHIFT);
    key(&mut f, &s, &mut ts, 27, "", Mods::NONE);
    assert_eq!(with_code("hl6", |c| c.text()).unwrap(), code, "Esc: nothing written");
    key(&mut f, &s, &mut ts, 190, ".", CMD);
    // (the square: darker; Enter)
    key(&mut f, &s, &mut ts, 9, "", Mods::NONE);
    key(&mut f, &s, &mut ts, 40, "", Mods::SHIFT);
    key(&mut f, &s, &mut ts, 13, "\r", Mods::NONE);
    let text = with_code("hl6", |c| c.text()).unwrap();
    assert!(text.starts_with("Form1.Color = &H0000E6"), "{text:?}");
}

#[test]
fn choices_and_handlers() {
    let code = "SUB SaveClick\nEND SUB\nCREATE Form1 AS RForm\n  Align = alClient\n  CREATE Ok AS RButton\n    OnClick = \n  END CREATE\nEND CREATE\n";
    let (s, mut f, mut ts) = form("hl7", code);
    f.focus_id(&s, "hl7");
    let mut choice = site(code, "alClient", HelperKind::Choice, "alClient", "const");
    choice.choices = ["alNone", "alTop", "alBottom", "alLeft", "alRight", "alClient"].iter().map(|c| c.to_string()).collect();
    let at = code.find("OnClick = ").unwrap() + "OnClick =".len();
    let mut handler = HelperSite::new(HelperKind::Handler, at, at);
    handler.form = "name".into();
    handler.hint = "OkClick".into();
    handler.choices = vec!["SaveClick".into()];
    handler.at = code.find("CREATE Form1").unwrap();
    set_sites("hl7", vec![choice, handler]);
    f.paint(&s, &mut ts, 1.0);
    // (Align: Ctrl+. drops the constants; Up picks alRight)
    with_code_mut("hl7", |c| c.doc.set_selections(rapidr_editor::Selections::caret(code.find("alClient").unwrap() + 2)));
    key(&mut f, &s, &mut ts, 190, ".", CMD);
    key(&mut f, &s, &mut ts, 38, "", Mods::NONE);
    key(&mut f, &s, &mut ts, 13, "\r", Mods::NONE);
    let text = with_code("hl7", |c| c.text()).unwrap();
    assert!(text.contains("  Align = alRight\n"), "{text:?}");
    // (OnClick =: Create handler, the first item)
    let caret = text.find("OnClick = ").unwrap() + "OnClick = ".len();
    let at = text.find("OnClick = ").unwrap() + "OnClick =".len();
    let mut handler = HelperSite::new(HelperKind::Handler, at, at);
    handler.form = "name".into();
    handler.hint = "OkClick".into();
    handler.choices = vec!["SaveClick".into()];
    handler.at = text.find("CREATE Form1").unwrap();
    set_sites("hl7", vec![handler]);
    with_code_mut("hl7", |c| c.doc.set_selections(rapidr_editor::Selections::caret(caret)));
    key(&mut f, &s, &mut ts, 190, ".", CMD);
    assert!(with_code("hl7", |c| c.helpers.editor_text.clone()).unwrap().contains("Create handler OkClick"));
    key(&mut f, &s, &mut ts, 13, "\r", Mods::NONE);
    let text = with_code("hl7", |c| c.text()).unwrap();
    assert!(text.contains("SUB OkClick\n    \nEND SUB\n\nCREATE Form1"), "{text:?}");
    assert!(text.contains("OnClick = OkClick\n"), "{text:?}");
    // (the caret is in the new SUB, ready to type)
    let head = with_code("hl7", |c| c.doc.selections().primary().head).unwrap();
    assert_eq!(&text[head - 4..head + 1], "    \n");
    key(&mut f, &s, &mut ts, 90, "", CMD);
    assert!(with_code("hl7", |c| c.text()).unwrap().contains("OnClick = \n"), "one undo step");
}

#[test]
fn values_while_paused_show_at_the_line_ends() {
    let code = "Score = 40\nScore = Score + 2\nPRINT Name$\n";
    let (mut s, mut f, mut ts) = form("hl8", code);
    s.call("hl8", "showlinevalues", &[v_str("Score\t42\tInteger\nName$\t\"Bob\"\tString"), v_int(1), v_int(2)]);
    let list = f.paint(&s, &mut ts, 1.0);
    let t: Vec<String> = list.items.iter().filter_map(|i| match i {
        Item::Op { op: rapidr_value::objects::ops::Op::Text { text, .. }, .. } => Some(text.clone()),
        _ => None,
    }).collect();
    assert!(t.iter().filter(|x| *x == "42").count() >= 2, "Score's value at lines 1 and 2: {t:?}");
    assert!(!t.iter().any(|x| x == "\"Bob\""), "line 3 is past the paused line: {t:?}");
    with_code_mut("hl8", |c| c.doc.set_selections(rapidr_editor::Selections::caret(12)));
    assert_eq!(with_code("hl8", |c| c.get("linevaluestext").unwrap().to_string_val()).unwrap(), "Score = 42");
    s.call("hl8", "hidelinevalues", &[]);
    assert_eq!(with_code("hl8", |c| c.get("linevaluestext").unwrap().to_string_val()).unwrap(), "");
    let _ = f;
}

#[test]
fn a_missing_picture_is_marked_and_files_stay_in_the_project() {
    let dir = std::env::temp_dir().join(format!("rapidr-helpers-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // (a 2 × 1 BMP: one red, one blue pixel)
    let bmp: Vec<u8> = {
        let mut b = vec![0x42, 0x4D, 62, 0, 0, 0, 0, 0, 0, 0, 54, 0, 0, 0, 40, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 1, 0, 24, 0, 0, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        b.extend_from_slice(&[0, 0, 255, 255, 0, 0, 0, 0]);
        b
    };
    std::fs::write(dir.join("logo.bmp"), &bmp).unwrap();
    let folder = dir.to_string_lossy().into_owned();
    assert!(super::helpers::thumbnail(&folder, "logo.bmp").is_some(), "a picture in the project");
    assert!(super::helpers::thumbnail(&folder, "../logo.bmp").is_none(), "never outside it");
    let h = rapidr_value::objects::codehelp::Helpers::default();
    assert_eq!(super::helpers::file_there(&h, &folder, "logo.bmp"), Some(true));
    assert_eq!(super::helpers::file_there(&h, &folder, "gone.png"), Some(false));
    assert_eq!(super::helpers::file_there(&h, &folder, "/etc/hosts"), None, "outside the project: not even looked at");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Typing latency on a 4,500-line file (docs/preview4-plan.md S7, the
/// helpers' budget: within 5% of the editor without them): a key typed in
/// the middle of the file and the frame painted, 300 times, with the
/// helpers off and with a helper on every line (a colour mark and the
/// argument names, about what RapidRWebServer's code has). The text is the
/// files of the folder RAPIDR_BENCH_DIR (its .rr files joined: the 4,500
/// lines of Preview 4's project), else 4,500 made-up lines.
///
///   cargo test --release -p rapidr-ui-kernel typing_latency -- --ignored --nocapture
#[test]
#[ignore]
fn typing_latency() {
    let text = std::env::var("RAPIDR_BENCH_DIR")
        .ok()
        .and_then(|d| {
            let mut files: Vec<_> = std::fs::read_dir(d).ok()?.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "rr")).collect();
            files.sort();
            Some(files.iter().filter_map(|p| std::fs::read_to_string(p).ok()).collect::<Vec<_>>().join("\n"))
        })
        .unwrap_or_else(|| (0..4500).map(|i| format!("    Canvas.FillRect {i}, 10, 200, 40, RGB({}, 130, 20) ' row {i}", i % 255)).collect::<Vec<_>>().join("\n"));
    // (RAPIDR_BENCH_SITES: the language service's real sites for each of
    // RAPIDR_BENCH_DIR's files — rapidr-langsvc's helpers_cost writes them —
    // joined as the files are, instead of a site on every line)
    let parse = |rows: &str, offset: usize| -> Vec<HelperSite> {
        rows.lines()
            .filter_map(|l| {
                let f: Vec<&str> = l.split('\t').collect();
                let kind = match *f.first()? {
                    "Color" => HelperKind::Color,
                    "Font" => HelperKind::Font,
                    "File" => HelperKind::File,
                    "Choice" => HelperKind::Choice,
                    "Bool" => HelperKind::Bool,
                    "Component" => HelperKind::Component,
                    "Handler" => HelperKind::Handler,
                    "ArgName" => HelperKind::ArgName,
                    _ => HelperKind::Note,
                };
                let mut h = HelperSite::new(kind, offset + f.get(1)?.parse::<usize>().ok()?, offset + f.get(2)?.parse::<usize>().ok()?);
                h.value = f.get(3)?.to_string();
                h.form = f.get(4)?.to_string();
                h.label = f.get(5)?.to_string();
                h.flag = *f.get(6)? == "1";
                Some(h)
            })
            .collect()
    };
    let real: Option<(String, Vec<HelperSite>)> = std::env::var("RAPIDR_BENCH_DIR").ok().zip(std::env::var("RAPIDR_BENCH_SITES").ok().filter(|s| !s.is_empty())).and_then(|(dir, out)| {
        let mut files: Vec<_> = std::fs::read_dir(&dir).ok()?.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "rr")).collect();
        files.sort();
        let (mut all, mut sites) = (String::new(), Vec::new());
        for f in &files {
            if !all.is_empty() {
                all.push('\n');
            }
            let name = f.file_name()?.to_string_lossy().into_owned();
            let rows = std::fs::read_to_string(std::path::Path::new(&out).join(format!("{name}.sites"))).unwrap_or_default();
            sites.extend(parse(&rows, all.len()));
            all.push_str(&std::fs::read_to_string(f).ok()?);
        }
        Some((all, sites))
    });
    let text = real.as_ref().map_or(text, |(t, _)| t.clone());
    let lines = text.lines().count();
    // (a helper or more on every line: a colour mark after the line's
    // first `(…)` group or its end, an argument name before its first number)
    let sites = |text: &str| -> Vec<HelperSite> {
        let mut out = Vec::new();
        let mut at = 0;
        for line in text.split_inclusive('\n') {
            let body = line.trim_end_matches('\n');
            if let Some(k) = body.find(|c: char| c.is_ascii_digit()) {
                let mut h = HelperSite::new(HelperKind::ArgName, at + k, at + k + 1);
                h.label = "x1".into();
                h.flag = true;
                out.push(h);
            }
            if !body.trim().is_empty() {
                let end = body.find(')').map_or(body.len(), |k| k + 1);
                let mut h = HelperSite::new(HelperKind::Color, at + end.saturating_sub(1).min(body.len()), at + end);
                h.value = "1344255".into();
                h.form = "rgb".into();
                out.push(h);
            }
            at += line.len();
        }
        out
    };
    let mut report = Vec::new();
    for level in ["off", "quiet", "helpful"] {
        let id = format!("bench{level}");
        let (mut s, mut f, mut ts) = form(&id, &text);
        f.focus_id(&s, &id);
        with_code_mut(&id, |c| {
            c.set("helpers", &v_str(level));
            if level != "off" {
                let all = real.as_ref().map_or_else(|| sites(&c.text()), |(_, s)| s.clone());
                c.helpers.set_sites(all, None);
            }
        });
        f.paint(&s, &mut ts, 2.0);
        s.call(&id, "gotoline", &[v_int((lines / 2) as i64)]);
        f.paint(&s, &mut ts, 2.0);
        key(&mut f, &s, &mut ts, 35, "", Mods::NONE);
        f.paint(&s, &mut ts, 2.0);
        let mut times = Vec::new();
        let (mut keys, mut paints) = (Vec::new(), Vec::new());
        for k in 0..300 {
            let t = std::time::Instant::now();
            if k % 2 == 0 {
                key(&mut f, &s, &mut ts, 65, "a", Mods::NONE);
            } else {
                key(&mut f, &s, &mut ts, 8, "", Mods::NONE);
            }
            let tk = t.elapsed().as_secs_f64() * 1000.0;
            f.paint(&s, &mut ts, 2.0);
            let all = t.elapsed().as_secs_f64() * 1000.0;
            times.push(all);
            keys.push(tk);
            paints.push(all - tk);
        }
        keys.sort_by(|a, b| a.partial_cmp(b).unwrap());
        paints.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!("[typing] helpers {level}: the key {:.3} ms, the frame {:.3} ms (medians)", keys[150], paints[150]);
        times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let (median, p95) = (times[times.len() / 2], times[times.len() * 95 / 100]);
        let n = with_code(&id, |c| c.helpers.sites.len()).unwrap();
        // (the sites mapped through one typed key, alone)
        let map_us = with_code_mut(&id, |c| {
            let len = c.doc.len_bytes();
            let at = c.doc.selections().primary().head;
            let t = std::time::Instant::now();
            for _ in 0..300 {
                let set = rapidr_editor::ChangeSet::new(vec![rapidr_editor::transaction::Change::insert(at, "a".to_string())], len).unwrap();
                c.helpers.map(&set);
            }
            t.elapsed().as_secs_f64() * 1e6 / 300.0
        })
        .unwrap();
        println!("[typing] helpers {level}: the sites through an edit {map_us:.1} µs");
        report.push((level, median, p95, n));
    }
    for (level, median, p95, n) in &report {
        println!("[typing] {lines} lines, helpers {level} ({n} sites): median {median:.3} ms, p95 {p95:.3} ms per key and frame");
    }
    let (off, on) = (report[0].1, report[2].1);
    println!("[typing] helpers on / off: {:.1}%", (on / off - 1.0) * 100.0);
}

/// The editor hangs from the value it edits, even when a key scrolled the
/// view since the last frame (it once hung from the editor's top).
#[test]
fn the_editor_hangs_from_its_value() {
    let mut code: String = (0..80).map(|i| format!("PRINT {i}\n")).collect();
    code.push_str("Form1.Color = RGB(1, 2, 3)\n");
    let (mut s, mut f, mut ts) = form("hl9", &code);
    f.focus_id(&s, "hl9");
    set_sites("hl9", vec![site(&code, "RGB(1, 2, 3)", HelperKind::Color, "197121", "rgb")]);
    f.paint(&s, &mut ts, 1.0);
    // (to the last line by the program, End, Ctrl+. — no frame between)
    s.call("hl9", "gotoline", &[v_int(80)]);
    key(&mut f, &s, &mut ts, 35, "", Mods::NONE);
    key(&mut f, &s, &mut ts, 190, ".", CMD);
    let anchor = crate::value_edit::with("hl9", |p| p.anchor).unwrap();
    f.paint(&s, &mut ts, 1.0);
    let at = code.find("RGB(1").unwrap();
    let (_, top) = caret_at(&f, "hl9", at);
    let node_top = f.nodes.iter().find(|n| n.id == "hl9").unwrap().abs.1;
    assert_eq!(anchor.1, node_top + top.round() as i64, "the value's row, not the editor's top");
}

/// The editor follows its value as the view scrolls or the editor
/// resizes; its value scrolled out of sight, it closes and nothing is
/// written (a resized window once left it hanging over other code).
#[test]
fn the_editor_follows_its_value_and_closes_out_of_sight() {
    let mut code: String = (0..80).map(|i| format!("PRINT {i}\n")).collect();
    code.push_str("Form1.Color = RGB(1, 2, 3)\n");
    code.push_str(&(0..80).map(|i| format!("PRINT {i}\n")).collect::<String>());
    let (mut s, mut f, mut ts) = form("hl10", &code);
    f.focus_id(&s, "hl10");
    set_sites("hl10", vec![site(&code, "RGB(1, 2, 3)", HelperKind::Color, "197121", "rgb")]);
    f.paint(&s, &mut ts, 1.0);
    s.call("hl10", "gotoline", &[v_int(80)]);
    key(&mut f, &s, &mut ts, 35, "", Mods::NONE);
    key(&mut f, &s, &mut ts, 190, ".", CMD);
    f.paint(&s, &mut ts, 1.0);
    let before = crate::value_edit::with("hl10", |p| p.anchor).unwrap();
    // (the editor moved down in its form: its anchor follows the row)
    s.set("hl10", "top", v_int(40));
    f.sync(&s);
    f.paint(&s, &mut ts, 1.0);
    let at = code.find("RGB(1").unwrap();
    let (_, top) = caret_at(&f, "hl10", at);
    let node_top = f.nodes.iter().find(|n| n.id == "hl10").unwrap().abs.1;
    let after = crate::value_edit::with("hl10", |p| p.anchor).expect("still open: its row in sight");
    assert_eq!(after.1, node_top + top.round() as i64, "follows its row ({before:?} then {after:?})");
    // (scrolled far away: it closes, the code as it was)
    s.call("hl10", "gotoline", &[v_int(0)]);
    f.paint(&s, &mut ts, 1.0);
    assert!(!crate::value_edit::is_open("hl10"), "closed with its row out of sight");
    assert!(with_code("hl10", |c| c.doc.text().contains("RGB(1, 2, 3)")).unwrap());
    assert_eq!(with_code("hl10", |c| c.helpers.editor_text.clone()).unwrap(), "");
}
