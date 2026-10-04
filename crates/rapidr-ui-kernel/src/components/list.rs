//! QLISTBOX (and QFILELISTBOX, whose items are a directory's files): the
//! shared model (`rapidr_value::objects::list::ItemList`) holds the items
//! and the selection; this draws them in Windows' classic list box (a
//! sunken white box, the selected item white on blue, the focused one
//! dotted) and takes the mouse and the keys as the FLTK and web runtimes
//! route them — a click selects (MultiSelect: Shift extends, Ctrl
//! toggles) and fires OnClick, the arrows, Page Up / Down, Home and End
//! move the selection (OnClick too).
//!
//! An owner-drawn list box (Style lbOwnerDrawFixed / lbOwnerDrawVariable)
//! or one in Columns shows each item as the model renders it
//! (`ItemList::render_item`: what OnDrawItem drew, kept per item; runtime-
//! core fires OnMeasureItem / OnDrawItem before the pump, as FLTK's
//! `list_owner_draw` does).
//!
//! Also here, what the lists lane's components share: [`ListAction`]
//! (what the user did that the program answers, done by runtime-core),
//! the in-place editors of trees, list views and grids, the vertical
//! scroll bar (the shared `scrollbars::Scroller`), the sunken frame and
//! the replay of what an owner-draw handler drew ([`replay`]).

use std::cell::RefCell;
use std::collections::HashMap;

use rapidr_value::objects::a11y::{node_id, part_id, AccessNode, Action, Role};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::grid::CellDraw;
use rapidr_value::objects::list::ItemList;
use rapidr_value::objects::ops::{lift, Op, Place, Rect};
use rapidr_value::objects::trackbar::Shape;
use rapidr_value::objects::{with_list, with_list_mut};
use rapidr_value::scrollbars::{Child, Scroller};
use rapidr_value::Value;

