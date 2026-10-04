//! The lists lane's runtime side of the kernel host (docs/desktop-host-
//! plan.md §2.2 group 3): what the program does for QLISTBOX, QCOMBOBOX,
//! QLISTVIEW, QSTRINGGRID, QHEADER, QTREEVIEW / QOUTLINE and QDIRTREE /
//! QFILELISTBOX, host-neutrally, as gui.rs does it for FLTK's widgets.
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

use crate::object::{get_children_of, rp_comp_get, rp_comp_set, rp_fire_event, rp_fire_event_1, rp_fire_event_args, rp_fire_event_then, rp_has_handler};
use crate::value::{v_int, v_str, Value};

use super::kernel::{invalidate_all, is_shown_form};

// ------------------------------------------------------------ dispatch --

/// What the user did to component `id` (after the pump, each to
/// completion), as gui.rs's callbacks do it.
pub fn dispatch(id: &str, action: ListAction) {
    match action {
        ListAction::Fire(event, args) => rp_fire_event_args(id, &event, &args),
        ListAction::TreeSelect(i) => tree_user_select(id, i),
        ListAction::TreeToggle(i, open) => tree_user_toggle(id, i, open),
        ListAction::TreeEdit(i) => tree_begin_edit(id, i),
        ListAction::TreeEdited(i, text) => tree_end_edit(id, i, text),
        ListAction::GridSelect(c, r, extend) => {
            grid_user_select(id, c, r, extend);
        }
        ListAction::GridStore(value) => grid_store(id, value),
    }
    invalidate_all();
}

/// The user picked node `i`: OnChanging (Index, AllowChange) may refuse;
/// then the selection and OnChange (Index). (gui.rs's `tree_user_select`.)
fn tree_user_select(name: &str, i: usize) {
    if rapidr_value::objects::with_tree(name, |m| m.item_index) == Some(i as i64) {
        return;
    }
    let tree = name.to_string();
    rp_fire_event_then(name, "onchanging", &[v_int(i as i64), v_int(-1)], move |a| {
        let allowed = a[1].to_i64() != 0;
        if allowed {
            rapidr_value::objects::with_tree(&tree, |m| m.select(i as i64));
        }
        invalidate_all();
        if allowed {
            rp_fire_event_1(&tree, "onchange", v_int(i as i64));
        }
    });
}

/// The user expanded (`open`) or collapsed node `i`: OnExpanding /
/// OnCollapsing may refuse; then OnExpanded / OnCollapsed.
fn tree_user_toggle(name: &str, i: usize, open: bool) {
    let tree = name.to_string();
    rp_fire_event_then(name, if open { "onexpanding" } else { "oncollapsing" }, &[v_int(i as i64), v_int(-1)], move |a| {
        let allowed = a[1].to_i64() != 0;
        if allowed {
            rapidr_value::objects::with_tree(&tree, |m| m.set_expanded(i, open, false));
        }
        invalidate_all();
        if allowed {
            rp_fire_event_1(&tree, if open { "onexpanded" } else { "oncollapsed" }, v_int(i as i64));
        }
    });
}

/// F2 on node `i`: not in a ReadOnly tree; OnEditing (Index, AllowEdit)
/// may refuse; then its editor opens.
fn tree_begin_edit(name: &str, i: usize) {
    if rapidr_value::objects::with_tree(name, |m| m.read_only || i >= m.nodes.len()).unwrap_or(true) {
        return;
    }
    let tree = name.to_string();
    rp_fire_event_then(name, "onediting", &[v_int(i as i64), v_int(-1)], move |a| {
        if a[1].to_i64() == 0 {
            return;
        }
        let Some(text) = rapidr_value::objects::with_tree(&tree, |m| m.nodes.get(i).map(|n| n.text.clone())).flatten() else { return };
        open_editor(&tree, i, &text);
        invalidate_all();
    });
}

/// The edit of node `i` kept: OnEdited (Index, S) — S the text, which the
/// program may change — and the node gets it.
fn tree_end_edit(name: &str, i: usize, text: String) {
    let tree = name.to_string();
    rp_fire_event_then(name, "onedited", &[v_int(i as i64), v_str(&text)], move |a| {
        let text = a[1].to_string_val();
        rapidr_value::objects::with_tree(&tree, |m| m.set_text(i, text));
        invalidate_all();
    });
}

