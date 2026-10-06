//! The code editor for screen readers: a multi-line text field whose value
//! is the lines around the caret (a 10 MB file never goes into a tree or a
//! page's element), described with the caret's line and column and the
//! caret line's problems; a polite live region (a status node) announcing
//! line / column moves and problems as they change; the completion list
//! as a list box with its active option selected.

use rapidr_value::objects::a11y::{node_id, AccessNode, Action, Role};

use super::{popup, with_view, Ctx};
use crate::a11y::AccessValue;
use crate::components::{shared_describe, Cx};

/// The node of the editor.
pub fn describe(cx: &mut Cx) -> AccessNode {
    let mut n = shared_describe(cx, "RCODEEDITOR");
    let (x0, y0) = (cx.rect.0, cx.rect.1);
    let extra = with_view(cx, |x| {
        let head = x.c.doc.selections().primary().head;
        let (line, col) = x.c.line_col(head);
        let (errors, warnings, here) = x.c.line_severity_summary();
        let mut d = format!("Line {line}, column {col}");
        if x.c.doc.selections().len() > 1 {
            d.push_str(&format!(", {} cursors", x.c.doc.selections().len()));
        }
        if errors + warnings > 0 {
            d.push_str(&format!("; {errors} errors, {warnings} warnings"));
        }
        for h in &here {
            d.push_str("; ");
            d.push_str(h);
        }
        let mut kids = Vec::new();
        // (the live region: what was just announced)
        let mut live = AccessNode::new(node_id(&format!("{}$live", x.id)), Role::Status);
        live.name = x.ui.announce.clone();
        live.bounds = (x0, y0, 0, 0);
        kids.push(live);
        // (the completion list, its active item selected)
        if let Some(list) = &x.c.completion {
            let items = popup::filtered(x);
            if !items.is_empty() {
                let mut lb = AccessNode::new(node_id(&format!("{}$completion", x.id)), Role::ListBox);
                lb.name = "Suggestions".into();
                lb.bounds = (x0, y0, 0, 0);
                let sel = list.selected.min(items.len() - 1);
                for (k, &i) in items.iter().enumerate().take(200) {
                    let it = &list.items[i];
                    let mut o = AccessNode::new(node_id(&format!("{}$completion${k}", x.id)), Role::ListBoxOption);
                    o.name = if it.detail.is_empty() { format!("{}, {}", it.label, it.kind.name()) } else { format!("{}, {}, {}", it.label, it.kind.name(), it.detail) };
                    o.states.selected = Some(k == sel);
                    o.states.focused = k == sel;
                    o.bounds = (x0, y0, 0, 0);
                    lb.children.push(o);
                }
                kids.push(lb);
            }
        }
        (d, kids)
    });
    if let Some((d, kids)) = extra {
        if !n.description.is_empty() {
            n.description.push_str(". ");
        }
        n.description.push_str(&d);
        n.role = Role::MultilineTextInput;
        n.states.multiline = true;
        n.children.extend(kids);
    }
    if n.name.is_empty() {
        n.name = "Code editor".into();
    }
    n
}

/// A screen reader's request: focus, or (SetValue) the window of lines it
/// was shown replaced by its new text, as typing would.
pub fn access(cx: &mut Cx, action: Action, _part: Option<usize>, value: Option<&AccessValue>) -> bool {
    let (Action::SetValue, Some(AccessValue::Text(s))) = (action, value) else { return action == Action::Focus };
    let s = s.clone();
    let changed = with_view(cx, |x| {
        if x.c.doc.read_only {
            return false;
        }
        let w = x.c.text_window(100);
        if w.text == s {
            return false;
        }
        let a = x.c.byte_of(w.first);
        let b = a + w.text.len();
        let _ = x.c.doc.replace_range(a..b, &s, super::input::now_ms());
        super::input::edited(x, None);
        true
    })
    .unwrap_or(false);
    let _ = changed;
    true
}

/// The caret moved: its line and column announced (and the new line's
/// problems).
pub fn caret_moved(x: &mut Ctx, line_changed: bool) {
    let (line, col) = x.ui.last_caret;
    x.ui.announce = if line_changed {
        let (_, _, here) = x.c.line_severity_summary();
        let mut a = format!("Line {line}");
        for h in here {
            a.push_str(", ");
            a.push_str(&h);
        }
        a
    } else {
        format!("Column {col}")
    };
}

/// The completion list's active item changed: announced.
pub fn completion_moved(x: &mut Ctx) {
    let items = popup::filtered(x);
    if let Some(list) = &x.c.completion {
        if let Some(&i) = items.get(list.selected.min(items.len().saturating_sub(1))) {
            let it = &list.items[i];
            x.ui.announce = format!("{} ({}), {} of {}", it.label, it.kind.name(), list.selected + 1, items.len());
        }
    }
}

/// The problems changed: the counts and the caret line's announced.
pub fn announce_line_problems(x: &mut Ctx) {
    let (errors, warnings, here) = x.c.line_severity_summary();
    x.ui.announce = if here.is_empty() { format!("{errors} errors, {warnings} warnings") } else { here.join("; ") };
}