use super::{ComponentKind, Cx, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::a11y::AccessValue;
use crate::display::Picture;
use crate::input::{Clipboard, KernelEvent};
use crate::paint::{Painter, DARK, FACE, HIGHLIGHT, HIGHLIGHT_TEXT, LIGHT, SHADOW};
use crate::text::bgr_to_rgb;

// ------------------------------------------------------------ actions --

/// What the user did to a list, tree, grid, list view or header that the
/// program hears about with arguments or answers back: runtime-core does
/// it after the pump (`ui/kernel_lists.rs`), as gui.rs does for FLTK's
/// widgets — asking first (OnChanging …) and changing the model if the
/// answer allows.
#[derive(Clone, Debug, PartialEq)]
pub enum ListAction {
    /// `event` fired with `args` (OnColumnClick (Column), OnSectionTrack
    /// (Index, Width, State) …).
    Fire(String, Vec<Value>),
    /// A tree's node clicked: OnChanging (Index, AllowChange), then the
    /// selection and OnChange.
    TreeSelect(usize),
    /// A tree's node's button: OnExpanding / OnCollapsing (Index, Allow…),
    /// then OnExpanded / OnCollapsed.
    TreeToggle(usize, bool),
    /// F2 on a tree's node: OnEditing (Index, AllowEdit), then its editor.
    TreeEdit(usize),
    /// A tree's node's edit ended with Enter: OnEdited (Index, S), the node
    /// gets S.
    TreeEdited(usize, String),
    /// A grid's cell picked (`extend`: the range grows to it):
    /// OnSelectCell (Col, Row, CanSelect) may put it back.
    GridSelect(i64, i64, bool),
    /// A grid's selected cell edited (or picked from its drop-down list):
    /// stored, OnSetEditText (Col, Row, Value), OnChange.
    GridStore(String),
    /// A gcsList column's drop-down button on cell (Col, Row), at `Rect`
    /// (absolute): OnListDropDown (Col, Row, S) may change the items; then
    /// the list drops (`combo::open_list`).
    GridListDrop(i64, i64, Rect),
}

/// Queues `action` for the program (after the pump).
pub fn act(cx: &mut Cx, action: ListAction) {
    cx.events.push(KernelEvent::List(cx.id.to_string(), action));
}

/// `event` with `args`, for the program.
pub fn fire(cx: &mut Cx, event: &str, args: Vec<Value>) {
    act(cx, ListAction::Fire(event.to_string(), args));
}

// ------------------------------------------------- in-place editors --

/// A text being edited over a tree's node, a list view's caption or a
/// grid's cell (Windows' in-place edit control): what it edits and the
/// text so far. Enter keeps it, Escape drops it.
#[derive(Clone, Debug, PartialEq)]
pub struct InPlace {
    /// The node, item or (col, row) cell.
    pub target: (usize, usize),
    pub text: String,
    /// Where it shows in the component (`None`: where the component says).
    pub rect: Option<Rect>,
}

thread_local! {
    static EDITING: RefCell<HashMap<String, InPlace>> = RefCell::new(HashMap::new());
    static SCROLLS: RefCell<HashMap<String, Scroller>> = RefCell::new(HashMap::new());
    /// Each component's last click (item, when): a second one soon after on
    /// the same item is a double click.
    static CLICKS: RefCell<HashMap<String, (usize, std::time::Instant)>> = RefCell::new(HashMap::new());
}

/// Starts editing in component `id` (runtime-core, once OnEditing allowed
/// it; the component itself for a list view).
pub fn begin_edit(id: &str, edit: InPlace) {
    EDITING.with(|e| e.borrow_mut().insert(id.to_lowercase(), edit));
}

pub fn editing(id: &str) -> Option<InPlace> {
    EDITING.with(|e| e.borrow().get(&id.to_lowercase()).cloned())
}

/// Ends component `id`'s edit: what it was.
pub fn end_edit(id: &str) -> Option<InPlace> {
    EDITING.with(|e| e.borrow_mut().remove(&id.to_lowercase()))
}

/// The edit's text replaced (a test's `__enter` types "Renamed").
pub fn set_edit_text(id: &str, text: &str) {
    EDITING.with(|e| {
        if let Some(ed) = e.borrow_mut().get_mut(&id.to_lowercase()) {
            ed.text = text.to_string();
        }
    });
}

/// What a key does to component `id`'s edit: `Some(Some(keep))` when it
/// ends (Enter keeps, Escape drops), `Some(None)` when the edit took it,
/// `None` without an edit.
pub fn edit_key(id: &str, k: &KeyIn) -> Option<Option<bool>> {
    let mut ed = editing(id)?;
    match k.vk {
        13 => return Some(Some(true)),
        27 => return Some(Some(false)),
        8 => {
            ed.text.pop();
        }
        _ if !k.text.is_empty() && !k.mods.command && !k.mods.ctrl && !k.mods.alt && k.text.chars().all(|c| !c.is_control()) => ed.text.push_str(k.text),
        _ => return Some(None),
    }
    begin_edit(id, ed);
    Some(None)
}

/// Draws an edit box over `rect` (a white box, a black frame, the text and
/// its caret at the end).
pub fn paint_edit(p: &mut Painter, rect: Rect, text: &str, font: &Font, caret_on: bool) {
    let (x, y, w, h) = rect;
    p.fill(rect, 0xFFFFFF);
    p.edge(rect, &[0x000000], &[0x000000]);
    let inner = (x + 3, y + 1, (w - 6).max(0), (h - 2).max(0));
    p.clipped(inner, |p| {
        p.text(inner, text, font, bgr_to_rgb(font.color), Place::Left);
        if caret_on {
            let tw = rapidr_value::objects::text::text_size(text, font).0;
            let th = font.pixel_size().min(h - 2).max(1);
            p.fill((inner.0 + tw, y + (h - th) / 2, 1, th), 0x000000);
        }
    });
}

// ------------------------------------------------------- scrolling --

/// A vertical scroll bar for `content` pixels in an area `w` × `h` (the
/// shared Windows-classic bar): the position now (pixels scrolled), the
/// width left beside the bar, and its ops.
pub fn vscroll(id: &str, w: i64, h: i64, content: i64, step: i64) -> (i64, i64, Vec<Op>) {
    vscroll_at(id, w, h, content, step, None)
}

/// [`vscroll`] with the position the model says (a tree's TopIndex row),
/// when it has one.
pub fn vscroll_at(id: &str, w: i64, h: i64, content: i64, step: i64, at: Option<i64>) -> (i64, i64, Vec<Op>) {
    SCROLLS.with(|s| {
        let mut s = s.borrow_mut();
        let sc = s.entry(id.to_lowercase()).or_default();
        sc.horz.visible = false;
        sc.vert.increment = step.max(1);
        sc.vert.tracking = true;
        if let Some(at) = at {
            sc.vert.position = at.max(0);
        }
        let pos = sc.vert.position;
        sc.update(w, h, &[Child { left: 0, top: -pos, width: 0, height: content, align: rapidr_value::layout::Align::None, visible: true }]);
        let (cw, _) = sc.client(w, h);
        (sc.vert.position, cw, lift(sc.ops(w, h)))
    })
}

/// A horizontal scroll bar for `content` pixels across an area `w` × `h`
/// (a list box in Columns): the position, the height left above the bar,
/// and its ops.
pub fn hscroll(id: &str, w: i64, h: i64, content: i64, step: i64) -> (i64, i64, Vec<Op>) {
    SCROLLS.with(|s| {
        let mut s = s.borrow_mut();
        let sc = s.entry(id.to_lowercase()).or_default();
        sc.vert.visible = false;
        sc.horz.increment = step.max(1);
        sc.horz.tracking = true;
        let pos = sc.horz.position;
        sc.update(w, h, &[Child { left: -pos, top: 0, width: content, height: 0, align: rapidr_value::layout::Align::None, visible: true }]);
        let (_, ch) = sc.client(w, h);
        (sc.horz.position, ch, lift(sc.ops(w, h)))
    })
}

/// The mouse on component `id`'s scroll bar (in the area its bar is in):
/// whether the bar took it.
pub fn vscroll_mouse(id: &str, m: &MouseIn, w: i64, h: i64) -> bool {
    let (x, y) = (m.x.floor() as i64, m.y.floor() as i64);
    SCROLLS.with(|s| {
        let mut s = s.borrow_mut();
        let Some(sc) = s.get_mut(&id.to_lowercase()) else { return false };
        match m.kind {
            MouseKind::Down => sc.mouse_down(x, y, w, h).is_some(),
            MouseKind::Move if sc.pressed.is_some() => {
                sc.mouse_drag(x, y, w, h);
                true
            }
            MouseKind::Up if sc.pressed.is_some() => {
                sc.mouse_up(w, h);
                true
            }
            _ => false,
        }
    })
}

/// Scrolls component `id` so that `top .. bottom` (content pixels) shows in
/// a view `view` pixels tall.
pub fn scroll_into_view(id: &str, top: i64, bottom: i64, view: i64) {
    SCROLLS.with(|s| {
        let mut s = s.borrow_mut();
        let sc = s.entry(id.to_lowercase()).or_default();
        let pos = sc.vert.position;
        let to = if top < pos {
            top
        } else if bottom > pos + view {
            bottom - view
        } else {
            return;
        };
        sc.set("vertposition", &rapidr_value::v_int(to.max(0)));
    });
}

/// A second click on item `item` of `id` soon after the first: a double
/// click (Windows' 500 ms).
pub fn double_click(id: &str, item: usize) -> bool {
    let now = std::time::Instant::now();
    CLICKS.with(|c| {
        let mut c = c.borrow_mut();
        let key = id.to_lowercase();
        let dbl = c.get(&key).is_some_and(|&(i, t)| i == item && now.duration_since(t).as_millis() < 500);
        if dbl {
            c.remove(&key);
        } else {
            c.insert(key, (item, now));
        }
        dbl
    })
}

// ----------------------------------------------------------- drawing --

/// Windows' sunken client edge around a white box `w` × `h` (a list box,
/// a tree view, a grid).
pub fn sunken(p: &mut Painter, w: i64, h: i64, background: u32) {
    p.fill((0, 0, w, h), background);
    p.edge((0, 0, w, h), &[SHADOW, DARK], &[LIGHT, FACE]);
}

/// A component's Color, else white (a list's, a tree's background).
pub fn background(cx: &Cx) -> u32 {
    match cx.store.get(cx.id, "color") {
        v @ (Value::Integer(_) | Value::Double(_)) => bgr_to_rgb(v.to_i64()),
        _ => 0xFFFFFF,
    }
}

/// A bitmap's screen pixels (`Bitmap::display_rgba`) as a picture.
pub fn picture_of((w, h, rgba, _scale): (usize, usize, Vec<u8>, usize)) -> Picture {
    Picture { width: w, height: h, rgba }
}

/// What an owner-draw handler drew on a cell (OnDrawCell's ops, kept
/// relative to the cell's top left), drawn as ops from the current origin:
/// as FLTK's `grid_replay` draws them. `key` names the cell's pictures.
pub fn replay(p: &mut Painter, ops: &[CellDraw], font: &Font, key: &str) {
    let c = |v: u32| bgr_to_rgb(i64::from(v));
    let box_of = |x1: i64, y1: i64, x2: i64, y2: i64| (x1.min(x2), y1.min(y2), (x2 - x1).abs(), (y2 - y1).abs());
    for (k, op) in ops.iter().enumerate() {
        match op {
            CellDraw::Line(x1, y1, x2, y2, col) => p.line((*x1 as f64 + 0.5, *y1 as f64 + 0.5), (*x2 as f64 + 0.5, *y2 as f64 + 0.5), c(*col)),
            CellDraw::Rect(x1, y1, x2, y2, col) => {
                let r = box_of(*x1, *y1, *x2, *y2);
                if r.2 > 0 && r.3 > 0 {
                    p.edge(r, &[c(*col)], &[c(*col)]);
                }
            }
            CellDraw::Fill(x1, y1, x2, y2, col) => p.fill(box_of(*x1, *y1, *x2, *y2), c(*col)),
            CellDraw::Ellipse(x1, y1, x2, y2, col, fill) => {
                let (l, t, w, h) = box_of(*x1, *y1, *x2, *y2);
                let (cx, cy, rx, ry) = (l as f64 + w as f64 / 2.0, t as f64 + h as f64 / 2.0, w as f64 / 2.0, h as f64 / 2.0);
                let points = (0..48).map(|i| {
                    let a = i as f64 / 48.0 * std::f64::consts::TAU;
                    (cx + rx * a.cos(), cy + ry * a.sin())
                });
                p.shape(Shape { points: points.collect(), fill: fill.map(c), stroke: Some(c(*col)) });
            }
            CellDraw::Pixel(x, y, col) => p.fill((*x, *y, 1, 1), c(*col)),
            CellDraw::Text(x, y, text, col, bg) => {
                let (tw, th) = rapidr_value::objects::text::text_size(text, font);
                if let Some(bg) = bg {
                    p.fill((*x, *y, tw, th), c(*bg));
                }
                p.text((*x, *y, tw.max(1) + 2, th.max(1)), text, font, c(*col), Place::TopLeft);
            }
            CellDraw::Image(x, y, b) => {
                let (w, h) = (b.img.width as i64, b.img.height as i64);
                let pic = picture_of(b.clone().display_rgba());
                p.picture(&format!("{key}:{k}"), 0, pic, (*x, *y, w, h));
            }
        }
    }
}

// ------------------------------------------------------------ QLISTBOX --

/// Where a list box's items show: inside the frame, less the scroll bar
/// (the area FLTK's table gives them, which OnDrawItem's rects follow:
/// a vertical bar's room in one column, a horizontal one's in Columns).
pub fn view_size(l: &ItemList, w: i64, h: i64) -> (i64, i64) {
    if l.combo {
        (w - 8, 10_000)
    } else if l.multi_column() {
        (w - 4, h - 4 - 16)
    } else {
        (w - 4 - 16, h - 4)
    }
}

pub struct ListBox;

/// The item at (x, y) of the component (`None`: none there).
fn item_at(id: &str, x: f64, y: f64, w: i64, h: i64) -> Option<usize> {
    let (pos, _, _) = vscroll_state(id);
    with_list(id, |l| {
        let (x, y) = (x.floor() as i64 - 2, y.floor() as i64 - 2);
        if x < 0 || y < 0 || x >= w - 4 || y >= h - 4 {
            return None;
        }
        if l.multi_column() {
            l.item_at(x + hscroll_position(id), y)
        } else if l.custom_drawn() {
            l.item_at(x, y + pos)
        } else {
            let i = (y + pos) / l.row_height();
            usize::try_from(i).ok().filter(|&i| i < l.items.len())
        }
    })
    .flatten()
}

pub fn vscroll_state(id: &str) -> (i64, bool, i64) {
    SCROLLS.with(|s| s.borrow().get(&id.to_lowercase()).map_or((0, false, 0), |sc| (sc.vert.position, sc.vert.shown, sc.vert.range)))
}

/// How far component `id` is scrolled across.
pub fn hscroll_position(id: &str) -> i64 {
    SCROLLS.with(|s| s.borrow().get(&id.to_lowercase()).map_or(0, |sc| sc.horz.position))
}

/// Item `i`'s rectangle in the component (as scrolled now).
pub fn item_rect(id: &str, i: usize, w: i64, h: i64) -> Option<Rect> {
    let (pos, _, _) = vscroll_state(id);
    with_list(id, |l| {
        if l.custom_drawn() {
            let mut l = l.clone();
            let (vw, vh) = view_size(&l, w, h);
            l.set_view(vw, vh);
            let (x1, y1, x2, y2) = *l.item_rects().get(i)?;
            if l.multi_column() {
                let dx = hscroll_position(id);
                return Some((2 + x1 - dx, 2 + y1, x2 - x1, y2 - y1));
            }
            Some((2 + x1, 2 + y1 - pos, x2 - x1, y2 - y1))
        } else {
            let rh = l.row_height();
            (i < l.items.len()).then(|| (2, 2 + i as i64 * rh - pos, w - 4, rh))
        }
    })
    .flatten()
}

impl ListBox {
    /// The user picked item `i` (with Shift / Ctrl): selected as a click
    /// does, OnClick (a second click soon after: OnDblClick).
    fn pick(cx: &mut Cx, i: usize, shift: bool, ctrl: bool, mouse: bool) {
        with_list_mut(cx.id, |l| l.click(i as i64, shift, ctrl));
        let dbl = mouse && double_click(cx.id, i);
        if dbl {
            fire(cx, "ondblclick", Vec::new());
        } else {
            cx.click();
        }
    }
}

impl ComponentKind for ListBox {
    fn name(&self) -> &'static str {
        "RLISTBOX"
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        sunken(p, w, h, background(cx));
        let Some(mut l) = with_list(cx.id, |l| l.clone()) else { return };
        let font = cx.font.clone();
        let text_color = if cx.state.enabled { bgr_to_rgb(font.color) } else { SHADOW };
        let (iw, ih) = (w - 4, h - 4);
        if l.custom_drawn() {
            let (vw, vh) = view_size(&l, w, h);
            l.set_view(vw, vh);
            with_list_mut(cx.id, |m| m.set_view(vw, vh));
            let rects = l.item_rects();
            // (one column scrolls down; Columns scroll across)
            let (dx, dy, cw, ch, bar) = if l.multi_column() {
                let content = rects.iter().map(|r| r.2).max().unwrap_or(0);
                let (pos, ch, bar) = hscroll(cx.id, iw, ih, content, l.column_layout().1);
                (pos, 0, iw, ch, bar)
            } else {
                let content = rects.iter().map(|r| r.3).max().unwrap_or(0);
                let (pos, cw, bar) = vscroll(cx.id, iw, ih, content, l.row_height());
                (0, pos, cw, ih, bar)
            };
            p.at((2, 2), |p| {
                p.clipped((0, 0, cw, ch), |p| {
                    for (i, &(x1, y1, x2, y2)) in rects.iter().enumerate() {
                        let (x1, y1, x2, y2) = (x1 - dx, y1 - dy, x2 - dx, y2 - dy);
                        if y2 <= 0 || y1 >= ch || x2 <= 0 || x1 >= cw {
                            continue;
                        }
                        let font = font.clone();
                        let shown = l.render_item(i, x2 - x1, &font).display_rgba();
                        p.picture(&format!("{}#item{i}", cx.id), 0, picture_of(shown), (x1, y1, x2 - x1, y2 - y1));
                        if cx.state.focused && l.item_index == i as i64 {
                            p.focus((x1, y1, x2 - x1, y2 - y1));
                        }
                    }
                });
                p.ops(bar);
            });
            return;
        }
        let rh = l.row_height();
        let (pos, cw, bar) = vscroll(cx.id, iw, ih, l.items.len() as i64 * rh, rh);
        p.at((2, 2), |p| {
            p.clipped((0, 0, cw, ih), |p| {
                let first = (pos / rh).max(0) as usize;
                for i in first..l.items.len() {
                    let top = i as i64 * rh - pos;
                    if top >= ih {
                        break;
                    }
                    let selected = l.is_selected(i);
                    if selected {
                        p.fill((0, top, cw, rh), HIGHLIGHT);
                    }
                    let text = l.items[i].replace(['\n', '\r', '\t'], " ");
                    p.text((2, top, cw - 2, rh), &text, &font, if selected { HIGHLIGHT_TEXT } else { text_color }, Place::Left);
                    if cx.state.focused && l.item_index == i as i64 {
                        p.focus((0, top, cw, rh));
                    }
                }
            });
            p.ops(bar);
        });
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let (w, h) = (cx.width(), cx.height());
        let inner = MouseIn { x: m.x - 2.0, y: m.y - 2.0, ..*m };
        if vscroll_mouse(cx.id, &inner, w - 4, h - 4) {
            return MouseOut::default();
        }
        if m.kind == MouseKind::Down {
            if let Some(i) = item_at(cx.id, m.x, m.y, w, h) {
                Self::pick(cx, i, m.mods.shift, m.mods.ctrl || m.mods.command, true);
            }
        }
        MouseOut::default()
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, _clip: &mut dyn Clipboard) -> bool {
        if k.mods.alt || k.mods.command {
            return false;
        }
        let (w, h) = (cx.width(), cx.height());
        let Some((count, current, rh, multi, per)) = with_list(cx.id, |l| {
            let mut l2 = l.clone();
            let (vw, vh) = view_size(l, w, h);
            l2.set_view(vw, vh);
            (l.items.len() as i64, l.item_index, l.row_height(), l.multi_column(), l2.column_layout().0)
        }) else {
            return false;
        };
        let page = if multi { per } else { ((h - 4) / rh.max(1)).max(1) };
        let next = match k.vk {
            40 => current + 1,
            38 => (current - 1).max(0),
            39 if multi => current + per,
            37 if multi => (current - per).max(0),
            34 => current + page,
            33 => (current - page).max(0),
            36 => 0,
            35 => count - 1,
            _ => return false,
        };
        let next = next.clamp(0, (count - 1).max(0));
        if count > 0 && next != current {
            Self::pick(cx, next as usize, k.mods.shift, false, false);
            if let Some((_, y, _, rh)) = item_rect(cx.id, next as usize, w, h) {
                let (pos, _, _) = vscroll_state(cx.id);
                scroll_into_view(cx.id, y - 2 + pos, y - 2 + pos + rh, h - 4);
            }
        }
        true
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = AccessNode::new(node_id(cx.id), Role::ListBox);
        n.actions = vec![Action::Focus];
        n.bounds = cx.rect;
        let (w, h) = (cx.width(), cx.height());
        let items = with_list(cx.id, |l| l.items.iter().enumerate().map(|(i, t)| (i, t.clone(), l.is_selected(i))).collect::<Vec<_>>()).unwrap_or_default();
        for (i, text, selected) in items.into_iter().take(1_000) {
            let mut o = AccessNode::new(part_id(cx.id, 1, i), Role::ListBoxOption);
            o.name = text;
            o.states.selected = Some(selected);
            o.actions = vec![Action::Click];
            if let Some((x, y, iw, ih)) = item_rect(cx.id, i, w, h) {
                o.bounds = (cx.rect.0 + x, cx.rect.1 + y, iw, ih);
            }
            n.children.push(o);
        }
        n
    }

    fn access(&self, cx: &mut Cx, action: Action, part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        match (action, part) {
            (Action::Click, Some(i)) if cx.state.enabled => {
                Self::pick(cx, i, false, false, false);
                true
            }
            _ => false,
        }
    }

    /// `__item_i`: a click on item `i` (its row brought into view first).
    fn test_action(&self, cx: &mut Cx, action: &str) -> bool {
        let Some(i) = action.strip_prefix("__item_").and_then(|s| s.parse::<usize>().ok()) else { return false };
        let (w, h) = (cx.width(), cx.height());
        let Some((_, y, _, rh)) = item_rect(cx.id, i, w, h) else { return true };
        let (pos, _, _) = vscroll_state(cx.id);
        scroll_into_view(cx.id, y - 2 + pos, y - 2 + pos + rh, h - 4);
        // (the bar's position moves at the next paint: the item picked
        // directly when it isn't shown whole yet)
        let mid = |r: &Rect| r.0 + r.2.min(w - 20) / 2;
        match item_rect(cx.id, i, w, h).filter(|r| r.1 >= 2 && r.1 + r.3 <= h - 2 && mid(r) >= 2 && mid(r) < w - 2) {
            Some(r @ (_, y, _, ih)) => {
                let at = MouseIn { kind: MouseKind::Down, x: mid(&r) as f64 + 0.5, y: (y + ih / 2) as f64 + 0.5, button: rapidr_value::input::Button::Left, mods: crate::input::Mods::NONE, inside: true, captured: true };
                // (a test's click is never a double click)
                CLICKS.with(|c| c.borrow_mut().remove(cx.id));
                self.mouse(cx, &at);
                self.mouse(cx, &MouseIn { kind: MouseKind::Up, ..at });
            }
            None => Self::pick(cx, i, false, false, false),
        }
        true
    }
}

