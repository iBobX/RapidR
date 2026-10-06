//! `Align` (RapidQ manual, Appendix A: "Align determines how the control
//! aligns within its parent control"): RapidQ's components are Delphi VCL
//! controls, so aligned children are laid out as Delphi's
//! `TWinControl.AlignControls` does. Both runtimes call [`align_controls`]
//! with a container's client rectangle and its children, and move the
//! children to the rectangles it returns.
//!
//! * The aligns are handled in the order alTop, alBottom, alLeft, alRight,
//!   alClient. Each control takes a strip off the edge of what's left of the
//!   client area — keeping its height (alTop / alBottom) or width (alLeft /
//!   alRight) — and alClient controls fill what remains.
//! * Controls with the same align are ordered by position (for alTop, the
//!   one nearer the top first; for alBottom, the one nearer the bottom
//!   first; likewise left / right). Among equal positions, creation order
//!   for alTop / alLeft and the reverse for alBottom / alRight (Delphi
//!   compares with `<` and `>=`). The control whose Align, size or
//!   visibility just changed comes first. So `Splitter (alLeft)` created before `Tree (alLeft)` ends
//!   up to the right of the tree, as in RapidQ.
//! * Invisible controls and those with alNone are left alone.
//!
//! RapidR adds Delphi's `Anchors` and `Constraints` (RapidQ had Align
//! only; with their defaults nothing changes): [`AnchorRules`] keeps an
//! unaligned control at its distances from its parent's edges when the
//! parent's client area resizes, and [`Constraints`] bound every size a
//! component takes — set by the program, by Align or Anchors, or by the
//! user resizing a form.

/// RAPIDQ.INC: `alNone = 0`, `alTop = 1`, `alBottom = 2`, `alLeft = 3`,
/// `alRight = 4`, `alClient = 5`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    None,
    Top,
    Bottom,
    Left,
    Right,
    Client,
}

impl Align {
    pub fn from_value(n: i64) -> Align {
        match n {
            1 => Align::Top,
            2 => Align::Bottom,
            3 => Align::Left,
            4 => Align::Right,
            5 => Align::Client,
            _ => Align::None,
        }
    }

    pub fn value(self) -> i64 {
        self as i64
    }
}

/// A component's geometry, as stored: its Left / Top / Width / Height
/// (and a form's ClientWidth / ClientHeight) are integers, as in RapidQ
/// (Delphi's Integer properties), so `Height = ClientHeight / 2 + 20` keeps
/// an integer — rounded half to even, as the FPU stores it (304.5 → 304).
/// A check box's Checked is true or false whatever number is stored (RC.EXE:
/// `Checked = 5` reads 1).
pub fn property_value(prop: &str, val: crate::Value) -> crate::Value {
    match val {
        crate::Value::Double(f) if matches!(prop.to_ascii_lowercase().as_str(), "left" | "top" | "width" | "height" | "clientwidth" | "clientheight") => {
            crate::Value::Integer(crate::numeric::round_to_int(f))
        }
        crate::Value::Integer(_) | crate::Value::Double(_) if prop.eq_ignore_ascii_case("checked") => crate::Value::Boolean(val.to_bool()),
        v => v,
    }
}

/// (the input lane's) A QSTATUSBAR's size grip: a square this big at its
/// bottom-right (Windows' SM_CXVSCROLL).
pub const STATUS_GRIP: i64 = 16;

/// (the input lane's) Whether a QSTATUSBAR shows its size grip, as Delphi's
/// TStatusBar.SizeGrip (RapidQ's SizeGrip, True by default): on a form
/// (`on_form`: the form is its parent) that is sizeable (BorderStyle
/// bsSizeable 2 or bsSizeToolWin 5), docked at its bottom.
pub fn status_grip(size_grip: bool, on_form: bool, border_style: i64, align: Align) -> bool {
    size_grip && on_form && matches!(border_style, 2 | 5) && align == Align::Bottom
}

/// The Align a component type starts with (QSTATUSBAR docks at the bottom,
/// QHEADER at the top — Delphi's THeaderControl; RC.EXE reads `Align` 1 and
/// a header created at Left 10, Top 10, Width 300 shows across the whole
/// top of the form — QSPLITTER at the left, as in RapidQ), for RapidR's
/// type names.
pub fn default_align(type_name: &str) -> Align {
    match type_name.to_ascii_uppercase().as_str() {
        "RSTATUSBAR" => Align::Bottom,
        "RHEADER" => Align::Top,
        "RSPLITTER" => Align::Left,
        _ => Align::None,
    }
}

