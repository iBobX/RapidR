//! RDESIGNSURFACE's program source (docs/ide-plan.md I4): the surface's
//! `Source` read by `rapidr-designer`'s [`Document`] — every top-level
//! CREATE block into the designer model, each designer change written back
//! as the smallest text edits, one undo history of exact bytes. The
//! surface itself is `rapidr_value::objects::design` (shared by every
//! runtime); it reaches the parser through the [`SourceDoc`] this installs
//! ([`install`]: the desktop's and the web's Studio runtimes, when they make
//! a design surface).
//!
//! Positions go to the program as OnSourceEdit(StartLine, StartCol,
//! EndLine, EndCol, Text): lines and columns from 0, columns in characters.

use std::path::Path;

use rapidr_designer::{Designer, Document, TextPatch};
use rapidr_preprocessor::PreprocessOptions;
use rapidr_value::objects::design::{set_source_reader, SourceDoc, SourceEdit};

/// A file's text in the designer.
struct Doc {
    doc: Document,
}

/// Installs the reader (idempotent).
pub fn install() {
    set_source_reader(read);
}

fn read(text: &str, path: &str) -> Box<dyn SourceDoc> {
    let path = (!path.is_empty()).then(|| Path::new(path));
    Box::new(Doc { doc: Document::open(text, path, PreprocessOptions::default()) })
}

/// The edits `patches` made, each as positions in the text as the ones
/// before left it (`text`: the text before the first).
fn edits_of(mut text: String, patches: &[TextPatch]) -> Vec<SourceEdit> {
    let mut out = Vec::new();
    for p in patches {
        if p.start > p.end || p.end > text.len() || !text.is_char_boundary(p.start) || !text.is_char_boundary(p.end) {
            continue;
        }
        out.push(SourceEdit::of(&text, p.start, p.end, &p.insert));
        text.replace_range(p.start..p.end, &p.insert);
    }
    out
}

/// The one edit turning `before` into `after`: what they don't share at
/// either end (an undo's or a redo's transaction, as the editor applies it).
fn diff_edit(before: &str, after: &str) -> Vec<SourceEdit> {
    if before == after {
        return Vec::new();
    }
    let mut start = before.bytes().zip(after.bytes()).take_while(|(a, b)| a == b).count();
    while !before.is_char_boundary(start) || !after.is_char_boundary(start) {
        start -= 1;
    }
    let max_tail = before.len().min(after.len()) - start;
    let mut tail = before.bytes().rev().zip(after.bytes().rev()).take(max_tail).take_while(|(a, b)| a == b).count();
    while !before.is_char_boundary(before.len() - tail) || !after.is_char_boundary(after.len() - tail) {
        tail -= 1;
    }
    vec![SourceEdit::of(before, start, before.len() - tail, &after[start..after.len() - tail])]
}

/// Whether `name` is a word of `text` (any case): a name a new component
/// mustn't take.
fn has_word(text: &str, name: &str) -> bool {
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let lower = text.to_ascii_lowercase();
    let want = name.to_ascii_lowercase();
    let mut from = 0;
    while let Some(i) = lower[from..].find(&want) {
        let at = from + i;
        let before = lower[..at].chars().next_back();
        let after = lower[at + want.len()..].chars().next();
        if !before.is_some_and(is_ident) && !after.is_some_and(is_ident) {
            return true;
        }
        from = at + want.len();
    }
    false
}

impl SourceDoc for Doc {
    fn text(&self) -> String {
        self.doc.text().to_string()
    }

    fn set_text(&mut self, text: &str) -> bool {
        if self.doc.text() == text {
            return false;
        }
        self.doc.set_text(text);
        self.doc.clear_history();
        true
    }

    fn forms(&self) -> Vec<(String, String)> {
        self.doc.forms().iter().map(|f| (f.name().to_string(), f.designer.design.node(f.designer.design.root()).map(|n| n.type_written.clone()).unwrap_or_default())).collect()
    }

