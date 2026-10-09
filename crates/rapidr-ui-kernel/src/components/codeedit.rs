//! RCODEEDITOR (Stage 10): RapidR's code editor (the IDE's) — the memo
//! (`memo.rs`) of its code [`Flavor`](super::memo): Courier New at 13
//! pixels, black on white, no word wrap, both scroll bars as needed, Tab
//! typing a tab, a 40-pixel line-number gutter, and BASIC's
//! colours as the editor's styled runs (keywords dark blue and bold,
//! strings red, comments green and italic, numbers maroon:
//! `rapidr_value::objects::code`), coloured again only for the paragraphs
//! an edit touched.
//!
//! Its text is the shared model's (a `TextEdit` in code mode: Text with
//! '\n' line breaks, Lines, LineCount, SelStart, WhereX / WhereY …), so
//! GetSubList, GotoSub and GotoLine are the model's too; GotoLine and
//! GotoSub ask the editor to scroll the caret into view. The user's
//! typing reaches the model at once, then OnChange.

use rapidr_value::objects::a11y::{AccessNode, Action};
use rapidr_value::objects::ops::Rect;

use super::edit::MenuState;
use super::memo::Memo;
use rapidr_value::input::Cursor;
use super::{ComponentKind, Cx, Ime, KeyIn, MouseIn, MouseOut};
use crate::a11y::AccessValue;
use crate::input::{Clipboard, Mods};
use crate::paint::Painter;
use crate::store::Store;

pub struct CodeEditor;

impl ComponentKind for CodeEditor {
    fn name(&self) -> &'static str {
        "RCODEEDITOR"
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        Memo.paint(cx, p);
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        Memo.mouse(cx, m)
    }

    fn pointer(&self, cx: &mut Cx, x: i64, y: i64) -> Cursor {
        Memo.pointer(cx, x, y)
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, clip: &mut dyn Clipboard) -> bool {
        Memo.key(cx, k, clip)
    }

    fn ime(&self, cx: &mut Cx, ime: &Ime) -> bool {
        Memo.ime(cx, ime)
    }

    fn ime_area(&self, cx: &mut Cx) -> Option<Rect> {
        Memo.ime_area(cx)
    }

    fn wants_ime(&self, store: &dyn Store, id: &str) -> bool {
        Memo.wants_ime(store, id)
    }

    fn wheel(&self, cx: &mut Cx, dx: f64, dy: f64, mods: Mods) -> bool {
        Memo.wheel(cx, dx, dy, mods)
    }

    fn tick(&self, cx: &mut Cx) {
        Memo.tick(cx);
    }

    fn context_menu(&self, cx: &mut Cx) -> Option<MenuState> {
        Memo.context_menu(cx)
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        Memo.describe(cx)
    }

    fn access(&self, cx: &mut Cx, action: Action, part: Option<usize>, value: Option<&AccessValue>) -> bool {
        Memo.access(cx, action, part, value)
    }
}

#[cfg(test)]
mod tests {
    use rapidr_value::objects::with_textedit;
    use rapidr_value::objects::ops::{Op, Place};
    use rapidr_value::{v_int, v_str};

    use crate::display::Item;
    use crate::{FormUi, KernelEvent, MemClipboard, MemStore, Mods, TextSystem};

    fn code_form(text: &str) -> (MemStore, FormUi, TextSystem) {
        let mut s = MemStore::new();
        s.add("cf", "RFORM", None).set("cf", "clientwidth", v_int(400)).set("cf", "clientheight", v_int(300));
        s.add("ce", "RCODEEDITOR", Some("cf")).set("ce", "left", v_int(8)).set("ce", "top", v_int(8)).set("ce", "width", v_int(300)).set("ce", "height", v_int(120));
        s.set("ce", "text", v_str(text));
        let f = FormUi::build(&s, "cf", false);
        (s, f, TextSystem::new())
    }

    fn numbers(list: &crate::display::DisplayList) -> Vec<(String, i64)> {
        list.items
            .iter()
            .filter_map(|i| match i {
                Item::Op { origin: (8, 8), op: Op::Text { text, rect, place: Place::TopRight, .. } } => Some((text.clone(), rect.1)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn it_numbers_its_lines_colours_basic_and_types_tabs() {
        let src = (1..=30).map(|i| format!("PRINT {i} ' line")).collect::<Vec<_>>().join("\n");
        let (mut s, mut f, mut ts) = code_form(&src);
        let list = f.paint(&s, &mut ts, 1.0);
        let shown = numbers(&list);
        assert_eq!(shown.first().map(|n| n.0.as_str()), Some("1"));
        assert!(shown.len() > 4 && shown.len() < 12, "only the lines in view: {shown:?}");
        assert!(shown.windows(2).all(|w| w[1].1 > w[0].1), "down the gutter");
        // the paragraphs' runs: PRINT (bold, blue), the number, the comment
        let e = f.node("ce").unwrap().ui.edit.as_ref().unwrap();
        assert_eq!(e.ed.spans(0).len(), 3);
        assert_eq!(e.ed.look().font.pixel_size(), 13);
        assert!(!e.ed.look().wrap);
        // GotoLine scrolls the caret's line into view
        s.call("ce", "gotoline", &[v_int(25)]);
        let list = f.paint(&s, &mut ts, 1.0);
        let shown = numbers(&list);
        assert!(shown.iter().any(|n| n.0 == "26"), "{shown:?}");
        assert_eq!(with_textedit("ce", |t| t.text().lines().nth(25).map(str::to_string)).flatten().as_deref(), Some("PRINT 26 ' line"));
        // Tab types a tab (the focus stays), then OnChange
        f.focus_id(&s, "ce");
        let mut clip = MemClipboard::default();
        f.key_down(&s, &mut ts, 9, "\t", Mods::NONE, &mut clip);
        let events = f.take_events();
        assert!(events.contains(&KernelEvent::Change("ce".into())), "{events:?}");
        assert_eq!(f.focus.map(|i| f.nodes[i].id.clone()).as_deref(), Some("ce"));
        assert!(with_textedit("ce", |t| t.raw().contains("\tPRINT 26")).unwrap());
    }
}
