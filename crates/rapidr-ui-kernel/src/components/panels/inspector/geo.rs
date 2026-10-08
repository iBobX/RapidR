//! Where an inspector's parts are (its own logical pixels), the same for
//! every theme so that a click lands on the same part everywhere: the
//! subject's header, the Properties / Events tabs, the search strip with
//! the view's button, the rows with the line between their columns, the
//! description at the bottom; inside a row its expander, name, reset
//! glyph, value, check box and button; the anchors editor's pins and the
//! colour picker's swatches.

use rapidr_value::objects::ops::Rect;
use rapidr_value::panels::inspector::values::{self, Kind};
use rapidr_value::panels::inspector::{model::PINS_H, Inspector, Row, RowKind};
use rapidr_value::panels::rows::{ROW, SEARCH, TABS};

use super::super::common::inside;

/// The subject's header (its icon, name and type).
pub const HEADER: i64 = 30;
/// The description under the rows (when the panel is tall enough).
pub const FOOTER: i64 = 54;
/// How far a level of parts steps in.
pub const STEP: i64 = 12;
/// A row's button (drop / "…") width.
pub const BUTTON: i64 = 18;

/// The panel's areas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geo {
    pub header: Rect,
    pub tabs: Option<Rect>,
    /// The search strip, its box, the view's button.
    pub strip: Rect,
    pub search: Rect,
    pub view_button: Rect,
    /// The rows' area (the scroll bar inside it, at its right).
    pub list: Rect,
    pub footer: Option<Rect>,
}

/// The areas of an inspector `w` × `h` (`inset`: a classic edge's).
pub fn geo(m: &Inspector, w: i64, h: i64, inset: i64) -> Geo {
    let (x, iw) = (inset, (w - 2 * inset).max(0));
    let mut y = inset;
    let header = (x, y, iw, HEADER);
    y += HEADER;
    let tabs = m.show_events.then(|| {
        let r = (x, y, iw, TABS);
        y += TABS;
        r
    });
    let strip = (x, y, iw, SEARCH);
    let search = (x + 6, y + 5, (iw - 12 - 28).max(0), SEARCH - 10);
    let view_button = (x + iw - 6 - 24, y + 5, 24, SEARCH - 10);
    y += SEARCH;
    let bottom = h - inset;
    let footer = (bottom - y > 200).then(|| (x, bottom - FOOTER, iw, FOOTER));
    let list_h = (footer.map_or(bottom, |f| f.1) - y - 1).max(0);
    Geo { header, tabs, strip, search, view_button, list: (x, y + 1, iw, list_h), footer }
}

/// The name column's width in a list `cw` wide.
pub fn name_width(m: &Inspector, cw: i64) -> i64 {
    m.name_width.clamp(40.min(cw / 2), (cw - 40).max(40.min(cw / 2)))
}

/// What a press lands on in a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Expander,
    Name,
    Reset,
    Value,
    Check,
    Button,
    /// The anchors editor's pin (0 left, 1 top, 2 right, 3 bottom).
    Pin(usize),
    /// The colour picker's swatch.
    Swatch(usize),
    /// The colour picker's value field.
    Field,
}

/// A row's place: (x, y, w, h) in the panel, given the list's scroll.
pub fn row_rect(g: &Geo, rows: &[Row], i: usize, pos: i64, cw: i64) -> Rect {
    let top: i64 = rows[..i].iter().map(|r| r.height).sum();
    (g.list.0, g.list.1 + top - pos, cw, rows[i].height)
}

/// Where row `r`'s parts are, its rectangle `rr`, the name column `nw`
/// wide.
pub struct RowParts {
    pub expander: Option<(i64, i64)>,
    pub name_x: i64,
    pub reset: Rect,
    pub value: Rect,
    pub check: Option<Rect>,
    pub swatch: Option<Rect>,
    pub text_x: i64,
    pub button: Option<Rect>,
}

/// A row's value kind (a set's flag is a Boolean, a font's part its own).
pub fn row_kind(m: &Inspector, r: &Row) -> Option<Kind> {
    match r.kind {
        RowKind::Event => Some(Kind::Text),
        RowKind::Category | RowKind::Pins | RowKind::Picker => None,
        RowKind::Flag(_) => Some(Kind::Bool),
        RowKind::FontPart(i) => m.snap.props.get(r.prop)?.parts.get(i).map(|q| q.kind.clone()),
        RowKind::Line(_) | RowKind::AddLine => Some(Kind::Text),
        RowKind::Prop => m.snap.props.get(r.prop).map(|p| p.kind.clone()),
    }
}

/// The parts of row `r` at `rr`.
pub fn parts(m: &Inspector, r: &Row, rr: Rect, nw: i64) -> RowParts {
    let (x, y, w, h) = rr;
    // (by categories, a category's rows line up with its title)
    let level = if m.alphabetic || m.on_events() { r.level } else { r.level.saturating_sub(1) };
    let indent = x + level as i64 * STEP;
    let expander = r.expandable.then_some((indent + 10, y + h / 2));
    let name_x = indent + 20;
    let reset = (x + nw - 20, y + (h - 16) / 2, 16, 16);
    let vx = x + nw + 1;
    let value = (vx, y, (w - nw - 1).max(0), h);
    let kind = row_kind(m, r);
    let check = matches!(kind, Some(Kind::Bool)).then_some((vx + 6, y + (h - 13) / 2, 13, 13));
    let swatch = matches!(kind, Some(Kind::Color)).then_some((vx + 6, y + (h - 14) / 2, 14, 14));
    let text_x = match (check, swatch) {
        (Some(c), _) => c.0 + c.2 + 6,
        (_, Some(s)) => s.0 + s.2 + 6,
        _ => vx + 6,
    };
    let has_button = match (r.kind, &kind) {
        (RowKind::Event, _) => true,
        (RowKind::Prop | RowKind::FontPart(_), Some(k)) => k.drops() && !matches!(k, Kind::Bool) || k.ellipsis() || matches!(k, Kind::Color),
        _ => false,
    };
    let button = has_button.then_some((x + w - BUTTON - 2, y + 2, BUTTON, h - 4));
    RowParts { expander, name_x, reset, value, check, swatch, text_x, button }
}