/// The Width × Height a component starts with, the same on every runtime
/// (RapidR's type names). RapidQ's components get RapidQ's sizes, as RC.EXE
/// reads them on a component created without one (Delphi's defaults; a
/// QCOMBOBOX is the height of its edit box, its list not counted; a QLABEL
/// then sizes itself to its caption — AutoSize); RapidR's own components,
/// theirs.
pub fn default_size(type_name: &str) -> Option<(i64, i64)> {
    Some(match type_name.to_ascii_uppercase().as_str() {
        "RFORM" => (320, 240),
        "RBUTTON" => (75, 25),
        "RLABEL" => (65, 17),
        "REDIT" => (121, 21),
        "RRICHEDIT" | "RMEMO" => (185, 89),
        "RCOOLBTN" => (25, 25),
        "ROVALBTN" => (100, 50),
        "RCHECKBOX" => (97, 17),
        "RRADIOBUTTON" => (113, 17),
        "RSCROLLBAR" => (121, 17),
        "RPANEL" | "RBEVEL" | "RSCROLLBOX" => (185, 41),
        "RGROUPBOX" => (185, 105),
        "RTABCONTROL" => (289, 193),
        "RSTRINGGRID" => (320, 120),
        "RLISTVIEW" => (250, 150),
        // (QDigDisplay.inc: one 12 × 24 cell, its Display "0")
        "RDIGDISPLAY" => (12, 24),
        // (RC.EXE's QGLASSFRAME)
        "RGLASSFRAME" => (105, 105),
        // (I1: RapidR Studio's docking)
        "RDOCKMANAGER" => (400, 300),
        "RCOMBOBOX" => (145, 25),
        "RLISTBOX" | "RTREEVIEW" | "RDIRTREE" => (121, 97),
        "RFILELISTBOX" => (145, 97),
        "RTRACKBAR" => (150, 45),
        "RCANVAS" | "RIMAGE" => (105, 105),
        "RDXSCREEN" => (100, 100),
        // (QGAUGE)
        "RPROGRESSBAR" => (100, 100),
        // (docked: the width is the parent's)
        "RHEADER" => (320, 17),
        "RSTATUSBAR" => (320, 19),
        "RSPLITTER" => (3, 200),
        // RapidR's own
        "RPROGRESS" => (200, 25),
        "RCODEEDITOR" | "RWEBVIEW" => (400, 300),
        "RDIFFVIEW" => (500, 300),
        "RDESIGNSURFACE" => (640, 480),
        "RPLOT" => (600, 400),
        // (the web's own elements: the DOM runtime's 100 × 25)
        "RDOM" | "RWEBAUDIO" | "RWEBVIDEO" => (100, 25),
        _ => return None,
    })
}

/// A form's frame, the same in both runtimes. As in RapidQ (Delphi),
/// `Width` / `Height` are the whole window — caption and borders included —
/// and `ClientWidth` / `ClientHeight` the area inside, below the main menu:
/// a 29px caption, a 1px border on every side, and no frame at all with
/// `BorderStyle = bsNone` (0). The desktop's window manager draws its own
/// frame, so there the frame is only accounted for, keeping a form's inside
/// the size it has in the browser.
pub const FORM_CAPTION: i64 = 29;
pub const FORM_BORDER: i64 = 1;
/// A form's main menu bar inside its window, above the client area (the
/// web's, and the desktop's off macOS — or on it with `RAPIDR_MENU=window`;
/// a macOS menu is otherwise the system's, at the top of the screen).
pub const MAIN_MENU_HEIGHT: i64 = 28;

/// The frame around a form's inside: (left + right, caption + top +
/// bottom), for its BorderStyle.
pub fn form_frame(border_style: i64) -> (i64, i64) {
    if border_style == 0 {
        (0, 0)
    } else {
        (2 * FORM_BORDER, FORM_CAPTION + 2 * FORM_BORDER)
    }
}

/// A form's client size for its Width / Height, BorderStyle and the height
/// of its main menu.
pub fn form_client_size(width: i64, height: i64, border_style: i64, menu: i64) -> (i64, i64) {
    let (fw, fh) = form_frame(border_style);
    ((width - fw).max(0), (height - fh - menu).max(0))
}