    fn designer(&self, form: usize) -> Option<Designer> {
        self.doc.forms().get(form).map(|f| f.designer.clone())
    }

    fn commit(&mut self, form: usize, designer: Designer) -> (Vec<SourceEdit>, Option<Designer>) {
        let before = self.doc.text().to_string();
        let Some(d) = self.doc.designer(form) else { return (Vec::new(), None) };
        *d = designer;
        let patches = self.doc.sync();
        let after = self.doc.forms().get(form).map(|f| f.designer.clone());
        (edits_of(before, &patches), after)
    }

    fn undo(&mut self) -> Option<Vec<SourceEdit>> {
        let before = self.doc.text().to_string();
        self.doc.undo().then(|| diff_edit(&before, self.doc.text()))
    }

    fn redo(&mut self) -> Option<Vec<SourceEdit>> {
        let before = self.doc.text().to_string();
        self.doc.redo().then(|| diff_edit(&before, self.doc.text()))
    }

    fn can_undo(&self) -> bool {
        self.doc.can_undo()
    }

    fn can_redo(&self) -> bool {
        self.doc.can_redo()
    }

    fn name_taken(&self, name: &str) -> bool {
        has_word(self.doc.text(), name)
    }

    fn diagnostics(&self) -> Vec<String> {
        self.doc.diagnostics().to_vec()
    }

    fn error_line(&self) -> Option<usize> {
        self.doc.error_line()
    }

    fn add_form(&mut self, name: &str) -> Vec<SourceEdit> {
        let before = self.doc.text().to_string();
        let patches = self.doc.add_form(name);
        edits_of(before, &patches)
    }

