//! The lists lane's program side of the kernel (docs/desktop-host-plan.md
//! §2.2 group 3; runtime-core's `ui/kernel_lists.rs` until Stage W2): what
//! the program does for QLISTBOX, QCOMBOBOX, QLISTVIEW, QSTRINGGRID,
//! QHEADER, QTREEVIEW / QOUTLINE and QDIRTREE / QFILELISTBOX,
//! host-neutrally.
//!
//! - [`dispatch`]: what the user did that the program answers
//!   (`ListAction`, queued by the kernel's components): OnChanging,
//!   OnExpanding / OnCollapsing, OnEditing, OnEdited, OnSelectCell … ask
//!   first, then the model changes.
//! - [`pre_paint`]: before each pump, the owner-draw events of shown
//!   components — OnMeasureItem / OnDrawItem (lists), OnDrawCell (grids,
//!   only once on screen, as RapidQ), OnDrawSection (headers, their faces
//!   painted on their surface), OnGetImageIndex / OnGetSelectedIndex
//!   (trees) — recorded by the models as what the kernel draws.
//! - [`tree_refresh`]: OnDeletion for nodes the program deleted.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use rapidr_ui_kernel::components::list::{view_size, ListAction};
use rapidr_ui_kernel::components::tree::open_editor;

use rapidr_value::{v_int, v_str, Value};

use crate::forms::form_shown;
use crate::windows::invalidate;
use crate::Program;

// ------------------------------------------------------------ dispatch --

/// What the user did to component `id` (after the pump, each to
/// completion).
pub fn dispatch<P: Program>(p: P, id: &str, action: ListAction) {
    match action {
        ListAction::Fire(event, args) => p.fire_args(id, &event, &args),
        ListAction::TreeSelect(i) => tree_user_select(p, id, i),
        ListAction::TreeToggle(i, open) => tree_user_toggle(p, id, i, open),
        ListAction::TreeEdit(i) => tree_begin_edit(p, id, i),
        ListAction::TreeEdited(i, text) => tree_end_edit(p, id, i, text),
        ListAction::GridSelect(c, r, extend) => {
            grid_user_select(p, id, c, r, extend);
        }
        // (I1 / L-PANELS: a list an inspector or a toolbar dropped)
        ListAction::GridStore(value) if rapidr_value::panels::is_panel(&p.type_of(id)) => {
            p.container(rapidr_ui_kernel::components::form::Container::Panel { id: id.to_string(), action: rapidr_value::panels::User::Picked(value) })
        }
        ListAction::GridStore(value) => grid_store(p, id, value),
        ListAction::GridListDrop(c, r, anchor) => grid_list_drop(p, id, c, r, anchor),
    }
    invalidate();
}

/// The user picked node `i`: OnChanging (Index, AllowChange) may refuse;
/// then the selection and OnChange (Index).
fn tree_user_select<P: Program>(p: P, name: &str, i: usize) {
    if rapidr_value::objects::with_tree(name, |m| m.item_index) == Some(i as i64) {
        return;
    }
    let tree = name.to_string();
    p.fire_then(
        name,
        "onchanging",
        &[v_int(i as i64), v_int(-1)],
        Box::new(move |a| {
            let allowed = a[1].to_i64() != 0;
            if allowed {
                rapidr_value::objects::with_tree(&tree, |m| m.select(i as i64));
            }
            invalidate();
            if allowed {
                p.fire_args(&tree, "onchange", &[v_int(i as i64)]);
            }
        }),
    );
}

/// The user expanded (`open`) or collapsed node `i`: OnExpanding /
/// OnCollapsing may refuse; then OnExpanded / OnCollapsed.
fn tree_user_toggle<P: Program>(p: P, name: &str, i: usize, open: bool) {
    let tree = name.to_string();
    p.fire_then(
        name,
        if open { "onexpanding" } else { "oncollapsing" },
        &[v_int(i as i64), v_int(-1)],
        Box::new(move |a| {
            let allowed = a[1].to_i64() != 0;
            if allowed {
                rapidr_value::objects::with_tree(&tree, |m| m.set_expanded(i, open, false));
            }
            invalidate();
            if allowed {
                p.fire_args(&tree, if open { "onexpanded" } else { "oncollapsed" }, &[v_int(i as i64)]);
            }
        }),
    );
}

