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
    fn diff_edits() {
        assert_eq!(diff_edit("abcXdef", "abcYYdef"), vec![SourceEdit { start: (0, 3), end: (0, 4), text: "YY".into() }]);
        assert_eq!(diff_edit("a\nb", "a\nb\nc"), vec![SourceEdit { start: (1, 1), end: (1, 1), text: "\nc".into() }]);
        assert!(diff_edit("x", "x").is_empty());
    }
}