#[cfg(test)]
mod tests {
    //! The lists lane's components headless: a form in a [`MemStore`],
    //! input in, events and display lists out.

    use rapidr_value::input::Button;
    use rapidr_value::objects::{with_list, with_tree};
    use rapidr_value::{v_int, v_str};

    use super::ListAction;
    use crate::display::Item;
    use crate::{FormUi, KernelEvent, MemStore, Mods, Op, TextSystem};

    fn form(build: impl FnOnce(&mut MemStore)) -> (MemStore, FormUi, TextSystem) {
        let mut s = MemStore::new();
        s.add("f", "RFORM", None);
        build(&mut s);
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(&s, "f", false);
        drop(f.paint(&s, &mut ts, 1.0));
        (s, f, ts)
    }

    fn click(f: &mut FormUi, s: &MemStore, ts: &mut TextSystem, x: f64, y: f64) -> Vec<KernelEvent> {
        f.mouse_down(s, ts, x, y, Button::Left, Mods::NONE);
        f.mouse_up(s, ts, x, y, Button::Left, Mods::NONE);
        f.take_events().into_iter().filter(|e| !matches!(e, KernelEvent::Mouse { .. })).collect()
    }

    #[test]
    fn list_box_click_keys_and_test_item() {
        let (s, mut f, mut ts) = form(|s| {
            s.add("lst", "RLISTBOX", Some("f")).set("lst", "left", v_int(10)).set("lst", "top", v_int(10));
            s.call("lst", "additems", &[v_str("a"), v_str("b"), v_str("c")]);
        });
        // (row 1 is 16 pixels down, inside the 2-pixel frame)
        assert_eq!(click(&mut f, &s, &mut ts, 30.0, 12.0 + 16.0 + 4.0), vec![KernelEvent::Click("lst".into())]);
        assert_eq!(with_list("lst", |l| l.item_index), Some(1));
        f.key_down(&s, &mut ts, 40, "", Mods::NONE, &mut crate::MemClipboard::default());
        assert_eq!(with_list("lst", |l| l.item_index), Some(2));
        assert!(f.take_events().contains(&KernelEvent::Click("lst".into())));
        assert!(f.test_action(&s, &mut ts, "lst", "__item_0"));
        assert_eq!(with_list("lst", |l| l.item_index), Some(0));
        // (the selected row white on blue)
        let list = f.paint(&s, &mut ts, 1.0);
        assert!(list.items.iter().any(|i| matches!(i, Item::Op { op: Op::Fill { color: crate::paint::HIGHLIGHT, .. }, .. })));
    }