/// A selection the user made (`StringGrid::user_select`): when it moved,
/// OnSelectCell (Col, Row, CanSelect); `CanSelect = 0` puts it back.
fn grid_user_select(name: &str, c: i64, r: i64, extend: bool) -> bool {
    let Some(before) = rapidr_value::objects::with_grid_mut(name, |g| g.user_select(c, r, extend)).flatten() else {
        return false;
    };
    let grid = name.to_string();
    rp_fire_event_then(name, "onselectcell", &[v_int(c), v_int(r), v_int(-1)], move |a| {
        if a[2].to_i64() == 0 {
            rapidr_value::objects::with_grid_mut(&grid, |g| g.set_selection(before));
            invalidate_all();
        }
    });
    true
}

/// The user entered `value` in the selected cell: stored, then
/// OnSetEditText (Col, Row, Value) and OnChange, if it changed.
fn grid_store(name: &str, value: String) {
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
        rp_fire_event_args(name, "onsetedittext", &[v_int(c), v_int(r), v_str(&value)]);
        rp_fire_event(name, "onchange");
    }
}

// --------------------------------------------------- tree's own calls --

/// QTREEVIEW changed: OnDeletion (Index) for the nodes the program deleted
/// (as FLTK's `tree_refresh`).
pub fn tree_refresh(name: &str) {
    let name = name.to_lowercase();
    for i in rapidr_value::objects::with_tree(&name, |m| m.take_deleted()).unwrap_or_default() {
        rp_fire_event_1(&name, "ondeletion", v_int(i as i64));
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
fn components(parent: &str, out: &mut Vec<(String, String)>) {
    for (id, t) in get_children_of(parent) {
        let id = id.to_lowercase();
        out.push((id.clone(), t.to_ascii_uppercase()));
        components(&id, out);
    }
}

/// Whether `id` shows: Visible up to its form (which shows).
fn shown(id: &str) -> bool {
    let mut cur = id.to_lowercase();
    for _ in 0..64 {
        if is_shown_form(&cur) {
            return true;
        }
        if !super::kernel_store::flag(&cur, "visible", true) {
            return false;
        }
        let p = rp_comp_get(&cur, "parent").to_string_val();
        if p.is_empty() {
            return false;
        }
        cur = p.to_lowercase();
    }
    false
}

/// A component's Width × Height (RapidQ's default size when unset).
fn size_of(id: &str, type_name: &str) -> (i64, i64) {
    let (dw, dh) = rapidr_value::layout::default_size(type_name).unwrap_or((75, 25));
    let int = |p: &str, d: i64| match rp_comp_get(id, p) {
        Value::Null => d,
        v => v.to_i64(),
    };
    (int("width", dw), int("height", dh))
}

/// The owner-draw events of the shown forms' components, before the
/// windows are drawn (plan §1.5 rule 3: what OnDrawItem / OnDrawCell /
/// OnDrawSection draw is recorded by the models as what the kernel shows).
pub fn pre_paint(forms: &[String]) {
    let mut comps = Vec::new();
    for f in forms {
        components(f, &mut comps);
    }
    for (id, t) in comps {
        match t.as_str() {
            "RLISTBOX" | "RCOMBOBOX" if shown(&id) => list_pre_paint(&id, &t),
            "RSTRINGGRID" if shown(&id) => {
                // (VisibleRowCount / VisibleColCount: its inside, as FLTK's
                // table sets it; the kernel paints only when a window does)
                let (w, h) = size_of(&id, &t);
                rapidr_value::objects::with_grid_mut(&id, |g| g.view = (w - 4, h - 4));
                grid_owner_draw(&id);
            }
            "RHEADER" if shown(&id) => header_paint(&id),
            "RTREEVIEW" if shown(&id) => tree_ask_images(&id),
            _ => {}
        }
    }
}

/// An owner-drawn or multi-column list: its view's size, OnMeasureItem's
/// heights (lbOwnerDrawVariable), then OnDrawItem for every item after a
/// change (gui.rs's `list_refresh` for its table, `list_measure`,
/// `list_owner_draw`).
fn list_pre_paint(name: &str, type_name: &str) {
    if !rapidr_value::objects::with_list(name, |l| l.custom_drawn() || (l.combo && l.owner_drawn())).unwrap_or(false) {
        return;
    }
    let (w, h) = size_of(name, type_name);
    rapidr_value::objects::with_list_mut(name, |l| {
        let (vw, vh) = view_size(l, w, h);
        l.set_view(vw, vh);
    });
    if list_measure(name) {
        return;
    }
    list_owner_draw(name);
}

/// OnMeasureItem (Index, Height) for each item of a variable-height list
/// whose items changed: `true` while answers are still to come.
fn list_measure(name: &str) -> bool {
    if rp_has_handler(name, "onmeasureitem") {
        let asks = rapidr_value::objects::with_list_mut(name, |l| l.measure_needed()).unwrap_or_default();
        for (round, i, h) in asks {
            let list = name.to_string();
            rp_fire_event_then(name, "onmeasureitem", &[v_int(i as i64), v_int(h)], move |a| {
                if rapidr_value::objects::with_list_mut(&list, |l| l.measured(round, i, a[1].to_i64())).unwrap_or(false) {
                    invalidate_all();
                }
            });
        }
    }
    rapidr_value::objects::with_list(name, |l| l.measuring()).unwrap_or(false)
}

/// OnDrawItem (Index, State, Rect) for every item after the list changed;
/// each Rect a QRECT (a property bag).
fn list_owner_draw(name: &str) {
    if !rp_has_handler(name, "ondrawitem") {
        return;
    }
    if !rapidr_value::objects::with_list_mut(name, |l| l.owner_drawn() && l.owner_draw_needed()).unwrap_or(false) {
        return;
    }
    let items = rapidr_value::objects::with_list(name, |l| l.owner_draw_items()).unwrap_or_default();
    for (i, state, (left, top, right, bottom)) in items {
        let rect = format!("{name}.itemrect({i})");
        for (prop, v) in [("left", left), ("top", top), ("right", right), ("bottom", bottom)] {
            rp_comp_set(&rect, prop, v_int(v));
        }
        rp_fire_event_args(name, "ondrawitem", &[v_int(i as i64), v_int(state), v_str(&rect)]);
    }
    invalidate_all();
}

/// OnDrawCell (Col, Row, State, Rect) for every cell after a shown grid
/// changed (gui.rs's `grid_owner_draw`).
fn grid_owner_draw(name: &str) {
    if !rp_has_handler(name, "ondrawcell") {
        return;
    }
    if !rapidr_value::objects::with_grid_mut(name, |g| g.owner_draw_needed()).unwrap_or(false) {
        return;
    }
    let cells = rapidr_value::objects::with_grid(name, |g| g.owner_draw_cells()).unwrap_or_default();
    for (col, row, state, (left, top, right, bottom)) in cells {
        let rect = format!("{name}.cellrect({col},{row})");
        for (prop, v) in [("left", left), ("top", top), ("right", right), ("bottom", bottom)] {
            rp_comp_set(&rect, prop, v_int(v));
        }
        rp_fire_event_args(name, "ondrawcell", &[v_int(col as i64), v_int(row as i64), v_int(state), v_str(&rect)]);
    }
    invalidate_all();
}

/// A shown QHEADER's faces painted on its surface again when its size or
/// sections changed (or one is pressed), and OnDrawSection (Index,
/// Pressed, Rect) for its owner-drawn ones (gui.rs's `header_refresh`).
fn header_paint(name: &str) {
    let (w, h) = size_of(name, "RHEADER");
    let Some(state) = rapidr_value::objects::with_header(name, |hd| format!("{w}x{h} {:?} {:?}", hd.pressed, hd.sections)) else { return };
    if HEADERS.with(|s| s.borrow().get(name) == Some(&state)) {
        return;
    }
    HEADERS.with(|s| s.borrow_mut().insert(name.to_string(), state));
    for (i, pressed, (left, top, right, bottom)) in rapidr_value::objects::paint_header(name, w, h) {
        let rect = format!("{name}.sectionrect({i})");
        for (prop, v) in [("left", left), ("top", top), ("right", right), ("bottom", bottom)] {
            rp_comp_set(&rect, prop, v_int(v));
        }
        rp_fire_event_args(name, "ondrawsection", &[v_int(i as i64), v_int(if pressed { -1 } else { 0 }), v_str(&rect)]);
    }
    invalidate_all();
}

/// OnGetImageIndex (Index) for each shown node, OnGetSelectedIndex (Index)
/// for the selected one, when what the tree shows changed (gui.rs's
/// `tree_ask_images`).
fn tree_ask_images(name: &str) {
    let ask = |e: &str| rp_has_handler(name, e);
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
        rp_fire_event_1(name, event, v_int(i as i64));
    }
    TREES_ASKING.with(|a| a.borrow_mut().remove(&key));
    invalidate_all();
}
