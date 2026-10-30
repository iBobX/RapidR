//! QRICHEDIT: QMEMO's multi-line edit (`memo.rs`) over a formatted text —
//! the shared model's formats (`rapidr_value::objects::richtext`: RapidQ's
//! SelAttributes, Font and Alignment, RapidR's Paragraph) drawn as parley's
//! ranged styles, each paragraph with its alignment, indents and bullet
//! (`edit.rs`'s `restyle`). What the user types takes the format RapidQ's
//! rich edit gives it (the model's rules), and Ctrl+Z puts the formats
//! back with the text. Screen readers get the text's runs with their
//! formats.

use rapidr_value::i18n::tr_ui_fill;
use rapidr_value::objects::a11y::{AccessNode, Action};
use rapidr_value::objects::ops::Rect;
use rapidr_value::objects::with_textedit;

use super::edit::{text_format, MenuState};
use super::memo::Memo;
use rapidr_value::input::Cursor;
use super::{ComponentKind, Cx, Ime, KeyIn, MouseIn, MouseOut};
use crate::a11y::AccessValue;
use crate::input::{Clipboard, Mods};
use crate::paint::Painter;
use crate::store::Store;

pub struct RichEdit;

impl ComponentKind for RichEdit {
    fn name(&self) -> &'static str {
        "RRICHEDIT"
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

    /// A multi-line text field whose text runs carry their formats (font,
    /// size, bold, italic, underline, strike-out, colour: what a screen
    /// reader reports as the text's attributes), described with the format
    /// at the caret ("At the caret: Courier New 12 point, bold, red").
    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = Memo.describe(cx);
        if let Some(f) = with_textedit(cx.id, |t| t.caret_format()).flatten() {
            let spoken = tr_ui_fill("At the caret: {format}", &[("format", &text_format(&f).spoken())]);
            n.description = if n.description.is_empty() { spoken } else { format!("{}; {spoken}", n.description) };
        }
        n.text = Memo::rich_access(cx).map(Box::new);
        n
    }

    fn access(&self, cx: &mut Cx, action: Action, part: Option<usize>, value: Option<&AccessValue>) -> bool {
        Memo.access(cx, action, part, value)
    }
}