    #[test]
    fn combo_drops_its_list_and_picks() {
        let (s, mut f, mut ts) = form(|s| {
            s.add("cb", "RCOMBOBOX", Some("f")).set("cb", "left", v_int(10)).set("cb", "top", v_int(10));
            s.call("cb", "additems", &[v_str("red"), v_str("green"), v_str("blue")]);
        });
        // (a click on the box drops the list under it; a click on a row picks)
        click(&mut f, &s, &mut ts, 20.0, 20.0);
        assert!(super::super::combo::is_dropped("cb"));
        let events = click(&mut f, &s, &mut ts, 30.0, 35.0 + 16.0 + 8.0);
        assert!(!super::super::combo::is_dropped("cb"));
        assert_eq!(events, vec![KernelEvent::Change("cb".into())]);
        assert_eq!(with_list("cb", |l| (l.item_index, l.text.clone())), Some((1, "green".into())));
        assert!(f.test_action(&s, &mut ts, "cb", "__item_2"));
        assert_eq!(f.take_events(), vec![KernelEvent::Change("cb".into())]);
        assert_eq!(with_list("cb", |l| l.item_index), Some(2));
    }

    #[test]
    fn tree_clicks_ask_the_program() {
        let (s, mut f, mut ts) = form(|s| {
            s.add("tv", "RTREEVIEW", Some("f")).set("tv", "width", v_int(200)).set("tv", "height", v_int(200));
            s.call("tv", "additems", &[v_str("1"), v_str("2")]);
            s.call("tv", "addchilditems", &[v_int(0), v_str("a")]);
        });
        // (node 0's button: OnClick, then OnExpanding's question)
        let events = click(&mut f, &s, &mut ts, 2.0 + 9.0, 2.0 + 9.0);
        assert_eq!(events, vec![KernelEvent::Click("tv".into()), KernelEvent::List("tv".into(), ListAction::TreeToggle(0, true))]);
        // (a node's text: OnChanging's question, then OnClick)
        let events = click(&mut f, &s, &mut ts, 2.0 + 30.0, 2.0 + 18.0 + 9.0);
        assert_eq!(events, vec![KernelEvent::List("tv".into(), ListAction::TreeSelect(2)), KernelEvent::Click("tv".into())]);
        // (F2 asks OnEditing for the selected node)
        with_tree("tv", |t| t.select(2));
        f.test_action(&s, &mut ts, "tv", "__edit");
        assert_eq!(f.take_events(), vec![KernelEvent::List("tv".into(), ListAction::TreeEdit(2))]);
        // (the editor, once allowed: "Renamed" and Enter ask OnEdited)
        super::super::tree::open_editor("tv", 2, "2");
        f.test_action(&s, &mut ts, "tv", "__enter");
        assert_eq!(f.take_events(), vec![KernelEvent::List("tv".into(), ListAction::TreeEdited(2, "Renamed".into()))]);
        assert!(super::editing("tv").is_none());
    }