/// The Width / Height that give a form this client size (ClientWidth =,
/// ClientHeight =).
pub fn form_outer_size(client_width: i64, client_height: i64, border_style: i64, menu: i64) -> (i64, i64) {
    let (fw, fh) = form_frame(border_style);
    (client_width.max(0) + fw, client_height.max(0) + fh + menu)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Rect {
    pub left: i64,
    pub top: i64,
    pub width: i64,
    pub height: i64,
}

impl Rect {
    pub fn new(left: i64, top: i64, width: i64, height: i64) -> Rect {
        Rect { left, top, width, height }
    }

    fn right(&self) -> i64 {
        self.left + self.width
    }

    fn bottom(&self) -> i64 {
        self.top + self.height
    }
}

/// A child of the container being laid out.
#[derive(Clone, Copy, Debug)]
pub struct Control {
    pub align: Align,
    pub visible: bool,
    pub rect: Rect,
    /// Its MinWidth … MaxHeight: an aligned control never takes a size
    /// outside them (the strip it takes off the client area is the size it
    /// got, as in Delphi's AlignControls).
    pub constraints: Constraints,
}

/// Lays out `controls` (in creation order) inside `client` (in the
/// children's coordinates); `changed` is the index of the control whose
/// Align, size or visibility just changed, if any. Returns the new
/// rectangle of every aligned, visible control, as `(index, rect)`.
pub fn align_controls(client: Rect, controls: &[Control], changed: Option<usize>) -> Vec<(usize, Rect)> {
    // What's left of the client area, as edges.
    let (mut left, mut top, mut right, mut bottom) = (client.left, client.top, client.right(), client.bottom());
    let mut out = Vec::new();
    for align in [Align::Top, Align::Bottom, Align::Left, Align::Right, Align::Client] {
        let takes_part = |i: usize| controls[i].align == align && controls[i].visible;
        let mut order: Vec<usize> = Vec::new();
        if let Some(c) = changed.filter(|&c| c < controls.len() && takes_part(c)) {
            order.push(c);
        }
        for i in (0..controls.len()).filter(|&i| takes_part(i) && Some(i) != changed) {
            let r = controls[i].rect;
            let at = order
                .iter()
                .position(|&j| {
                    let o = controls[j].rect;
                    match align {
                        Align::Top => r.top < o.top,
                        Align::Bottom => r.bottom() >= o.bottom(),
                        Align::Left => r.left < o.left,
                        Align::Right => r.right() >= o.right(),
                        _ => false,
                    }
                })
                .unwrap_or(order.len());
            order.insert(at, i);
        }
        for i in order {
            let r = controls[i].rect;
            let k = controls[i].constraints;
            // (the size along the edge it keeps; the other is what's left)
            let (w, h) = (k.width(r.width.max(0)), k.height(r.height.max(0)));
            let placed = match align {
                Align::Top => {
                    let p = Rect::new(left, top, right - left, h);
                    top += h;
                    p
                }
                Align::Bottom => {
                    bottom -= h;
                    Rect::new(left, bottom, right - left, h)
                }
                Align::Left => {
                    let p = Rect::new(left, top, w, bottom - top);
                    left += w;
                    p
                }
                Align::Right => {
                    right -= w;
                    Rect::new(right, top, w, bottom - top)
                }
                _ => Rect::new(left, top, right - left, bottom - top),
            };
            // Never a negative size when the client area runs out; never
            // one outside its constraints.
            out.push((i, Rect { width: k.width(placed.width.max(0)), height: k.height(placed.height.max(0)), ..placed }));
        }
    }
    out
}

/// `Anchors` (Delphi's TAnchors) as RapidR's integer bit set: akLeft = 1,
/// akTop = 2, akRight = 4, akBottom = 8 (the constants are RapidR's own,
/// `rapidr_ast::rapidr_constant`). A component starts anchored left and
/// top: it stays where it is when its parent resizes, as in RapidQ.
pub const AK_LEFT: i64 = 1;
pub const AK_TOP: i64 = 2;
pub const AK_RIGHT: i64 = 4;
pub const AK_BOTTOM: i64 = 8;
pub const DEFAULT_ANCHORS: i64 = AK_LEFT | AK_TOP;

/// What an anchored control remembers of its place (Delphi's FAnchorRules
/// and FOriginalParentSize), taken when its Anchors are set and whenever
/// the program places it (Left / Top / Width / Height, Parent) — not when
/// anchoring itself moves it. Per axis: anchored to both edges, its size
/// (it stretches); to the far edge only, its position (it moves); to
/// neither, its centre (it keeps its place proportionally).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnchorRules {
    anchors: i64,
    x: i64,
    y: i64,
    parent: (i64, i64),
}

impl AnchorRules {
    /// The rules of a control at `rect` with `anchors` in a parent whose
    /// client area is `parent` (width, height); `None` with the default
    /// anchors, or in a parent of no size yet (nothing to follow, as in
    /// Delphi).
    pub fn new(anchors: i64, rect: Rect, parent: (i64, i64)) -> Option<AnchorRules> {
        if anchors & 15 == DEFAULT_ANCHORS || parent.0 == 0 || parent.1 == 0 {
            return None;
        }
        let rule = |lo: bool, hi: bool, pos: i64, size: i64| match (lo, hi) {
            (true, true) => size,
            (false, true) => pos,
            (false, false) => pos + size / 2,
            (true, false) => pos,
        };
        Some(AnchorRules {
            anchors,
            x: rule(anchors & AK_LEFT != 0, anchors & AK_RIGHT != 0, rect.left, rect.width),
            y: rule(anchors & AK_TOP != 0, anchors & AK_BOTTOM != 0, rect.top, rect.height),
            parent,
        })
    }

