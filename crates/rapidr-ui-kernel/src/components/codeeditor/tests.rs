use rapidr_value::objects::{with_code, with_code_mut};
use rapidr_value::{v_int, v_str};

use crate::display::Item;
use crate::{FormUi, KernelEvent, MemClipboard, MemStore, Mods, TextSystem};

fn code_form(id: &str, text: &str) -> (MemStore, FormUi, TextSystem) {
    let mut s = MemStore::new();
    let form = format!("{id}f");
    s.add(&form, "RFORM", None).set(&form, "clientwidth", v_int(640)).set(&form, "clientheight", v_int(400));
    s.add(id, "RCODEEDITOR", Some(&form)).set(id, "left", v_int(8)).set(id, "top", v_int(8)).set(id, "width", v_int(600)).set(id, "height", v_int(240));
    s.set(id, "text", v_str(text));
    let f = FormUi::build(&s, &form, false);
    (s, f, TextSystem::new())
}

fn numbers(list: &crate::display::DisplayList) -> Vec<String> {
    list.items
        .iter()
        .filter_map(|i| match i {
            Item::Op { op: rapidr_value::objects::ops::Op::Text { text, place: rapidr_value::objects::ops::Place::TopRight, .. }, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn text(id: &str) -> String {
    with_code(id, |c| c.text()).unwrap()
}

fn key(f: &mut FormUi, s: &MemStore, ts: &mut TextSystem, vk: i64, text: &str, mods: Mods) {
    let mut clip = MemClipboard::default();
    f.key_down(s, ts, vk, text, mods, &mut clip);
}

fn typed(f: &mut FormUi, s: &MemStore, ts: &mut TextSystem, t: &str) {
    for c in t.chars() {
        let vk = match c {
            ' ' => 32,
            '.' => 190,
            c if c.is_ascii_alphanumeric() => c.to_ascii_uppercase() as i64,
            _ => 0,
        };
        key(f, s, ts, vk, &c.to_string(), Mods::NONE);
    }
}

#[test]
fn it_draws_only_the_rows_in_view_and_numbers_them() {
    let src = (1..=3000).map(|i| format!("PRINT {i} ' line")).collect::<Vec<_>>().join("\n");
    let (mut s, mut f, mut ts) = code_form("ce1", &src);
    let list = f.paint(&s, &mut ts, 1.0);
    let shown = numbers(&list);
    assert_eq!(shown.first().map(String::as_str), Some("1"));
    assert!(shown.len() > 5 && shown.len() < 20, "only the lines in view: {shown:?}");
    let rows = list.items.iter().filter(|i| matches!(i, Item::Text(t) if t.node == "ce1")).count();
    assert_eq!(rows, shown.len(), "one layout per row in view");
    // GotoLine (0-based) scrolls the caret's line into view
    s.call("ce1", "gotoline", &[v_int(2500)]);
    let list = f.paint(&s, &mut ts, 1.0);
    assert!(numbers(&list).iter().any(|n| n == "2501"), "{:?}", numbers(&list));
    // the layouts the host draws are there
    let t = list.items.iter().find_map(|i| match i {
        Item::Text(t) => Some(t.clone()),
        _ => None,
    });
    assert!(f.editor_layout_at("ce1", t.unwrap().para).is_some());
}

#[test]
fn typing_multi_carets_undo_and_events() {
    let (s, mut f, mut ts) = code_form("ce2", "a = 1\nb = 2\nc = 3\n");
    f.paint(&s, &mut ts, 1.0);
    f.focus_id(&s, "ce2");
    // a caret on each line's start (Ctrl+Alt+Down twice), then typing
    let cmd_alt = Mods { command: true, ctrl: true, alt: true, ..Mods::NONE };
    key(&mut f, &s, &mut ts, 40, "", cmd_alt);
    key(&mut f, &s, &mut ts, 40, "", cmd_alt);
    assert_eq!(with_code("ce2", |c| c.doc.selections().len()).unwrap(), 3);
    typed(&mut f, &s, &mut ts, "x");
    assert_eq!(text("ce2"), "xa = 1\nxb = 2\nxc = 3\n");
    let events = f.take_events();
    assert!(events.contains(&KernelEvent::Change("ce2".into())), "{events:?}");
    assert!(events.iter().any(|e| matches!(e, KernelEvent::Fire { event, .. } if event == "oncaretmove")), "{events:?}");
    // Escape back to one caret; Ctrl+Z undoes the typing
    key(&mut f, &s, &mut ts, 27, "", Mods::NONE);
    assert_eq!(with_code("ce2", |c| c.doc.selections().len()).unwrap(), 1);
    let cmd = Mods { command: true, ctrl: true, ..Mods::NONE };
    key(&mut f, &s, &mut ts, 90, "", cmd);
    assert_eq!(text("ce2"), "a = 1\nb = 2\nc = 3\n");
    // Tab types (the focus stays); Enter keeps the indentation
    key(&mut f, &s, &mut ts, 9, "\t", Mods::NONE);
    assert_eq!(f.focus.map(|i| f.nodes[i].id.clone()).as_deref(), Some("ce2"));
    key(&mut f, &s, &mut ts, 13, "\r", Mods::NONE);
    assert!(text("ce2").starts_with("    \n    a = 1"), "{:?}", text("ce2"));
}

/// A snippet's stops (docs/studio-wow.md ED-8): `sub` and Tab gives the
/// skeleton with its name selected; typed over, Tab goes to the
/// parameters, then into the body — the stops follow the typing (they
/// were once moved through their own insertion and lost).
#[test]
fn snippets_go_from_stop_to_stop() {
    let (mut s, mut f, mut ts) = code_form("ce9", "");
    s.set("ce9", "languageservice", v_int(0));
    with_code_mut("ce9", |c| c.opts.language_service = false);
    f.paint(&s, &mut ts, 1.0);
    f.focus_id(&s, "ce9");
    let _ = f.take_events();
    typed(&mut f, &s, &mut ts, "su");
    // (no service: the program's list — the language's snippets come with it)
    s.call("ce9", "showcompletion", &[v_str("SUB\tkeyword")]);
    with_code_mut("ce9", |c| {
        let mut sn = rapidr_editor::service::Completion::new("sub", rapidr_editor::service::CompletionKind::Snippet);
        sn.insert = Some("SUB ${1:Name}(${2})\n\t$0\nEND SUB".into());
        sn.snippet = true;
        c.completion.as_mut().unwrap().items.insert(0, sn);
    });
    key(&mut f, &s, &mut ts, 9, "", Mods::NONE);
    let sel = || with_code("ce9", |c| { let p = c.doc.selections().primary(); c.doc.slice(p.start()..p.end()).into_owned() }).unwrap();
    assert_eq!(sel(), "Name");
    typed(&mut f, &s, &mut ts, "Go");
    key(&mut f, &s, &mut ts, 9, "", Mods::NONE);
    typed(&mut f, &s, &mut ts, "n");
    assert!(text("ce9").starts_with("SUB Go(n)\n"), "{:?}", text("ce9"));
    key(&mut f, &s, &mut ts, 9, "", Mods::NONE);
    typed(&mut f, &s, &mut ts, "x");
    assert!(text("ce9").contains("\n    x\nEND SUB") || text("ce9").contains("\n\tx\nEND SUB"), "the body: {:?}", text("ce9"));
}

#[test]
fn folds_hide_lines_and_completion_from_the_program() {
    let (mut s, mut f, mut ts) = code_form("ce3", "SUB A\n  PRINT 1\n  PRINT 2\nEND SUB\nx = 1\n");
    f.paint(&s, &mut ts, 1.0);
    with_code_mut("ce3", |c| {
        c.fold(0);
    });
    let list = f.paint(&s, &mut ts, 1.0);
    assert_eq!(numbers(&list), ["1", "5", "6"]);
    // OnCompletionRequest when no service serves the language; the
    // program's ShowCompletion; Enter accepts
    s.set("ce3", "languageservice", v_int(0));
    with_code_mut("ce3", |c| c.opts.language_service = false);
    f.focus_id(&s, "ce3");
    s.call("ce3", "gotolinecolumn", &[v_int(5), v_int(6)]);
    f.paint(&s, &mut ts, 1.0);
    let _ = f.take_events();
    typed(&mut f, &s, &mut ts, " L");
    let events = f.take_events();
    assert!(events.iter().any(|e| matches!(e, KernelEvent::Fire { event, args, .. } if event == "oncompletionrequest" && args.len() == 3)), "{events:?}");
    s.call("ce3", "showcompletion", &[v_str("Left\tproperty\nLen\tbuiltin\nLocate\tkeyword")]);
    typed(&mut f, &s, &mut ts, "e");
    let n = with_code("ce3", |c| c.completion.as_ref().map(|l| l.items.len())).flatten();
    assert_eq!(n, Some(3));
    let list = f.paint(&s, &mut ts, 1.0);
    let has_popup = list.items.iter().any(|i| matches!(i, Item::Op { op: rapidr_value::objects::ops::Op::Text { text, .. }, .. } if text == "ft"));
    assert!(has_popup, "the list shows Left (its typed part apart)");
    key(&mut f, &s, &mut ts, 13, "\r", Mods::NONE);
    assert!(text("ce3").contains("x = 1 Left"), "{:?}", text("ce3"));
}

#[test]
fn screen_readers_read_the_window_as_text_runs() {
    use rapidr_editor::Buffer;
    use rapidr_value::objects::a11y::node_id;
    let long = "y".repeat(600);
    let mut src = (1..=300).map(|i| format!("PRINT {i}")).collect::<Vec<_>>();
    src[1] = "\tIF a$ = \"b\" THEN".into();
    src[2] = long.clone();
    let src = src.join("\n");
    let (s, mut f, mut ts) = code_form("ce4", &src);
    f.paint(&s, &mut ts, 2.0);
    f.focus_id(&s, "ce4");
    // "IF" on line 2 selected backwards: the caret after the tab
    with_code_mut("ce4", |c| {
        let a = c.doc.buffer().line_start(1) + 1;
        c.doc.set_selections(rapidr_editor::Selections::single(rapidr_editor::Selection::new(a + 2, a)));
    });
    f.paint(&s, &mut ts, 2.0);
    let tree = f.access_tree(&s, &mut ts);
    let n = tree.find(node_id("ce4")).unwrap();
    assert_eq!(n.description, "Line 2, column 2", "the caret's place (after the name rule)");
    let t = n.text.as_ref().expect("text runs");
    // the runs joined are the value (the window of lines): 101 lines, the
    // long one in 3 pieces
    let joined: String = t.runs.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(Some(&joined), n.value.as_ref());
    assert_eq!(t.runs.len(), 102 + 2);
    for r in &t.runs {
        let sum: usize = r.char_lengths.iter().map(|&l| l as usize).sum();
        assert_eq!(sum, r.text.len());
        assert!(r.char_lengths.len() <= 251);
    }
    assert_eq!((t.runs[0].text.as_str(), t.runs[0].char_lengths.len()), ("PRINT 1\n", 8), "the break one character");
    assert!(t.runs[2].continues && t.runs[3].continues && !t.runs[4].continues);
    assert_eq!(t.runs[2].text.len() + t.runs[3].text.len() + t.runs[4].text.len(), 600 + 1);
    // the lines in view are laid out: the tab as wide as 4 columns, the
    // characters left to right; the editor's place in the form
    let r1 = &t.runs[1];
    assert_eq!(r1.char_positions.len(), r1.char_lengths.len());
    let (w_tab, w_i) = (r1.char_widths[0], r1.char_widths[1]);
    assert!(w_tab > 3.0 * w_i && w_tab < 5.0 * w_i, "tab {w_tab}, I {w_i}");
    assert!(r1.char_positions.windows(2).all(|p| p[1] >= p[0]));
    assert!(r1.bounds.0 > 8 && r1.bounds.1 > t.runs[0].bounds.1, "{:?} {:?}", r1.bounds, t.runs[0].bounds);
    // words as Ctrl+Right stops: the tab, IF, a$ (BASIC's), =, ", b, ", THEN
    assert_eq!(r1.word_starts, [0, 1, 4, 7, 9, 10, 11, 13]);
    // off-screen lines: no positions
    assert!(t.runs.last().unwrap().char_positions.is_empty());
    // the selection: anchor after "IF", the caret after the tab
    let (anchor, focus) = t.selection.unwrap();
    assert_eq!((anchor.run, anchor.char_index), (1, 3));
    assert_eq!((focus.run, focus.char_index), (1, 1));
    // the caret at a line's end: on its break
    with_code_mut("ce4", |c| {
        let e = c.doc.buffer().line_end(0);
        c.doc.set_selections(rapidr_editor::Selections::single(rapidr_editor::Selection::new(e, e)));
    });
    let tree = f.access_tree(&s, &mut ts);
    let t = tree.find(node_id("ce4")).unwrap().text.clone().unwrap();
    assert_eq!(t.selection.map(|(_, f)| (f.run, f.char_index)), Some((0, "PRINT 1".len())));
}

use rapidr_editor::service as svc;

/// A language service of the test's own: completion of two names,
/// upper-case keywords, one problem, a hover.
struct Fake {
    text: String,
}

impl svc::LanguageService for Fake {
    fn serves(&self, language: &str) -> bool {
        language == "rapidq-basic"
    }
    fn update(&mut self, _file: &str, text: &str) {
        self.text = text.to_string();
    }
    fn close(&mut self, _file: &str) {}
    fn completions(&mut self, _file: &str, offset: usize) -> svc::Completions {
        let start = self.text[..offset].rfind(|c: char| !c.is_alphanumeric()).map_or(0, |i| i + 1);
        svc::Completions { items: vec![svc::Completion::new("Caption", svc::CompletionKind::Property), svc::Completion::new("ShowModal", svc::CompletionKind::Method)], start, end: offset }
    }
    fn hover(&mut self, _file: &str, offset: usize) -> Option<svc::Hover> {
        Some(svc::Hover { text: "```\nDIM x AS INTEGER\n```\nA variable.".into(), start: offset, end: offset + 1 })
    }
    fn signature(&mut self, _file: &str, _offset: usize) -> Option<svc::SignatureHelp> {
        None
    }
    fn diagnostics(&mut self, file: &str) -> Vec<svc::Diagnostic> {
        vec![svc::Diagnostic { file: file.into(), start: 0, end: 3, severity: svc::Severity::Error, message: "boom".into(), code: String::new() }]
    }
    fn definition(&mut self, _file: &str, _offset: usize) -> Vec<svc::Location> {
        Vec::new()
    }
    fn references(&mut self, _file: &str, _offset: usize) -> Vec<svc::Location> {
        Vec::new()
    }
    fn rename(&mut self, _file: &str, _offset: usize, _new_name: &str) -> Result<Vec<(String, Vec<svc::Edit>)>, String> {
        Err("no".into())
    }
    fn outline(&mut self, _file: &str) -> Vec<svc::OutlineItem> {
        Vec::new()
    }
    fn semantic_tokens(&mut self, _file: &str) -> Vec<svc::SemanticToken> {
        Vec::new()
    }
    fn format(&mut self, _file: &str, _indent: &str) -> Vec<svc::Edit> {
        Vec::new()
    }
    fn case_edits(&mut self, _file: &str, offset: usize, ch: char, case: &str) -> Vec<svc::Edit> {
        let end = offset - ch.len_utf8();
        let start = self.text[..end].rfind(|c: char| !c.is_alphanumeric()).map_or(0, |i| i + 1);
        let word = &self.text[start..end];
        if case == "upper" && word.eq_ignore_ascii_case("dim") && word != "DIM" {
            return vec![svc::Edit { start, end, text: "DIM".into() }];
        }
        Vec::new()
    }
    fn case_triggers(&self) -> &'static [char] {
        &[' ']
    }
    fn position(&mut self, _file: &str, _offset: usize) -> Option<(usize, usize)> {
        None
    }
}

#[test]
fn the_language_service_completes_cases_and_diagnoses() {
    svc::install(Box::new(Fake { text: String::new() }));
    let (s, mut f, mut ts) = code_form("ce4", "");
    f.paint(&s, &mut ts, 1.0);
    f.focus_id(&s, "ce4");
    // keyword case as the word ends
    typed(&mut f, &s, &mut ts, "dim x");
    assert_eq!(text("ce4"), "DIM x");
    // completion opens after a trigger, filters as the word grows, Tab
    // accepts
    typed(&mut f, &s, &mut ts, " = Form.Sh");
    assert!(with_code("ce4", |c| c.completion.is_some()).unwrap(), "completion after Form.");
    key(&mut f, &s, &mut ts, 9, "\t", Mods::NONE);
    assert_eq!(text("ce4"), "DIM x = Form.ShowModal");
    // the problems once typing stops
    let later = crate::tick::now() + std::time::Duration::from_secs(2);
    crate::tick::set_test_now(Some(later));
    f.tick(&s, &mut ts, later);
    crate::tick::set_test_now(None);
    assert_eq!(with_code("ce4", |c| c.diagnostics.len()).unwrap(), 1);
    let list = f.paint(&s, &mut ts, 1.0);
    let squiggle = list.items.iter().any(|i| matches!(i, Item::Op { op: rapidr_value::objects::ops::Op::Stroke { .. }, .. }));
    assert!(squiggle, "the problem is underlined");
}

/// The designer's changes through ApplyPatches (S-DESIGN-2's shared
/// undo): each a step of the editor's own history, Ctrl+Z typed in the
/// editor takes them back one at a time to the text as loaded — and then
/// there is nothing left to undo.
#[test]
fn designer_patches_are_the_editors_undo_steps() {
    let first = "CREATE Form AS QFORM\n    Width = 340\nEND CREATE\n";
    let (mut s, mut f, mut ts) = code_form("ce9", first);
    f.paint(&s, &mut ts, 1.0);
    f.focus_id(&s, "ce9");
    s.call("ce9", "applypatches", &[v_str("1\t12\t1\t15\t360"), v_int(0)]);
    s.call("ce9", "applypatches", &[v_str("1\t15\t1\t15\t\\n    Left = 8\n2\t11\t2\t12\t9"), v_int(0)]);
    s.call("ce9", "applypatches", &[v_str("1\t12\t1\t15\t400"), v_int(0)]);
    assert_eq!(text("ce9"), "CREATE Form AS QFORM\n    Width = 400\n    Left = 9\nEND CREATE\n");
    f.paint(&s, &mut ts, 1.0);
    let cmd = Mods { command: true, ctrl: true, ..Mods::NONE };
    for _ in 0..3 {
        key(&mut f, &s, &mut ts, 90, "", cmd);
        f.paint(&s, &mut ts, 1.0);
    }
    assert_eq!(text("ce9"), first);
    assert!(!with_code("ce9", |c| c.doc.can_undo()).unwrap(), "nothing left to undo");
}