    #[test]
    fn grid_cells_and_owner_drawing() {
        let (s, mut f, mut ts) = form(|s| {
            s.add("g", "RSTRINGGRID", Some("f")).set("g", "width", v_int(340)).set("g", "height", v_int(140));
        });
        // (cell (2, 1): 2 + 64 + 1 + 64 + 1 across, 2 + 24 + 1 down)
        let events = click(&mut f, &s, &mut ts, 2.0 + 130.0 + 10.0, 2.0 + 25.0 + 10.0);
        assert_eq!(events, vec![KernelEvent::List("g".into(), ListAction::GridSelect(2, 1, false)), KernelEvent::Click("g".into())]);
        assert!(f.test_action(&s, &mut ts, "g", "__cell_3_2"));
        assert_eq!(f.take_events(), vec![KernelEvent::List("g".into(), ListAction::GridSelect(3, 2, false))]);
        // (what OnDrawCell drew, replayed over the cell)
        let reader = |_: &str, _: &str| rapidr_value::Value::Null;
        rapidr_value::objects::call("g", "fillrect", &[v_int(140), v_int(30), v_int(150), v_int(40), v_int(0xFF)], &reader);
        let list = f.paint(&s, &mut ts, 1.0);
        assert!(list.items.iter().any(|i| matches!(i, Item::Op { op: Op::Fill { color: 0xFF0000, rect: (10, 5, 10, 10) }, .. })));
    }
}