    /// The Anchors they were taken with (a component whose Anchors are
    /// no longer these doesn't follow them).
    pub fn anchors(&self) -> i64 {
        self.anchors
    }

    /// Where the control (now at `rect`) goes when its parent's client area
    /// is `parent`: Delphi's AlignControls for an unaligned control.
    pub fn place(&self, rect: Rect, parent: (i64, i64)) -> Rect {
        let a = self.anchors;
        let (left, width) = place_axis(a & AK_LEFT != 0, a & AK_RIGHT != 0, self.x, self.parent.0, parent.0, rect.left, rect.width);
        let (top, height) = place_axis(a & AK_TOP != 0, a & AK_BOTTOM != 0, self.y, self.parent.1, parent.1, rect.top, rect.height);
        Rect::new(left, top, width.max(0), height.max(0))
    }
}

/// One axis of [`AnchorRules::place`]: the new position and size.
fn place_axis(lo: bool, hi: bool, rule: i64, was: i64, now: i64, pos: i64, size: i64) -> (i64, i64) {
    match (lo, hi) {
        (true, true) => (pos, now - (was - rule)),
        (false, true) => (now - (was - rule), size),
        (false, false) => (mul_div(rule, now, was) - size / 2, size),
        (true, false) => (pos, size),
    }
}

/// Windows' MulDiv: `a * b / c` rounded to the nearest (halves away from
/// zero), -1 when `c` is 0.
fn mul_div(a: i64, b: i64, c: i64) -> i64 {
    if c == 0 {
        return -1;
    }
    let (p, c) = (a as i128 * b as i128, c as i128);
    let q = (p.abs() + c.abs() / 2) / c.abs();
    (if (p < 0) != (c < 0) { -q } else { q }) as i64
}

/// The anchoring of the components (by name, any case), the same store in
/// both runtimes: set by [`anchor_record`], read by [`anchor_rules`].
mod anchor_store {
    use std::cell::RefCell;
    use std::collections::HashMap;
    thread_local! {
        pub static RULES: RefCell<HashMap<String, super::AnchorRules>> = RefCell::new(HashMap::new());
    }
}

/// Records (or, with the default anchors, forgets) where component `name`
/// is: see [`AnchorRules::new`].
pub fn anchor_record(name: &str, anchors: i64, rect: Rect, parent: (i64, i64)) {
    let key = name.to_lowercase();
    anchor_store::RULES.with(|r| match AnchorRules::new(anchors, rect, parent) {
        Some(rules) => {
            r.borrow_mut().insert(key, rules);
        }
        None => {
            r.borrow_mut().remove(&key);
        }
    });
}

/// The anchoring recorded for component `name`, if it follows its parent.
pub fn anchor_rules(name: &str) -> Option<AnchorRules> {
    anchor_store::RULES.with(|r| r.borrow().get(&name.to_lowercase()).copied())
}

/// Lays out the anchored children of a container whose client area is now
/// `parent` (width, height): each `(rules, rect, align, constraints)`; the
/// aligned ones are Align's (Delphi: Align wins). Returns the new rectangle
/// of each child that moves or resizes, as `(index, rect)`.
pub fn anchor_controls(parent: (i64, i64), children: &[(Option<AnchorRules>, Rect, Align, Constraints)]) -> Vec<(usize, Rect)> {
    children
        .iter()
        .enumerate()
        .filter_map(|(i, (rules, rect, align, k))| {
            let rules = rules.filter(|_| *align == Align::None)?;
            let mut r = rules.place(*rect, parent);
            (r.width, r.height) = k.size(r.width, r.height);
            (r != *rect).then_some((i, r))
        })
        .collect()
}

/// `Constraints` (Delphi's TSizeConstraints): MinWidth, MinHeight,
/// MaxWidth, MaxHeight; 0 is no limit. RapidR keeps them as flat
/// properties (`MinWidth`), `Constraints.MinWidth` being the same one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Constraints {
    pub min_width: i64,
    pub min_height: i64,
    pub max_width: i64,
    pub max_height: i64,
}

/// The constraint properties, flat, as stored.
pub const CONSTRAINT_PROPERTIES: [&str; 4] = ["minwidth", "minheight", "maxwidth", "maxheight"];