/// F2 on node `i`: not in a ReadOnly tree; OnEditing (Index, AllowEdit)
/// may refuse; then its editor opens.
fn tree_begin_edit<P: Program>(p: P, name: &str, i: usize) {
    if rapidr_value::objects::with_tree(name, |m| m.read_only || i >= m.nodes.len()).unwrap_or(true) {
        return;
    }
    let tree = name.to_string();
    p.fire_then(
        name,
        "onediting",
        &[v_int(i as i64), v_int(-1)],
        Box::new(move |a| {
            if a[1].to_i64() == 0 {
                return;
            }
            let Some(text) = rapidr_value::objects::with_tree(&tree, |m| m.nodes.get(i).map(|n| n.text.clone())).flatten() else { return };
            open_editor(&tree, i, &text);
            invalidate();
        }),
    );
}

/// The edit of node `i` kept: OnEdited (Index, S) — S the text, which the
/// program may change — and the node gets it.
fn tree_end_edit<P: Program>(p: P, name: &str, i: usize, text: String) {
    let tree = name.to_string();
    p.fire_then(
        name,
        "onedited",
        &[v_int(i as i64), v_str(&text)],
        Box::new(move |a| {
            let text = a[1].to_string_val();
            rapidr_value::objects::with_tree(&tree, |m| m.set_text(i, text));
            invalidate();
        }),
    );
}

/// A selection the user made (`StringGrid::user_select`): when it moved,
/// OnSelectCell (Col, Row, CanSelect); `CanSelect = 0` puts it back.
fn grid_user_select<P: Program>(p: P, name: &str, c: i64, r: i64, extend: bool) -> bool {
    let Some(before) = rapidr_value::objects::with_grid_mut(name, |g| g.user_select(c, r, extend)).flatten() else {
        return false;
    };
    let grid = name.to_string();
    p.fire_then(
        name,
        "onselectcell",
        &[v_int(c), v_int(r), v_int(-1)],
        Box::new(move |a| {
            if a[2].to_i64() == 0 {
                rapidr_value::objects::with_grid_mut(&grid, |g| g.set_selection(before));
                invalidate();
            }
        }),
    );
    true
}

/// The user entered `value` in the selected cell: stored, then
/// OnSetEditText (Col, Row, Value) and OnChange, if it changed.
fn grid_store<P: Program>(p: P, name: &str, value: String) {
    let changed = rapidr_value::objects::with_grid_mut(name, |g| {
        let (c, r) = (g.col, g.row);
        if c < 0 || r < 0 || g.cell(c as usize, r as usize) == value {
            return None;
        }
        g.set_cell(c as usize, r as usize, value.clone());
        Some((c, r))
    })
    .flatten();
    if let Some((c, r)) = changed {
        p.fire_args(name, "onsetedittext", &[v_int(c), v_int(r), v_str(&value)]);
        p.fire(name, "onchange");
    }
}

/// A gcsList column's drop-down: OnListDropDown (Col, Row, S) may change
/// the items (its S comes back); then the kernel drops them under the
/// cell, and a pick is stored like an edit.
fn grid_list_drop<P: Program>(p: P, name: &str, c: i64, r: i64, anchor: rapidr_value::objects::ops::Rect) {
    let Some(list) = rapidr_value::objects::with_grid(name, |g| g.list_text(c as usize, r as usize)).flatten() else { return };
    let Some(form) = p.form_of(name) else { return };
    let grid = name.to_string();
    p.fire_then(
        name,
        "onlistdropdown",
        &[v_int(c), v_int(r), v_str(&list)],
        Box::new(move |a| {
            let items = rapidr_value::objects::grid::list_lines(&a[2].to_string_val());
            rapidr_ui_kernel::components::combo::open_list(&form, &grid, items, anchor);
            invalidate();
        }),
    );
}

// --------------------------------------------------- tree's own calls --

/// QTREEVIEW changed: OnDeletion (Index) for the nodes the program
/// deleted.
pub fn tree_refresh<P: Program>(p: P, name: &str) {
    let name = name.to_lowercase();
    for i in rapidr_value::objects::with_tree(&name, |m| m.take_deleted()).unwrap_or_default() {
        p.fire_args(&name, "ondeletion", &[v_int(i as i64)]);
    }
}

/// `GetItemAt (X, Y)`: the node in the row there (the kernel's rows, inside
/// the 2-pixel frame), or -1.
pub fn tree_item_at(name: &str, x: i64, y: i64) -> i64 {
    use rapidr_value::objects::tree::ROW_HEIGHT;
    rapidr_value::objects::with_tree(name, |m| m.item_at(x - 2, y - 2, ROW_HEIGHT)).unwrap_or(-1)
}

