//! What a form looks like, as data: a list of positioned ops (the shared
//! op vocabulary, `rapidr_value::objects::ops::Op`) and parley editor
//! layouts, in drawing order. The host renders it (vello on the GPU or the
//! CPU); a test reads it. Ops are in logical pixels relative to their
//! item's `origin` (the component's absolute position in the window's
//! client area, below nothing: an in-window menu bar is inside the list);
//! editor items are in device pixels, as their layouts are.

use std::collections::HashMap;
use std::sync::Arc;

use rapidr_value::objects::ops::{Op, Place, Rect};

/// A rectangle in device pixels (x0, y0, x1, y1), relative to its
/// [`TextItem`]'s origin.
pub type DevRect = (f64, f64, f64, f64);

/// An editor's text: node `node`'s parley layout (the host reads it with
/// [`crate::FormUi::editor_layout`]) drawn at `origin`, with the
/// selection's highlight and the caret. The host draws the highlight
/// rectangles, the layout in its own colours, the selected glyphs again in
/// `highlight_text` clipped to the highlight, the composition underlines,
/// then the caret.
#[derive(Clone, Debug, PartialEq)]
pub struct TextItem {
    pub node: String,
    /// Which paragraph's layout of the node's editor (a memo has one per
    /// paragraph; [`crate::FormUi::editor_layout_at`]).
    pub para: usize,
    /// Where the layout's (0, 0) is, in device pixels in the window.
    pub origin: (f64, f64),
    pub selection: Vec<DevRect>,
    pub highlight: u32,
    pub highlight_text: u32,
    /// An input method's composition (preedit), underlined.
    pub underlines: Vec<DevRect>,
    pub caret: Option<DevRect>,
    pub caret_color: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    /// `op` drawn with its (0, 0) at `origin` (logical pixels).
    Op { origin: (i64, i64), op: Op },
    Text(TextItem),
}

/// A picture a component made while painting (a list view's rows, an
/// owner-drawn item's bitmap, a tree's icons): straight-alpha RGBA,
/// `width` × `height` pixels. An [`Op::Image`] whose `source` names it
/// draws it scaled into the op's rectangle.
#[derive(Clone, Debug, PartialEq)]
pub struct Picture {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

/// A form's client area, drawn: `size` logical pixels (with an in-window
/// main menu's height), for a screen of `scale` device pixels per logical
/// pixel (editor layouts are made for it).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DisplayList {
    pub size: (i64, i64),
    pub scale: f64,
    pub items: Vec<Item>,
    /// The pictures [`Op::Image`]s draw, by `source` (those not here are
    /// the program's bitmaps, which the host finds by object id).
    pub images: HashMap<String, Arc<Picture>>,
}

impl DisplayList {
    /// One line per item, for tests and debugging: `fill 0,0 10x5 #f0f0f0 @8,12`.
    pub fn dump(&self) -> String {
        let mut out = String::new();
        for item in &self.items {
            let line = match item {
                Item::Op { origin: (ox, oy), op } => format!("{} @{ox},{oy}", op_line(op)),
                Item::Text(t) => format!(
                    "editor {}#{} @{:.1},{:.1} sel {} caret {}",
                    t.node,
                    t.para,
                    t.origin.0,
                    t.origin.1,
                    t.selection.len(),
                    if t.caret.is_some() { "on" } else { "off" }
                ),
            };
            out.push_str(&line);
            out.push('\n');
        }
        out
    }
}

fn rect((x, y, w, h): Rect) -> String {
    format!("{x},{y} {w}x{h}")
}

fn op_line(op: &Op) -> String {
    match op {
        Op::Fill { rect: r, color } => format!("fill {} #{color:06x}", rect(*r)),
        Op::Edge { rect: r, light, dark } => {
            let c = |v: &[u32]| v.iter().map(|c| format!("#{c:06x}")).collect::<Vec<_>>().join("/");
            format!("edge {} {} {}", rect(*r), c(light), c(dark))
        }
        Op::Line { from, to, color } => format!("line {},{}-{},{} #{color:06x}", from.0, from.1, to.0, to.1),
        Op::Shape(s) => format!("shape {} points fill {} stroke {}", s.points.len(), s.fill.map_or("-".into(), |c| format!("#{c:06x}")), s.stroke.map_or("-".into(), |c| format!("#{c:06x}"))),
        Op::Text { rect: r, text, font, color, angle, place } => {
            let place = match place {
                Place::Center => "center",
                Place::TopLeft => "topleft",
                Place::TopCenter => "topcenter",
                Place::TopRight => "topright",
                Place::Left => "left",
            };
            format!("text {} {text:?} {} {}px #{color:06x} {place}{}", rect(*r), font.name, font.pixel_size(), if *angle != 0 { format!(" {angle}deg") } else { String::new() })
        }
        Op::Focus { rect: r } => format!("focus {}", rect(*r)),
        Op::Arrow { color, .. } => format!("arrow #{color:06x}"),
        Op::Checker { rect: r, a, b } => format!("checker {} #{a:06x}/#{b:06x}", rect(*r)),
        Op::Round { rect: r, radius, fill, stroke, width } => {
            let c = |c: &Option<u32>| c.map_or("-".into(), |c| format!("#{c:06x}"));
            format!("round {} r{radius} fill {} stroke {} w{width}", rect(*r), c(fill), c(stroke))
        }
        Op::Shadow { rect: r, radius, size, drop, color, alpha } => format!("shadow {} r{radius} s{size} d{drop} #{color:06x}/{alpha}", rect(*r)),
        Op::Stroke { points, color, width } => format!("stroke {} points #{color:06x} w{width}", points.len()),
        Op::Polygon { points, color } => format!("polygon {} points #{color:06x}", points.len()),
        Op::Image { source, revision, rect: r } => format!("image {source}#{revision} {}", rect(*r)),
        Op::ClipPush { rect: r } => format!("clip {}", rect(*r)),
        Op::ClipPolygon { points } => format!("clip polygon {} points", points.len()),
        Op::ClipPop => "unclip".to_string(),
    }
}