impl Constraints {
    /// A component's, from its stored properties (`get("minwidth")`, …).
    pub fn of(get: impl Fn(&str) -> i64) -> Constraints {
        Constraints { min_width: get("minwidth"), min_height: get("minheight"), max_width: get("maxwidth"), max_height: get("maxheight") }
    }

    pub fn is_none(&self) -> bool {
        *self == Constraints::default()
    }

    /// A width within them (Delphi: the minimum first, then the maximum —
    /// with both set the minimum wins, which the setters keep below the
    /// maximum anyway).
    pub fn width(&self, w: i64) -> i64 {
        bound(w, self.min_width, self.max_width)
    }

    pub fn height(&self, h: i64) -> i64 {
        bound(h, self.min_height, self.max_height)
    }

    pub fn size(&self, w: i64, h: i64) -> (i64, i64) {
        (self.width(w), self.height(h))
    }

    /// The constraints once `prop` (flat, lowercase) is set to `value`, as
    /// Delphi's setters do: a minimum above the maximum raises the maximum,
    /// a maximum below the minimum lowers the minimum; negative is 0.
    /// `None` when `prop` isn't one of them.
    pub fn with(mut self, prop: &str, value: i64) -> Option<Constraints> {
        let v = value.max(0);
        match prop {
            "minwidth" => {
                self.min_width = v;
                if self.max_width > 0 && v > self.max_width {
                    self.max_width = v;
                }
            }
            "minheight" => {
                self.min_height = v;
                if self.max_height > 0 && v > self.max_height {
                    self.max_height = v;
                }
            }
            "maxwidth" => {
                self.max_width = v;
                if v > 0 && v < self.min_width {
                    self.min_width = v;
                }
            }
            "maxheight" => {
                self.max_height = v;
                if v > 0 && v < self.min_height {
                    self.min_height = v;
                }
            }
            _ => return None,
        }
        Some(self)
    }

    /// `(prop, value)` for each of the four (to store them all).
    pub fn properties(&self) -> [(&'static str, i64); 4] {
        [("minwidth", self.min_width), ("minheight", self.min_height), ("maxwidth", self.max_width), ("maxheight", self.max_height)]
    }
}

fn bound(v: i64, min: i64, max: i64) -> i64 {
    if min > 0 && v < min {
        min
    } else if max > 0 && v > max {
        max
    } else {
        v
    }
}

/// `Constraints.MinWidth` … (lowercase): the flat property it is.
pub fn constraint_alias(prop: &str) -> Option<&'static str> {
    let flat = prop.strip_prefix("constraints.")?;
    CONSTRAINT_PROPERTIES.iter().find(|p| **p == flat).copied()
}

/// What a visual component's RapidR layout properties read before the
/// program sets them: Anchors akLeft + akTop, no constraints.
pub fn default_property(type_name: &str, prop: &str) -> Option<i64> {
    default_size(type_name)?;
    match prop {
        "anchors" => Some(DEFAULT_ANCHORS),
        p if CONSTRAINT_PROPERTIES.contains(&p) || constraint_alias(p).is_some() => Some(0),
        _ => None,
    }
}

/// Dragging a QSPLITTER (Delphi's TSplitter, which RapidQ wraps): the
/// control just outside the splitter's anchored edge — left of an alLeft
/// splitter, above an alTop one, … — gets wider / taller as the splitter
/// moves away from that edge, never below `MinSize` nor leaving less than
/// `MinSize` for the rest of the client area.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SplitterDrag {
    /// Index (in the controls given to [`splitter_drag`]) of the control
    /// being resized.
    pub control: usize,
    /// True when it's the control's Width that changes (alLeft / alRight).
    pub horizontal: bool,
    start: i64,
    /// +1 when moving right / down grows the control (alLeft / alTop).
    sign: i64,
    min: i64,
    max: i64,
}

impl SplitterDrag {
    /// The control's size once the mouse moved `delta` pixels (along the
    /// splitter's axis) from where the drag started.
    pub fn size_for(&self, delta: i64) -> i64 {
        (self.start + self.sign * delta).min(self.max).max(self.min)
    }
}

