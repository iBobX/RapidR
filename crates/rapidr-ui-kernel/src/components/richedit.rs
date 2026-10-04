//! QRICHEDIT, step 1 (plan §4): a plain multi-line edit, the same as QMEMO
//! (`memo.rs`) — as FLTK's RRICHEDIT is today, and as `text_edits.bas`
//! needs (its text and selection). Step 2, if wanted, is a style-run model
//! (SelAttributes, Paragraph, RTF) drawn with parley's ranged styles.

use rapidr_value::objects::a11y::{AccessNode, Action};
use rapidr_value::objects::ops::Rect;

use super::edit::MenuState;
use super::memo::Memo;
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