// ----------------------------------------------------------- pre-paint --

thread_local! {
    /// Each tree asking its program for icons (OnGetImageIndex), and what
    /// it showed when it last asked (`TreeView::view_hash`).
    static TREES_ASKING: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    static TREES_ASKED: RefCell<HashMap<String, u64>> = RefCell::new(HashMap::new());
    /// What each header's faces were last painted for (size, sections,
    /// pressed).
    static HEADERS: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
}

/// The components of shown form `form`, depth first (id, type).
fn components<P: Program>(p: P, parent: &str, out: &mut Vec<(String, String)>) {
    for (id, t) in p.children(parent) {
        let id = id.to_lowercase();
        out.push((id.clone(), t.to_ascii_uppercase()));
        components(p, &id, out);
    }
}

/// Whether `id` shows: Visible up to its form (which shows).
fn shown<P: Program>(p: P, id: &str) -> bool {
    let mut cur = id.to_lowercase();
    for _ in 0..64 {
        if form_shown(&cur) {
            return true;
        }
        if !p.flag(&cur, "visible", true) {
            return false;
        }
        let parent = p.get(&cur, "parent").to_string_val();
        if parent.is_empty() {
            return false;
        }
        cur = parent.to_lowercase();
    }
    false
}

/// A component's Width × Height (RapidQ's default size when unset).
fn size_of<P: Program>(p: P, id: &str, type_name: &str) -> (i64, i64) {
    let (dw, dh) = rapidr_value::layout::default_size(type_name).unwrap_or((75, 25));
    let int = |prop: &str, d: i64| match p.get(id, prop) {
        Value::Null => d,
        v => v.to_i64(),
    };
    (int("width", dw), int("height", dh))
}

/// The owner-draw events of the shown forms' components, before the
/// windows are drawn (plan §1.5 rule 3: what OnDrawItem / OnDrawCell /
/// OnDrawSection draw is recorded by the models as what the kernel shows).
pub fn pre_paint<P: Program>(p: P, forms: &[String]) {
    let mut comps = Vec::new();
    for f in forms {
        components(p, f, &mut comps);
    }
    for (id, t) in comps {
        match t.as_str() {
            "RLISTBOX" | "RCOMBOBOX" if shown(p, &id) => list_pre_paint(p, &id, &t),
            "RSTRINGGRID" if shown(p, &id) => {
                // (VisibleRowCount / VisibleColCount: its inside — set here,
                // since the kernel paints only when a window does)
                let (w, h) = size_of(p, &id, &t);
                rapidr_value::objects::with_grid_mut(&id, |g| g.view = (w - 4, h - 4));
                grid_owner_draw(p, &id);
            }
            "RHEADER" if shown(p, &id) => header_paint(p, &id),
            "RTREEVIEW" if shown(p, &id) => tree_ask_images(p, &id),
            _ => {}
        }
    }
}

/// An owner-drawn or multi-column list: its view's size, OnMeasureItem's
/// heights (lbOwnerDrawVariable), then OnDrawItem for every item after a
/// change.
fn list_pre_paint<P: Program>(p: P, name: &str, type_name: &str) {
    if !rapidr_value::objects::with_list(name, |l| l.custom_drawn() || (l.combo && l.owner_drawn())).unwrap_or(false) {
        return;
    }
    let (w, h) = size_of(p, name, type_name);
    rapidr_value::objects::with_list_mut(name, |l| {
        let (vw, vh) = view_size(l, w, h);
        l.set_view(vw, vh);
    });
    if list_measure(p, name) {
        return;
    }
    list_owner_draw(p, name);
}

/// OnMeasureItem (Index, Height) for each item of a variable-height list
/// whose items changed: `true` while answers are still to come.
fn list_measure<P: Program>(p: P, name: &str) -> bool {
    if p.has_handler(name, "onmeasureitem") {
        let asks = rapidr_value::objects::with_list_mut(name, |l| l.measure_needed()).unwrap_or_default();
        for (round, i, h) in asks {
            let list = name.to_string();
            p.fire_then(
                name,
                "onmeasureitem",
                &[v_int(i as i64), v_int(h)],
                Box::new(move |a| {
                    if rapidr_value::objects::with_list_mut(&list, |l| l.measured(round, i, a[1].to_i64())).unwrap_or(false) {
                        invalidate();
                    }
                }),
            );
        }
    }
    rapidr_value::objects::with_list(name, |l| l.measuring()).unwrap_or(false)
}