/// Starts dragging splitter `splitter` among its parent's `controls`, in a
/// client area `client` wide / high; `None` if nothing is there to resize
/// (or the splitter isn't aligned).
pub fn splitter_drag(client: Rect, controls: &[Control], splitter: usize, min_size: i64) -> Option<SplitterDrag> {
    let sp = controls.get(splitter)?;
    let r = sp.rect;
    let (x, y, horizontal, sign) = match sp.align {
        Align::Left => (r.left - 1, r.top, true, 1),
        Align::Right => (r.right(), r.top, true, -1),
        Align::Top => (r.left, r.top - 1, false, 1),
        Align::Bottom => (r.left, r.bottom(), false, -1),
        _ => return None,
    };
    let inside = |c: &Rect| {
        // A control of no width / height still counts on its splitter side.
        let (mut left, mut right, mut top, mut bottom) = (c.left, c.right(), c.top, c.bottom());
        if c.width == 0 {
            if matches!(sp.align, Align::Left | Align::Top) { left -= 1 } else { right += 1 }
        }
        if c.height == 0 {
            if matches!(sp.align, Align::Left | Align::Top) { top -= 1 } else { bottom += 1 }
        }
        (left..right).contains(&x) && (top..bottom).contains(&y)
    };
    let control = (0..controls.len()).find(|&i| i != splitter && controls[i].visible && inside(&controls[i].rect))?;
    let size = |c: &Control| if horizontal { c.rect.width } else { c.rect.height };
    let edge_aligns: [Align; 2] = if horizontal { [Align::Left, Align::Right] } else { [Align::Top, Align::Bottom] };
    let taken: i64 = controls.iter().filter(|c| c.visible && edge_aligns.contains(&c.align)).map(size).sum();
    let room = if horizontal { client.width } else { client.height };
    let start = size(&controls[control]);
    let max = room - min_size - taken + start;
    Some(SplitterDrag { control, horizontal, start, sign, min: min_size.min(max.max(0)), max: max.max(0) })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(align: Align, left: i64, top: i64, width: i64, height: i64) -> Control {
        Control { align, visible: true, rect: Rect::new(left, top, width, height), constraints: Constraints::default() }
    }

    fn laid(client: Rect, controls: &[Control], changed: Option<usize>) -> Vec<Rect> {
        let mut rects: Vec<Rect> = controls.iter().map(|c| c.rect).collect();
        for (i, r) in align_controls(client, controls, changed) {
            rects[i] = r;
        }
        rects
    }

    #[test]
    fn edges_then_client() {
        // A toolbar, a status bar, a tree on the left, a panel on the right
        // and an editor filling the rest of a 400×300 client area.
        let controls = [
            c(Align::Client, 0, 0, 10, 10),
            c(Align::Top, 5, 5, 50, 30),
            c(Align::Bottom, 0, 0, 50, 20),
            c(Align::Left, 0, 0, 100, 10),
            c(Align::Right, 0, 0, 60, 10),
            c(Align::None, 7, 8, 9, 10),
        ];
        let r = laid(Rect::new(0, 0, 400, 300), &controls, None);
        assert_eq!(r[1], Rect::new(0, 0, 400, 30));
        assert_eq!(r[2], Rect::new(0, 280, 400, 20));
        assert_eq!(r[3], Rect::new(0, 30, 100, 250));
        assert_eq!(r[4], Rect::new(340, 30, 60, 250));
        assert_eq!(r[0], Rect::new(100, 30, 240, 250));
        assert_eq!(r[5], Rect::new(7, 8, 9, 10), "alNone is left alone");
    }

    #[test]
    fn same_align_by_position_changed_first() {
        // Splitter (alLeft, width 5) created before Tree (alLeft, width
        // 200): aligning the tree puts it first, the splitter after it...
        let controls = [c(Align::Left, 0, 0, 5, 10), c(Align::Left, 0, 0, 200, 10), c(Align::Client, 0, 0, 1, 1)];
        let r = laid(Rect::new(0, 0, 500, 300), &controls, Some(1));
        assert_eq!(r[1].left, 0);
        assert_eq!(r[0].left, 200);
        assert_eq!(r[2], Rect::new(205, 0, 295, 300));
        // ...and from then on their positions keep that order.
        let placed = [c(Align::Left, r[0].left, 0, 5, 300), c(Align::Left, r[1].left, 0, 200, 300), c(Align::Client, 0, 0, 1, 1)];
        let again = laid(Rect::new(0, 0, 800, 300), &placed, None);
        assert_eq!((again[1].left, again[0].left), (0, 200));
        // Two alTop controls: the one nearer the top first; equal tops keep
        // creation order.
        let tops = [c(Align::Top, 0, 50, 10, 20), c(Align::Top, 0, 10, 10, 30), c(Align::Top, 0, 10, 10, 5)];
        let t = laid(Rect::new(0, 0, 100, 100), &tops, None);
        assert_eq!((t[1].top, t[2].top, t[0].top), (0, 30, 35));
        // Two alBottom controls at the same place: the later one is placed
        // first, at the very bottom.
        let bottoms = [c(Align::Bottom, 0, 0, 10, 20), c(Align::Bottom, 0, 0, 10, 20)];
        let b = laid(Rect::new(0, 0, 100, 100), &bottoms, None);
        assert_eq!((b[1].top, b[0].top), (80, 60));
    }

    #[test]
    fn invisible_and_overflowing() {
        let mut hidden = c(Align::Top, 0, 0, 10, 40);
        hidden.visible = false;
        let controls = [hidden, c(Align::Top, 0, 0, 10, 70), c(Align::Bottom, 0, 0, 10, 70), c(Align::Client, 0, 0, 1, 1)];
        let r = laid(Rect::new(0, 10, 100, 100), &controls, None);
        assert_eq!(r[0], Rect::new(0, 0, 10, 40), "an invisible control isn't moved");
        assert_eq!(r[1], Rect::new(0, 10, 100, 70));
        assert_eq!(r[2], Rect::new(0, 40, 100, 70));
        assert_eq!(r[3].height, 0, "no negative size when the space runs out");
    }

    #[test]
    fn splitter_resizes_its_neighbour() {
        // Tree (alLeft, 200) | Splitter (alLeft, 5) | Memo (alClient) in 500.
        let client = Rect::new(0, 0, 500, 300);
        let controls = [c(Align::Left, 0, 0, 200, 300), c(Align::Left, 200, 0, 5, 300), c(Align::Client, 205, 0, 295, 300)];
        let d = splitter_drag(client, &controls, 1, 30).unwrap();
        assert_eq!((d.control, d.horizontal), (0, true));
        assert_eq!(d.size_for(50), 250);
        assert_eq!(d.size_for(-500), 30, "MinSize");
        assert_eq!(d.size_for(1000), 500 - 30 - 5, "leaves MinSize for the rest");
        // An alBottom splitter above an alBottom panel: dragging up grows it.
        let controls = [c(Align::Client, 0, 0, 500, 195), c(Align::Bottom, 0, 195, 500, 5), c(Align::Bottom, 0, 200, 500, 100)];
        let d = splitter_drag(client, &controls, 1, 30).unwrap();
        assert_eq!((d.control, d.horizontal), (2, false));
        assert_eq!(d.size_for(-40), 140);
        // Nothing next to it, or not aligned: no drag.
        assert!(splitter_drag(client, &[c(Align::Left, 0, 0, 5, 300)], 0, 30).is_none());
        assert!(splitter_drag(client, &[c(Align::None, 0, 0, 5, 300)], 0, 30).is_none());
    }

    #[test]
    fn form_frame_sizes() {
        assert_eq!(form_client_size(400, 300, 2, 0), (398, 269));
        assert_eq!(form_client_size(400, 300, 2, 28), (398, 241));
        assert_eq!(form_client_size(400, 300, 0, 0), (400, 300), "bsNone has no frame");
        assert_eq!(form_outer_size(398, 241, 2, 28), (400, 300));
        assert_eq!(form_client_size(1, 1, 2, 0), (0, 0));
    }

    #[test]
    fn anchors_follow_the_parent() {
        let parent = (400, 300);
        // Right + bottom: moves, keeping its distances to those edges.
        let r = AnchorRules::new(AK_RIGHT | AK_BOTTOM, Rect::new(300, 250, 75, 25), parent).unwrap();
        assert_eq!(r.place(Rect::new(300, 250, 75, 25), (500, 350)), Rect::new(400, 300, 75, 25));
        // Left + right: stretches; top only: stays.
        let r = AnchorRules::new(AK_LEFT | AK_TOP | AK_RIGHT, Rect::new(10, 10, 200, 25), parent).unwrap();
        assert_eq!(r.place(Rect::new(10, 10, 200, 25), (500, 200)), Rect::new(10, 10, 300, 25));
        assert_eq!(r.place(Rect::new(10, 10, 300, 25), (100, 200)), Rect::new(10, 10, 0, 25), "never a negative size");
        // Neither left nor right: its centre keeps its proportion (MulDiv).
        let r = AnchorRules::new(AK_TOP, Rect::new(150, 0, 100, 20), parent).unwrap();
        assert_eq!(r.place(Rect::new(150, 0, 100, 20), (600, 300)).left, 250);
        assert_eq!(r.place(Rect::new(150, 0, 100, 20), (401, 300)).left, 151, "200 * 401 / 400 = 200.5 rounds up");
        // The rules are the first placement's: following again from where
        // anchoring put it lands at the same place.
        let r = AnchorRules::new(AK_RIGHT | AK_TOP, Rect::new(300, 5, 75, 25), parent).unwrap();
        let moved = r.place(Rect::new(300, 5, 75, 25), (450, 300));
        assert_eq!(r.place(moved, (400, 300)), Rect::new(300, 5, 75, 25));
        // Nothing to follow: the default anchors, a parent of no size.
        assert!(AnchorRules::new(DEFAULT_ANCHORS, Rect::new(1, 2, 3, 4), parent).is_none());
        assert!(AnchorRules::new(AK_RIGHT, Rect::new(1, 2, 3, 4), (0, 300)).is_none());
    }

    #[test]
    fn anchored_children_and_their_store() {
        let parent = (400, 300);
        let rule = |a, r| AnchorRules::new(a, r, parent);
        let btn = Rect::new(300, 250, 75, 25);
        let panel = Rect::new(10, 50, 300, 100);
        let min = Constraints { min_width: 250, ..Constraints::default() };
        let children = [
            (rule(AK_RIGHT | AK_BOTTOM, btn), btn, Align::None, Constraints::default()),
            (rule(AK_LEFT | AK_TOP | AK_RIGHT, panel), panel, Align::None, min),
            (rule(AK_RIGHT, btn), btn, Align::Top, Constraints::default()),
            (None, btn, Align::None, Constraints::default()),
        ];
        // Smaller: the button moves, the panel shrinks to its MinWidth; the
        // aligned one (Align wins) and the unanchored one stay.
        let moved = anchor_controls((300, 280), &children);
        assert_eq!(moved, vec![(0, Rect::new(200, 230, 75, 25)), (1, Rect::new(10, 50, 250, 100))]);
        assert!(anchor_controls(parent, &children).is_empty(), "same size: nothing moves");
        anchor_record("Btn", AK_RIGHT, btn, parent);
        assert_eq!(anchor_rules("BTN").map(|r| r.anchors()), Some(AK_RIGHT));
        anchor_record("btn", DEFAULT_ANCHORS, btn, parent);
        assert_eq!(anchor_rules("Btn"), None, "the default anchors forget");
        assert_eq!(mul_div(-3, 1, 2), -2);
        assert_eq!(mul_div(5, 1, 0), -1);
    }

    #[test]
    fn constraints_bound_sizes() {
        let k = Constraints { min_width: 100, min_height: 0, max_width: 300, max_height: 50 };
        assert_eq!(k.size(50, 80), (100, 50));
        assert_eq!(k.size(500, 10), (300, 10));
        assert!(Constraints::default().is_none());
        assert_eq!(Constraints::default().size(-5, 7), (-5, 7), "no limits: unchanged");
        // Delphi's setters keep the minimum at most the maximum.
        let k = Constraints::default().with("maxwidth", 200).unwrap().with("minwidth", 250).unwrap();
        assert_eq!((k.min_width, k.max_width), (250, 250));
        let k = k.with("maxwidth", 120).unwrap();
        assert_eq!((k.min_width, k.max_width), (120, 120));
        assert_eq!(Constraints::default().with("minheight", -4).unwrap().min_height, 0);
        assert!(Constraints::default().with("width", 1).is_none());
        assert_eq!(constraint_alias("constraints.maxheight"), Some("maxheight"));
        assert_eq!(constraint_alias("constraints.width"), None);
        assert_eq!(default_property("RBUTTON", "anchors"), Some(DEFAULT_ANCHORS));
        assert_eq!(default_property("RPANEL", "constraints.minwidth"), Some(0));
        assert_eq!(default_property("RTIMER", "anchors"), None, "not a visual component");
        // Aligned controls keep theirs: an alTop panel's MinHeight is the
        // strip it takes; an alClient one's MaxWidth leaves room unfilled.
        let mut top = c(Align::Top, 0, 0, 10, 20);
        top.constraints.min_height = 40;
        let mut client = c(Align::Client, 0, 0, 1, 1);
        client.constraints.max_width = 150;
        let r = laid(Rect::new(0, 0, 400, 300), &[top, client], None);
        assert_eq!(r[0], Rect::new(0, 0, 400, 40));
        assert_eq!(r[1], Rect::new(0, 40, 150, 260));
    }

    #[test]
    fn values_and_defaults() {
        for n in 0..=5 {
            assert_eq!(Align::from_value(n).value(), n);
        }
        assert_eq!(Align::from_value(99), Align::None);
        assert_eq!(default_align("RStatusBar"), Align::Bottom);
        // (the input lane's: a status bar's size grip — sizeable forms, the bottom)
        assert!(status_grip(true, true, 2, Align::Bottom) && status_grip(true, true, 5, Align::Bottom));
        assert!(!status_grip(false, true, 2, Align::Bottom), "SizeGrip off");
        assert!(!status_grip(true, false, 2, Align::Bottom), "on a panel");
        assert!(!status_grip(true, true, 1, Align::Bottom) && !status_grip(true, true, 3, Align::Bottom), "bsSingle, bsDialog");
        assert!(!status_grip(true, true, 2, Align::Top), "docked at the top");
        assert_eq!(default_align("RSPLITTER"), Align::Left);
        assert_eq!(default_align("RBUTTON"), Align::None);
    }
}