/// The anchors editor's frames in its row `rr`: the parent, the control,
/// and each pin's strut (left, top, right, bottom) as a hit rectangle.
pub fn pins(rr: Rect) -> (Rect, Rect, [Rect; 4]) {
    let (x, y, w, _) = rr;
    let (pw, ph) = (156, PINS_H - 26);
    let parent = (x + (w - pw) / 2, y + 8, pw, ph);
    let (cw, ch) = (64, 28);
    let control = (parent.0 + (pw - cw) / 2, parent.1 + (ph - ch) / 2, cw, ch);
    let (cx, cy) = (control.0 + cw / 2, control.1 + ch / 2);
    let left = (parent.0 + 1, cy - 8, control.0 - parent.0 - 2, 16);
    let right = (control.0 + cw + 1, cy - 8, parent.0 + pw - control.0 - cw - 2, 16);
    let top = (cx - 8, parent.1 + 1, 16, control.1 - parent.1 - 2);
    let bottom = (cx - 8, control.1 + ch + 1, 16, parent.1 + ph - control.1 - ch - 2);
    (parent, control, [left, top, right, bottom])
}

/// The colour picker's places in its row `rr`: each swatch (the 16
/// standard colours, then the system's), the current colour's field.
pub struct Picker {
    pub standard_label: Rect,
    pub system_label: Rect,
    pub swatches: Vec<Rect>,
    pub preview: Rect,
    pub field: Rect,
}

/// Swatches across a picker's grid.
pub const COLUMNS: usize = 8;
const SWATCH: i64 = 18;
const GAP: i64 = 4;

pub fn picker(rr: Rect) -> Picker {
    let (x, y, w, _) = rr;
    let grid_w = COLUMNS as i64 * (SWATCH + GAP) - GAP;
    let x0 = x + ((w - grid_w) / 2).max(8);
    let cell = |row: i64, col: i64, top: i64| (x0 + col * (SWATCH + GAP), top + row * (SWATCH + GAP), SWATCH, SWATCH);
    let standard_label = (x0, y + 6, grid_w, 14);
    let mut swatches = Vec::new();
    for i in 0..values::STANDARD_COLORS.len() as i64 {
        swatches.push(cell(i / COLUMNS as i64, i % COLUMNS as i64, y + 22));
    }
    let system_top = y + 22 + 2 * (SWATCH + GAP) + 4;
    let system_label = (x0, system_top, grid_w, 14);
    for i in 0..values::SYSTEM_COLORS.len() as i64 {
        swatches.push(cell(i / COLUMNS as i64, i % COLUMNS as i64, system_top + 16));
    }
    let bottom = system_top + 16 + 3 * (SWATCH + GAP) + 2;
    let preview = (x0, bottom, 28, 20);
    let field = (x0 + 34, bottom, grid_w - 34, 20);
    Picker { standard_label, system_label, swatches, preview, field }
}

/// The colour a picker's swatch `i` is.
pub fn swatch_name(i: usize) -> &'static str {
    values::STANDARD_COLORS.get(i).copied().unwrap_or_else(|| values::SYSTEM_COLORS.get(i - values::STANDARD_COLORS.len()).copied().unwrap_or("clBlack"))
}

/// How many swatches a picker has.
pub fn swatch_count() -> usize {
    values::STANDARD_COLORS.len() + values::SYSTEM_COLORS.len()
}

/// What a press at (x, y) of row `r` (at `rr`) lands on.
pub fn part_at(m: &Inspector, r: &Row, rr: Rect, nw: i64, x: i64, y: i64) -> Part {
    match r.kind {
        RowKind::Pins => {
            let (_, _, struts) = pins(rr);
            return struts.iter().position(|s| inside(*s, x, y)).map_or(Part::Name, Part::Pin);
        }
        RowKind::Picker => {
            let pk = picker(rr);
            if inside(pk.field, x, y) || inside(pk.preview, x, y) {
                return Part::Field;
            }
            return pk.swatches.iter().position(|s| inside(*s, x, y)).map_or(Part::Name, Part::Swatch);
        }
        RowKind::Category => return Part::Expander,
        _ => {}
    }
    let p = parts(m, r, rr, nw);
    if let Some((cx, _)) = p.expander {
        if x >= cx - 8 && x < cx + 8 && x < rr.0 + nw {
            return Part::Expander;
        }
    }
    if x < rr.0 + nw {
        return if inside(p.reset, x, y) && !r.is_default && !r.mixed && resettable(r) { Part::Reset } else { Part::Name };
    }
    if p.button.is_some_and(|b| inside((b.0, rr.1, b.2, rr.3), x, y)) {
        return Part::Button;
    }
    if p.check.is_some_and(|c| inside((c.0 - 3, rr.1, c.2 + 6, rr.3), x, y)) {
        return Part::Check;
    }
    Part::Value
}

/// A row that can be put back to its default.
pub fn resettable(r: &Row) -> bool {
    matches!(r.kind, RowKind::Prop | RowKind::FontPart(_) | RowKind::Event)
}

/// The list's content height.
pub fn content(rows: &[Row]) -> i64 {
    rows.iter().map(|r| r.height).sum::<i64>().max(ROW)
}