/// OnDrawItem (Index, State, Rect) for every item after the list changed;
/// each Rect a QRECT (a property bag).
fn list_owner_draw<P: Program>(p: P, name: &str) {
    if !p.has_handler(name, "ondrawitem") {
        return;
    }
    if !rapidr_value::objects::with_list_mut(name, |l| l.owner_drawn() && l.owner_draw_needed()).unwrap_or(false) {
        return;
    }
    let items = rapidr_value::objects::with_list(name, |l| l.owner_draw_items()).unwrap_or_default();
    for (i, state, (left, top, right, bottom)) in items {
        let rect = format!("{name}.itemrect({i})");
        for (prop, v) in [("left", left), ("top", top), ("right", right), ("bottom", bottom)] {
            p.set(&rect, prop, v_int(v));
        }
        p.fire_args(name, "ondrawitem", &[v_int(i as i64), v_int(state), v_str(&rect)]);
    }
    invalidate();
}

/// OnDrawCell (Col, Row, State, Rect) for every cell after a shown grid
/// changed.
fn grid_owner_draw<P: Program>(p: P, name: &str) {
    if !p.has_handler(name, "ondrawcell") {
        return;
    }
    if !rapidr_value::objects::with_grid_mut(name, |g| g.owner_draw_needed()).unwrap_or(false) {
        return;
    }
    let cells = rapidr_value::objects::with_grid(name, |g| g.owner_draw_cells()).unwrap_or_default();
    for (col, row, state, (left, top, right, bottom)) in cells {
        let rect = format!("{name}.cellrect({col},{row})");
        for (prop, v) in [("left", left), ("top", top), ("right", right), ("bottom", bottom)] {
            p.set(&rect, prop, v_int(v));
        }
        p.fire_args(name, "ondrawcell", &[v_int(col as i64), v_int(row as i64), v_int(state), v_str(&rect)]);
    }
    invalidate();
}

/// A shown QHEADER's faces painted on its surface again when its size or
/// sections changed (or one is pressed), and OnDrawSection (Index,
/// Pressed, Rect) for its owner-drawn ones.
fn header_paint<P: Program>(p: P, name: &str) {
    let (w, h) = size_of(p, name, "RHEADER");
    let Some(state) = rapidr_value::objects::with_header(name, |hd| format!("{w}x{h} {:?} {:?}", hd.pressed, hd.sections)) else { return };
    if HEADERS.with(|s| s.borrow().get(name) == Some(&state)) {
        return;
    }
    HEADERS.with(|s| s.borrow_mut().insert(name.to_string(), state));
    for (i, pressed, (left, top, right, bottom)) in rapidr_value::objects::paint_header(name, w, h) {
        let rect = format!("{name}.sectionrect({i})");
        for (prop, v) in [("left", left), ("top", top), ("right", right), ("bottom", bottom)] {
            p.set(&rect, prop, v_int(v));
        }
        p.fire_args(name, "ondrawsection", &[v_int(i as i64), v_int(if pressed { -1 } else { 0 }), v_str(&rect)]);
    }
    invalidate();
}

/// OnGetImageIndex (Index) for each shown node, OnGetSelectedIndex (Index)
/// for the selected one, when what the tree shows changed.
fn tree_ask_images<P: Program>(p: P, name: &str) {
    let ask = |e: &str| p.has_handler(name, e);
    let key = name.to_lowercase();
    let Some(view) = rapidr_value::objects::with_tree(name, |m| m.view_hash()) else { return };
    if !(ask("ongetimageindex") || ask("ongetselectedindex")) || TREES_ASKED.with(|a| a.borrow().get(&key) == Some(&view)) {
        return;
    }
    if !TREES_ASKING.with(|a| a.borrow_mut().insert(key.clone())) {
        return;
    }
    TREES_ASKED.with(|a| a.borrow_mut().insert(key.clone(), view));
    let (rows, selected) = rapidr_value::objects::with_tree(name, |m| (m.visible_rows(), m.item_index)).unwrap_or_default();
    for i in rows {
        let event = if i as i64 == selected { "ongetselectedindex" } else { "ongetimageindex" };
        p.fire_args(name, event, &[v_int(i as i64)]);
    }
    TREES_ASKING.with(|a| a.borrow_mut().remove(&key));
    invalidate();
}
