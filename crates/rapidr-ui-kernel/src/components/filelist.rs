//! QFILELISTBOX: a list box of a directory's files (Directory, Mask,
//! FileType): the list box's model (`ItemList` with its files read from
//! the directory) and drawing, its mouse and keys.

use rapidr_value::objects::a11y::{AccessNode, Action};

use super::list::ListBox;
use super::{ComponentKind, Cx, KeyIn, MouseIn, MouseOut};
use crate::a11y::AccessValue;
use crate::input::Clipboard;
use crate::paint::Painter;

/// QFILELISTBOX: a list box of a directory's files (the same model, its
/// items read from the directory).
pub struct FileListBox;

impl ComponentKind for FileListBox {
    fn name(&self) -> &'static str {
        "RFILELISTBOX"
    }

    fn field(&self) -> bool {
        true
    }
    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        ListBox.paint(cx, p)
    }
    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        ListBox.mouse(cx, m)
    }
    fn wheel(&self, cx: &mut Cx, dx: f64, dy: f64, mods: crate::input::Mods) -> bool {
        ListBox.wheel(cx, dx, dy, mods)
    }
    fn tick(&self, cx: &mut Cx) {
        ListBox.tick(cx)
    }
    fn key(&self, cx: &mut Cx, k: &KeyIn, clip: &mut dyn Clipboard) -> bool {
        ListBox.key(cx, k, clip)
    }
    fn describe(&self, cx: &mut Cx) -> AccessNode {
        ListBox.describe(cx)
    }
    fn access(&self, cx: &mut Cx, action: Action, part: Option<usize>, value: Option<&AccessValue>) -> bool {
        ListBox.access(cx, action, part, value)
    }
    fn test_action(&self, cx: &mut Cx, action: &str) -> bool {
        ListBox.test_action(cx, action)
    }
}