    fn create_handler(&mut self, form: usize, component: &str, event: &str) -> Result<(String, Option<usize>, Vec<SourceEdit>), String> {
        let before = self.doc.text().to_string();
        let h = self.doc.create_handler(form, component, event)?;
        Ok((h.sub, h.line, edits_of(before, &h.patches)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rapidr_value::objects::design::DesignSurface;

    const NOTEPAD: &str = "DIM FileName AS STRING\n\nCREATE OpenDialog AS QOPENDIALOG\nEND CREATE\n\nCREATE Form AS QFORM\n    Caption = \"Notepad\"\n    Width = 480\n    Height = 340\n    CREATE Ok AS QBUTTON\n        Left = 380: Top = 270\n        Anchors = akRight + akBottom\n    END CREATE\nEND CREATE\n\nForm.ShowModal\n";

    /// Applies OnSourceEdits as an editor would (line / column positions).
    fn apply(text: &mut String, edits: &[SourceEdit]) {
        for e in edits {
            let at = |(line, col): (usize, usize)| {
                let start: usize = text.split('\n').take(line).map(|l| l.len() + 1).sum();
                let rest = &text[start..];
                start + rest.char_indices().nth(col).map_or(rest.len(), |(i, _)| i)
            };
            let (a, b) = (at(e.start), at(e.end));
            text.replace_range(a..b, &e.text);
        }
    }

    fn surface() -> DesignSurface {
        install();
        let mut s = DesignSurface::default();
        assert!(s.open_source(NOTEPAD));
        s
    }

    #[test]
    fn the_first_form_is_designed_at_its_own_size() {
        let s = surface();
        assert_eq!(s.root_name(), "Form", "not the dialog before it");
        let (_, _, w, h) = s.form_rect();
        assert_eq!((w, h), (480, 340));
        assert_eq!(s.title(), "Notepad");
        assert!(!s.fit);
    }

    #[test]
    fn a_change_is_written_as_edits_and_undone_to_the_exact_text() {
        let mut s = surface();
        let mut editor = NOTEPAD.to_string();
        // the form's right edge dragged 40 further: Width and the anchored
        // button's Left written
        assert!(s.resize_form(520, 340));
        let events = s.take_events();
        let edits: Vec<SourceEdit> = events.iter().filter_map(|e| match e {
            rapidr_value::objects::design::DesignEvent::SourceEdit(e) => Some(e.clone()),
            _ => None,
        }).collect();
        assert!(!edits.is_empty());
        apply(&mut editor, &edits);
        assert!(editor.contains("Width = 520") && editor.contains("Left = 420: Top = 270"), "{editor}");
        assert_eq!(s.get("source").unwrap().to_string_val(), editor, "the editor and the document agree");
        // a new button from the toolbox, at (16, 16)
        let i = s.add_at("QBUTTON", (16, 16), None).expect("added");
        assert_eq!(s.call("getname", &[rapidr_value::v_int(i as i64)]).unwrap().to_string_val(), "Button1");
        let edits: Vec<SourceEdit> = s.take_events().into_iter().filter_map(|e| match e {
            rapidr_value::objects::design::DesignEvent::SourceEdit(e) => Some(e),
            _ => None,
        }).collect();
        apply(&mut editor, &edits);
        assert!(editor.contains("CREATE Button1 AS QBUTTON"), "{editor}");
        assert_eq!(s.get("source").unwrap().to_string_val(), editor);
        // undo twice: the exact bytes
        for _ in 0..2 {
            assert!(s.undo());
            let edits: Vec<SourceEdit> = s.take_events().into_iter().filter_map(|e| match e {
                rapidr_value::objects::design::DesignEvent::SourceEdit(e) => Some(e),
                _ => None,
            }).collect();
            apply(&mut editor, &edits);
        }
        assert_eq!(editor, NOTEPAD);
        assert_eq!(s.get("source").unwrap().to_string_val(), NOTEPAD);
        // the code edited elsewhere: read again, the history starts over
        let typed = NOTEPAD.replace("Width = 480", "Width = 500");
        assert!(s.open_source(&typed));
        assert_eq!(s.form_rect().2, 500);
        assert!(!s.can_undo());
    }

    #[test]
    fn names_already_in_the_file_are_not_taken() {
        install();
        let mut s = DesignSurface::default();
        s.open_source("DIM Button1 AS INTEGER\nCREATE F AS QFORM\nEND CREATE\n");
        let i = s.add_at("QBUTTON", (8, 8), None).unwrap();
        assert_eq!(s.call("getname", &[rapidr_value::v_int(i as i64)]).unwrap().to_string_val(), "Button2");
        assert!(has_word("a Button1 b", "button1") && !has_word("Button10", "Button1"));
    }

    #[test]
    fn a_file_without_a_form() {
        install();
        let mut s = DesignSurface::default();
        s.open_source("PRINT 1\n");
        assert!(s.no_form() && s.empty_text().is_some());
        assert_eq!(s.add_at("QBUTTON", (8, 8), None), None);
    }

    #[test]
    fn a_double_click_makes_the_handler() {
        let mut s = surface();
        let mut editor = NOTEPAD.to_string();
        let sub = s.create_handler("Ok", "").expect("a handler");
        assert_eq!(sub, "OkClick");
        let edits: Vec<SourceEdit> = s.take_events().into_iter().filter_map(|e| match e {
            rapidr_value::objects::design::DesignEvent::SourceEdit(e) => Some(e),
            _ => None,
        }).collect();
        apply(&mut editor, &edits);
        assert!(editor.contains("OnClick = OkClick") && editor.contains("SUB OkClick"), "{editor}");
        assert_eq!(s.get("source").unwrap().to_string_val(), editor);
        let line = s.handler_line as usize;
        assert!(editor.lines().nth(line).is_some(), "the caret's line is in the text");
        assert!(s.undo());
        assert_eq!(s.get("source").unwrap().to_string_val(), NOTEPAD);
    }

    const PANEL: &str = "CREATE F AS QFORM\n  Width = 400: Height = 300\n  CREATE Panel1 AS QPANEL\n    Left = 200: Top = 10: Width = 150: Height = 150\n  END CREATE\n  CREATE B AS QBUTTON\n    Left = 10: Top = 10\n  END CREATE\nEND CREATE\n";

    fn edits(s: &mut DesignSurface) -> Vec<SourceEdit> {
        s.take_events()
            .into_iter()
            .filter_map(|e| match e {
                rapidr_value::objects::design::DesignEvent::SourceEdit(e) => Some(e),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn dropped_on_a_panel_it_goes_into_it() {
        install();
        let mut s = DesignSurface::default();
        s.open_source(PANEL);
        let mut editor = PANEL.to_string();
        s.mouse_down(15, 15, false);
        s.mouse_drag(225, 45);
        s.mouse_drag(230, 50);
        s.mouse_up();
        apply(&mut editor, &edits(&mut s));
        let panel_end = editor.find("  END CREATE\n  CREATE B").or_else(|| editor.find("    CREATE B"));
        assert!(editor.contains("    CREATE B AS QBUTTON"), "nested in the panel: {editor}");
        assert!(panel_end.is_some());
        assert_eq!(s.get("source").unwrap().to_string_val(), editor);
        assert!(s.undo());
        assert_eq!(s.get("source").unwrap().to_string_val(), PANEL);
    }

    #[test]
    fn the_keyboard_designs() {
        install();
        let mut s = DesignSurface::default();
        s.open_source(PANEL);
        // Tab to the panel, Shift+arrow by the grid, Ctrl+arrow resizes
        assert!(s.key(9, "", false, false));
        assert_eq!(s.root_name(), "F");
        assert!(s.announcement.starts_with("Panel1 (QPANEL), 200, 10"), "{}", s.announcement);
        assert!(s.key(39, "", true, false));
        assert!(s.key(40, "", false, true));
        let text = s.get("source").unwrap().to_string_val();
        assert!(text.contains("Left = 208") && text.contains("Height = 151"), "{text}");
        // Escape: the form; Delete with nothing selected does nothing
        assert!(s.key(27, "", false, false));
        assert_eq!(s.selection(), None);
        // a new label: typing writes its Caption
        let i = s.add_at("QLABEL", (16, 200), None).unwrap();
        for c in ["O", "K"] {
            assert!(s.key(0, c, false, false));
        }
        let text = s.get("source").unwrap().to_string_val();
        assert!(text.contains("Caption = \"OK\""), "{text}");
        assert_eq!(s.call("getname", &[rapidr_value::v_int(i as i64)]).unwrap().to_string_val(), "Label1");
        // Delete removes it; Ctrl+Z brings it back
        assert!(s.key(13, "", false, false));
        assert!(s.key(46, "", false, false));
        assert!(!s.get("source").unwrap().to_string_val().contains("Label1"));
        assert!(s.key(90, "", false, true));
        assert!(s.get("source").unwrap().to_string_val().contains("Label1"));
    }

    #[test]
    fn code_with_errors_keeps_the_last_good_form_read_only() {
        let mut s = surface();
        let broken = NOTEPAD.replace("Form.ShowModal", "IF x THEN\nPRINT (");
        s.open_source(&broken);
        assert_eq!(s.code_error, Some(17));
        assert!(s.banner().unwrap().contains("line 17"));
        assert_eq!(s.form_rect().2, 480, "the last good form");
        assert_eq!(s.add_at("QBUTTON", (8, 8), None), None, "read-only");
        assert!(!s.key(46, "", false, false));
        s.open_source(NOTEPAD);
        assert_eq!(s.code_error, None);
        assert!(s.add_at("QBUTTON", (8, 8), None).is_some());
    }

    // ---- S-DESIGN-2: the tray's outside components, the menu editor, Tab
    // order, captions in place, a new form, zoom, the shared history ----

    use rapidr_value::objects::design::DesignEvent;

    /// The events' edits applied to `editor`; the other events.
    fn heard(s: &mut DesignSurface, editor: &mut String) -> Vec<DesignEvent> {
        let events = s.take_events();
        let edits: Vec<SourceEdit> = events.iter().filter_map(|e| if let DesignEvent::SourceEdit(e) = e { Some(e.clone()) } else { None }).collect();
        apply(editor, &edits);
        events.into_iter().filter(|e| !matches!(e, DesignEvent::SourceEdit(_))).collect()
    }

    fn type_keys(s: &mut DesignSurface, text: &str) {
        for c in text.chars() {
            let vk = rapidr_value::input::vk_of_char(c.to_ascii_uppercase()).unwrap_or(0);
            assert!(s.key(vk, &c.to_string(), c.is_ascii_uppercase(), false), "typed {c}");
        }
    }

    const NOTEPAD_BAS: &str = include_str!("../../../examples/rapidq/notepad.bas");

    #[test]
    fn dialogs_made_outside_the_form_are_in_its_tray() {
        install();
        let mut s = DesignSurface::default();
        assert!(s.open_source(NOTEPAD_BAS));
        let mut editor = NOTEPAD_BAS.to_string();
        let tray = s.tray();
        let names: Vec<&str> = tray.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["OpenDialog", "SaveDialog"], "the program's dialogs, beside the form");
        let n = s.ids().len();
        assert_eq!((tray[0].index, tray[1].index), (n, n + 1), "their indexes after the form's components");
        assert_eq!(s.call("gettype", &[rapidr_value::v_int(n as i64 + 1)]).unwrap().to_string_val(), "QSAVEDIALOG");
        // a click on one selects it: the inspector shows it, its change goes into its own block
        let (x, y, _, _) = tray[0].rect;
        assert_eq!(s.mouse_down(x + 3, y + 3, false), Some(DesignEvent::Select(n as i64)));
        s.mouse_up();
        assert_eq!(s.selection(), Some(n));
        assert_eq!(s.inspected_objects(), [("OpenDialog".to_string(), "QOPENDIALOG".to_string())]);
        let rows = s.with_inspected(|d| d.inspect().properties.iter().map(|r| r.name.to_string()).collect::<Vec<_>>());
        assert!(rows.iter().any(|r| r == "Filter"), "{rows:?}");
        s.with_inspected(|d| d.set_property("Filter", Some("\"BASIC (*.bas)|*.bas\"")).unwrap());
        heard(&mut s, &mut editor);
        assert!(editor.contains("CREATE OpenDialog AS QOPENDIALOG\n    Filter = \"BASIC (*.bas)|*.bas\"\nEND CREATE"), "{editor}");
        assert_eq!(s.get("source").unwrap().to_string_val(), editor);
        assert_eq!(s.selection(), Some(n), "still selected");
        assert!(s.select_name("SaveDialog") && s.selection() == Some(n + 1));
        assert!(s.undo());
        heard(&mut s, &mut editor);
        assert_eq!(editor, NOTEPAD_BAS);
    }

    const MENU_FORM: &str = "CREATE Form1 AS QFORM\n    Caption = \"Menus\"\n    Width = 320\n    Height = 240\n    CREATE MainMenu1 AS QMAINMENU\n    END CREATE\nEND CREATE\n\nForm1.ShowModal\n";

    #[test]
    fn a_menu_is_built_in_place() {
        install();
        let mut s = DesignSurface::default();
        s.open_source(MENU_FORM);
        let mut editor = MENU_FORM.to_string();
        // the empty bar offers Type Here
        assert!(s.menu_active());
        let v = s.menu_view().unwrap();
        assert_eq!(v.bar.len(), 1);
        let (x, y, _, _) = v.bar[0].rect;
        assert!(y < 0, "on the bar, above the client area");
        s.mouse_down(x + 4, y + 4, false);
        s.mouse_up();
        assert!(s.editing.is_some());
        type_keys(&mut s, "&File");
        assert!(s.key(13, "", false, false));
        // the File item is there, its menu open, its Type Here being typed
        let file = s.designer.design.find("File1").expect("named as Delphi names it");
        assert_eq!(s.menu_open, vec![file]);
        assert!(s.editing.is_some());
        type_keys(&mut s, "&Open...");
        // Tab: the ShortCut field takes Ctrl+O
        assert!(s.key(9, "", false, false));
        assert!(s.key(79, "", false, true));
        assert!(s.key(13, "", false, false));
        assert!(s.key(27, "", false, false), "Escape leaves the next Type Here");
        heard(&mut s, &mut editor);
        let want = "    CREATE MainMenu1 AS QMAINMENU\n        CREATE File1 AS QMENUITEM\n            Caption = \"&File\"\n            CREATE Open1 AS QMENUITEM\n                Caption = \"&Open...\"\n                ShortCut = \"Ctrl+O\"\n            END CREATE\n        END CREATE\n    END CREATE\n";
        assert!(editor.contains(want), "{editor}");
        assert_eq!(s.get("source").unwrap().to_string_val(), editor);
        // a separator and Exit; then Exit dragged above the separator
        let open = s.designer.design.find("Open1").unwrap();
        s.designer.selection.set(open);
        let r = s.menu_slot_rect(rapidr_value::objects::design::menus::Slot::New { parent: file, before: None }).unwrap();
        s.mouse_down(r.0 + 30, r.1 + 5, false);
        s.mouse_up();
        type_keys(&mut s, "-");
        assert!(s.key(13, "", false, false));
        type_keys(&mut s, "E&xit");
        assert!(s.key(13, "", false, false));
        assert!(s.key(27, "", false, false));
        heard(&mut s, &mut editor);
        assert!(editor.contains("CREATE N1 AS QMENUITEM\n                Caption = \"-\"") && editor.contains("CREATE Exit1 AS QMENUITEM"), "{editor}");
        let exit = s.designer.design.find("Exit1").unwrap();
        let n1 = s.designer.design.find("N1").unwrap();
        let er = s.menu_slot_rect(rapidr_value::objects::design::menus::Slot::Item(exit)).unwrap();
        let nr = s.menu_slot_rect(rapidr_value::objects::design::menus::Slot::Item(n1)).unwrap();
        s.mouse_down(er.0 + 30, er.1 + 5, false);
        s.mouse_drag(er.0 + 30, er.1 - 4);
        s.mouse_drag(nr.0 + 30, nr.1 + 2);
        s.mouse_up();
        heard(&mut s, &mut editor);
        let (e, n) = (editor.find("CREATE Exit1").unwrap(), editor.find("CREATE N1").unwrap());
        assert!(e < n, "Exit moved above the separator: {editor}");
        // the gutter of a selected item toggles Checked; F2 edits it
        s.designer.selection.set(open);
        let or = s.menu_slot_rect(rapidr_value::objects::design::menus::Slot::Item(open)).unwrap();
        s.mouse_down(or.0 + 4, or.1 + 5, false);
        s.mouse_up();
        heard(&mut s, &mut editor);
        assert!(editor.contains("ShortCut = \"Ctrl+O\"\n                Checked = 1"), "{editor}");
        assert!(s.key(113, "", false, false));
        assert!(s.key(36, "", false, false));
        assert!(s.key(46, "", false, false));
        assert!(s.key(13, "", false, false));
        heard(&mut s, &mut editor);
        assert!(editor.contains("Caption = \"Open...\""), "{editor}");
        // every step undone: the exact text
        while s.undo() {
            heard(&mut s, &mut editor);
        }
        assert_eq!(editor, MENU_FORM);
    }

    const BUTTONS: &str = "CREATE F AS QFORM\n  Width = 400: Height = 300\n  CREATE A AS QBUTTON\n    Left = 10: Top = 10\n  END CREATE\n  CREATE B AS QBUTTON\n    Left = 10: Top = 50\n  END CREATE\n  CREATE C AS QBUTTON\n    Left = 10: Top = 90\n  END CREATE\nEND CREATE\n";

    #[test]
    fn tab_order_by_clicks() {
        install();
        let mut s = DesignSurface::default();
        s.open_source(BUTTONS);
        let mut editor = BUTTONS.to_string();
        assert!(s.set("tabordermode", &rapidr_value::Value::Boolean(true)));
        let nums: Vec<String> = s.tab_numbers().into_iter().map(|(_, n)| n).collect();
        assert_eq!(nums, ["0", "1", "2"]);
        s.mouse_down(20, 100, false);
        s.mouse_up();
        s.mouse_down(20, 20, false);
        s.mouse_up();
        heard(&mut s, &mut editor);
        let c = s.designer.design.find("C").unwrap();
        assert_eq!(s.tab_numbers()[0], (c, "0".to_string()), "C first");
        assert!(editor.contains("TabOrder = 0") && editor.contains("TabOrder = 1"), "{editor}");
        assert!(s.chrome_ops().iter().any(|o| matches!(o, rapidr_value::objects::ops::Op::Text { text, .. } if text == "2")), "the badges drawn");
        assert!(s.key(27, "", false, false));
        assert!(s.tab_order.is_none());
        while s.undo() {
            heard(&mut s, &mut editor);
        }
        assert_eq!(editor, BUTTONS);
    }

    #[test]
    fn captions_edited_in_place() {
        install();
        let mut s = DesignSurface::default();
        s.open_source(BUTTONS);
        let mut editor = BUTTONS.to_string();
        // a click selects; a second (slow) click on it edits; a double click doesn't
        s.mouse_down(20, 20, false);
        s.mouse_up();
        assert!(s.editing.is_none());
        s.mouse_down(20, 20, false);
        s.mouse_up();
        assert!(s.editing.is_some(), "a slow click edits");
        assert_eq!(s.mouse_down(20, 20, true), Some(DesignEvent::DblClick(0)), "the double click is the handler's");
        assert!(s.editing.is_none());
        s.mouse_up();
        // F2: the caption selected, typing replaces it, Enter writes it once
        assert!(s.key(113, "", false, false));
        type_keys(&mut s, "Go");
        assert!(s.key(13, "", false, false));
        let events = heard(&mut s, &mut editor);
        assert!(editor.contains("    Left = 10: Top = 10\n    Caption = \"Go\"\n"), "{editor}");
        assert_eq!(events.iter().filter(|e| matches!(e, DesignEvent::Step(_))).count(), 1, "one step");
        // Escape leaves it as it was
        assert!(s.key(113, "", false, false));
        type_keys(&mut s, "No");
        assert!(s.key(27, "", false, false));
        assert!(heard(&mut s, &mut editor).is_empty());
        assert!(s.undo());
        heard(&mut s, &mut editor);
        assert_eq!(editor, BUTTONS);
    }

    #[test]
    fn a_form_for_a_file_without_one() {
        install();
        let mut s = DesignSurface::default();
        let text = "' a console program\nPRINT \"Hello\"\n";
        s.open_source(text);
        assert!(s.no_form() && s.add_form_button().is_some());
        let mut editor = text.to_string();
        assert!(s.key(13, "", false, false), "Enter adds one");
        heard(&mut s, &mut editor);
        assert_eq!(editor, "' a console program\nPRINT \"Hello\"\n\nCREATE Form1 AS QFORM\n    Caption = \"Form1\"\n    Width = 320\n    Height = 240\nEND CREATE\n\nForm1.ShowModal\n");
        assert!(!s.no_form() && s.root_name() == "Form1");
        assert_eq!(s.form_rect().2, 320);
        assert!(s.add_at("QBUTTON", (8, 8), None).is_some(), "designed at once");
        // a name taken goes on to the next
        let mut t = DesignSurface::default();
        t.open_source("DIM Form1 AS INTEGER\n");
        assert_eq!(t.add_form("").as_deref(), Some("Form2"));
    }

    #[test]
    fn zoom_keeps_rapidq_pixels() {
        install();
        let mut s = DesignSurface::default();
        s.open_source(BUTTONS);
        s.set_size(1200, 900);
        let (ox, oy) = s.client_origin();
        assert!(s.set("zoom", &rapidr_value::v_int(200)));
        assert_eq!(s.get("zoom"), Some(rapidr_value::v_int(200)));
        let (zx, zy) = s.client_origin();
        assert!(zx > ox && zy > oy, "the frame zoomed too");
        // the mouse on the surface lands on RapidQ's pixels
        assert_eq!(s.client_point(zx as f64 + 40.0, zy as f64 + 40.0), (20, 20));
        assert_eq!(s.mouse_down(20, 20, false), Some(DesignEvent::Select(0)));
        s.mouse_drag(28, 28);
        s.mouse_up();
        let text = s.get("source").unwrap().to_string_val();
        assert!(text.contains("Left = 16: Top = 1"), "moved 8 of the program's pixels (16 on the surface), snapped: {text}");
        // the handles keep their size; Ctrl + − / 0 step
        assert!(s.key(189, "", false, true));
        assert_eq!(s.get("zoom"), Some(rapidr_value::v_int(175)));
        assert!(s.key(48, "", false, true));
        assert_eq!(s.get("zoom"), Some(rapidr_value::v_int(100)));
        s.set("zoom", &rapidr_value::v_int(1000));
        assert_eq!(s.get("zoom"), Some(rapidr_value::v_int(400)));
    }

    #[test]
    fn one_history_with_the_code_editor() {
        install();
        let mut s = DesignSurface::default();
        s.open_source(BUTTONS);
        assert!(s.set("sharedundo", &rapidr_value::Value::Boolean(true)));
        let mut editor = BUTTONS.to_string();
        // each change: OnSourceStep, its edits, OnChange
        assert!(!s.key(39, "", false, false), "nothing selected yet");
        s.select_name("A");
        assert_eq!(s.take_events(), [DesignEvent::Select(0)], "OnSelect: the inspector follows");
        assert!(s.key(39, "", false, false));
        let events = heard(&mut s, &mut editor);
        assert_eq!(events, [DesignEvent::Step(false), DesignEvent::Change]);
        // a component added, then its caption typed: one step with the add
        s.add_at("QLABEL", (100, 100), None).unwrap();
        let events = heard(&mut s, &mut editor);
        assert!(events.contains(&DesignEvent::Step(false)));
        type_keys(&mut s, "Hi");
        let events = heard(&mut s, &mut editor);
        assert_eq!(events.iter().filter(|e| matches!(e, DesignEvent::Step(true))).count(), 2, "{events:?}");
        // Ctrl+Z asks the program: the code editor holds the history
        assert!(s.key(90, "", false, true));
        assert_eq!(s.take_events(), [DesignEvent::Undo(false)]);
        assert!(s.key(89, "", false, true));
        assert_eq!(s.take_events(), [DesignEvent::Undo(true)]);
        // the editor undid: the text given back, the designer follows, its selection kept
        let undone = editor.replace("    Caption = \"Hi\"\n", "");
        assert!(s.open_source(&undone));
        assert_eq!(s.root_name(), "F");
        // typing in the code doesn't stop the designer from writing
        let typed = undone.replace("Top = 50", "Top = 60");
        s.open_source(&typed);
        s.select_name("B");
        assert!(s.key(40, "", false, false));
        let text = s.get("source").unwrap().to_string_val();
        assert!(text.contains("Top = 61"), "{text}");
    }

    #[test]
    fn diff_edits() {
        assert_eq!(diff_edit("abcXdef", "abcYYdef"), vec![SourceEdit { start: (0, 3), end: (0, 4), text: "YY".into() }]);
        assert_eq!(diff_edit("a\nb", "a\nb\nc"), vec![SourceEdit { start: (1, 1), end: (1, 1), text: "\nc".into() }]);
        assert!(diff_edit("x", "x").is_empty());
    }
}
