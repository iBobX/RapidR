//! RDESIGNSURFACE, RapidR's form designer (the IDE's `examples/ide.rr`):
//! the designed components — type, name, bounds and their properties as
//! strings — the selection, and what the mouse does to them (a press
//! selects, a drag moves, the selected one's right, bottom and corner
//! handles resize, all on the 8-pixel grid), shared by every runtime. What
//! the program hears comes back as [`DesignEvent`]s: OnSelect (Index),
//! OnDblClick (Index), OnBgClick (X, Y), OnMove (Index, X, Y, W, H).
//!
//! RapidQ has no designer; FLTK's runtime was the first to draw one, so
//! its look is the reference: [`DesignSurface::ops`] is that drawing in the
//! shared op vocabulary (white with grid dots, each designed component as a
//! placeholder of its type, the selection's frame and handles) for the
//! hosts that draw ops. The designed form's caption and size are the
//! program's (FormCaption, the surface's Width / Height): the surface *is*
//! the designed form's inside.

use std::collections::BTreeMap;

use super::font::Font;
use super::ops::{Op, Place, Rect};
use super::trackbar::Shape;
use crate::{v_int, v_str, Value};

/// The grid moves and resizes snap to (and its dots are drawn on).
pub const GRID: i64 = 8;
/// How near a handle the mouse grabs it (either way).
const GRAB: i64 = 5;
/// The smallest a resize leaves a component.
const MIN_SIZE: i64 = 16;
/// A handle's size, drawn.
const HANDLE: i64 = 5;

/// A designed component.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignComp {
    pub name: String,
    /// Its RapidR type name as the program gave it (`RBUTTON` …).
    pub type_name: String,
    pub x: i64,
    pub y: i64,
    pub w: i64,
    pub h: i64,
    /// Its properties (lowercase names), as SetProp stored them.
    pub props: BTreeMap<String, String>,
}

impl DesignComp {
    pub fn prop(&self, name: &str) -> Option<&str> {
        self.props.get(name).map(String::as_str)
    }

    /// Its Caption, else its name.
    pub fn caption(&self) -> &str {
        self.prop("caption").unwrap_or(&self.name)
    }

    pub fn bounds(&self) -> Rect {
        (self.x, self.y, self.w, self.h)
    }

    /// A colour property (0xRRGGBB): see [`parse_color`].
    pub fn color(&self, prop: &str) -> Option<u32> {
        self.prop(prop).and_then(parse_color)
    }

    /// A true / false property: "1" or "True" (as the designer stores them).
    fn on(&self, prop: &str) -> bool {
        self.prop(prop).is_some_and(|s| s == "1" || s.eq_ignore_ascii_case("true"))
    }

    /// The font its Font.Name / FontName and Font.Size / FontSize (points)
    /// say; 12-pixel Arial when unset.
    pub fn font(&self) -> Font {
        let pick = |a: &str, b: &str| self.prop(a).or_else(|| self.prop(b)).map(str::trim).filter(|s| !s.is_empty());
        let name = pick("font.name", "fontname").unwrap_or("Arial").to_string();
        let size = pick("font.size", "fontsize").and_then(|s| s.parse::<i64>().ok()).filter(|s| *s > 0).unwrap_or(-12);
        Font { name, size, ..Font::default() }
    }
}

/// A colour as the designer's properties hold one (0xRRGGBB): `#RRGGBB`,
/// `rgb(r, g, b)`, or RapidQ's `&HBBGGRR` / the number a QCOLORDIALOG's
/// Color gives (BGR).
pub fn parse_color(s: &str) -> Option<u32> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix('#') {
        let byte = |i: usize| hex.get(i..i + 2).and_then(|h| u8::from_str_radix(h, 16).ok());
        return Some(u32::from(byte(0)?) << 16 | u32::from(byte(2)?) << 8 | u32::from(byte(4)?));
    }
    if let Some(inner) = s.strip_prefix("rgb(").and_then(|r| r.strip_suffix(')')) {
        let parts: Vec<u8> = inner.split(',').map(|p| p.trim().parse::<u8>()).collect::<Result<_, _>>().ok()?;
        let [r, g, b] = <[u8; 3]>::try_from(parts).ok()?;
        return Some(u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b));
    }
    let n = match s.strip_prefix("&H").or_else(|| s.strip_prefix("&h")) {
        Some(hex) => i64::from_str_radix(hex, 16).ok()?,
        None => s.parse::<i64>().ok()?,
    };
    if !(0..=0xFF_FFFF).contains(&n) {
        return None;
    }
    let n = n as u32;
    Some((n & 0xFF) << 16 | (n & 0xFF00) | (n >> 16))
}

/// What a press on the selected component's handles grabbed (else it
/// moves).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Grip {
    #[default]
    Move,
    Right,
    Bottom,
    Corner,
}

/// What the program hears.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DesignEvent {
    /// OnSelect (Index): a component pressed.
    Select(usize),
    /// OnDblClick (Index): a component pressed twice.
    DblClick(usize),
    /// OnBgClick (X, Y): the background pressed (nothing selected now).
    BgClick(i64, i64),
    /// OnMove (Index, X, Y, W, H): the selected component dragged or
    /// resized (each step of the drag).
    Move { index: usize, x: i64, y: i64, w: i64, h: i64 },
}

impl DesignEvent {
    /// The event's name (lowercase, as handlers are stored).
    pub fn event(&self) -> &'static str {
        match self {
            DesignEvent::Select(_) => "onselect",
            DesignEvent::DblClick(_) => "ondblclick",
            DesignEvent::BgClick(..) => "onbgclick",
            DesignEvent::Move { .. } => "onmove",
        }
    }

    /// Its arguments.
    pub fn args(&self) -> Vec<i64> {
        match *self {
            DesignEvent::Select(i) | DesignEvent::DblClick(i) => vec![i as i64],
            DesignEvent::BgClick(x, y) => vec![x, y],
            DesignEvent::Move { index, x, y, w, h } => vec![index as i64, x, y, w, h],
        }
    }
}

/// The surface: its components (the last drawn on top), the selection and
/// a drag in progress.
#[derive(Clone, Debug, PartialEq)]
pub struct DesignSurface {
    pub components: Vec<DesignComp>,
    /// The selected component; -1 for none (SelectComp stores any number).
    pub selected: i64,
    /// The designed form's caption (FormCaption).
    pub form_caption: String,
    grip: Grip,
    /// Where the press was in the component being moved.
    offset: (i64, i64),
    /// The mouse is down (a drag goes on).
    held: bool,
}

impl Default for DesignSurface {
    fn default() -> Self {
        DesignSurface { components: Vec::new(), selected: -1, form_caption: "Form1".into(), grip: Grip::Move, offset: (0, 0), held: false }
    }
}

/// `v` rounded to the grid (half up), as the designer snaps.
fn snap(v: i64) -> i64 {
    ((v + GRID / 2) / GRID) * GRID
}

impl DesignSurface {
    /// The selected component's index, when it is one.
    pub fn selection(&self) -> Option<usize> {
        usize::try_from(self.selected).ok().filter(|&i| i < self.components.len())
    }

    fn comp(&self, i: i64) -> Option<&DesignComp> {
        usize::try_from(i).ok().and_then(|i| self.components.get(i))
    }

    fn comp_mut(&mut self, i: i64) -> Option<&mut DesignComp> {
        usize::try_from(i).ok().and_then(|i| self.components.get_mut(i))
    }

    /// AddComponent: the new one is selected.
    pub fn add(&mut self, type_name: &str, name: &str, bounds: Rect) {
        let (x, y, w, h) = bounds;
        let mut props = BTreeMap::new();
        props.insert("caption".to_string(), name.to_string());
        self.components.push(DesignComp { name: name.to_string(), type_name: type_name.to_string(), x, y, w, h, props });
        self.selected = self.components.len() as i64 - 1;
    }

    /// RemoveComponent: the selection goes.
    pub fn remove(&mut self, i: i64) -> bool {
        let Some(i) = usize::try_from(i).ok().filter(|&i| i < self.components.len()) else { return false };
        self.components.remove(i);
        self.selected = -1;
        true
    }

    /// ClearAll.
    pub fn clear(&mut self) {
        self.components.clear();
        self.selected = -1;
    }

    /// The selected component's handle at (x, y), if the mouse is on one
    /// (the corner first, then the right and bottom middles).
    pub fn grip_at(&self, x: i64, y: i64) -> Option<Grip> {
        let c = self.selection().map(|i| &self.components[i])?;
        let near = |hx: i64, hy: i64| (x - hx).abs() <= GRAB && (y - hy).abs() <= GRAB;
        if near(c.x + c.w, c.y + c.h) {
            Some(Grip::Corner)
        } else if near(c.x + c.w, c.y + c.h / 2) {
            Some(Grip::Right)
        } else if near(c.x + c.w / 2, c.y + c.h) {
            Some(Grip::Bottom)
        } else {
            None
        }
    }

    /// The topmost component at (x, y) (its edges included).
    pub fn component_at(&self, x: i64, y: i64) -> Option<usize> {
        self.components.iter().rposition(|c| x >= c.x && x <= c.x + c.w && y >= c.y && y <= c.y + c.h)
    }

    /// The mouse pressed at (x, y) of the surface (`double`: the second
    /// press of a double click): a handle grabbed (nothing heard), a
    /// component selected (OnSelect / OnDblClick), or the background
    /// (nothing selected, OnBgClick).
    pub fn mouse_down(&mut self, x: i64, y: i64, double: bool) -> Option<DesignEvent> {
        self.held = true;
        if let Some(grip) = self.grip_at(x, y) {
            self.grip = grip;
            return None;
        }
        self.grip = Grip::Move;
        match self.component_at(x, y) {
            Some(i) => {
                let c = &self.components[i];
                self.offset = (x - c.x, y - c.y);
                self.selected = i as i64;
                Some(if double { DesignEvent::DblClick(i) } else { DesignEvent::Select(i) })
            }
            None => {
                self.selected = -1;
                Some(DesignEvent::BgClick(x, y))
            }
        }
    }

    /// The mouse dragged to (x, y): the selected component moves (or its
    /// grabbed handle resizes it) on the grid — OnMove at every step.
    pub fn mouse_drag(&mut self, x: i64, y: i64) -> Option<DesignEvent> {
        if !self.held {
            return None;
        }
        let index = self.selection()?;
        let (grip, (ox, oy)) = (self.grip, self.offset);
        let c = &mut self.components[index];
        match grip {
            Grip::Right => c.w = snap(x - c.x).max(MIN_SIZE),
            Grip::Bottom => c.h = snap(y - c.y).max(MIN_SIZE),
            Grip::Corner => {
                c.w = snap(x - c.x).max(MIN_SIZE);
                c.h = snap(y - c.y).max(MIN_SIZE);
            }
            Grip::Move => {
                c.x = snap(x - ox).max(0);
                c.y = snap(y - oy).max(0);
            }
        }
        Some(DesignEvent::Move { index, x: c.x, y: c.y, w: c.w, h: c.h })
    }

    /// The mouse let go: the drag ends (nothing heard).
    pub fn mouse_up(&mut self) {
        self.held = false;
        self.grip = Grip::Move;
    }

    /// A property (`None`: not the surface's; the runtime keeps it).
    pub fn get(&self, prop: &str) -> Option<Value> {
        Some(match prop {
            "compcount" | "count" => v_int(self.components.len() as i64),
            "formcaption" => v_str(&self.form_caption),
            _ => return None,
        })
    }

    pub fn set(&mut self, prop: &str, val: &Value) -> bool {
        match prop {
            "formcaption" => self.form_caption = val.to_string_val(),
            _ => return false,
        }
        true
    }

    /// Its methods (`None`: not one of them; Show and Hide are the host's).
    pub fn call(&mut self, method: &str, args: &[Value]) -> Option<Value> {
        let int = |i: usize, default: i64| args.get(i).map_or(default, Value::to_i64);
        let text = |i: usize| args.get(i).map(Value::to_string_val).unwrap_or_default();
        let index = int(0, -1);
        Some(match method {
            // AddComponent(Type, Name, X, Y, W, H)
            "addcomponent" => {
                self.add(&text(0), &text(1), (int(2, 0), int(3, 0), int(4, 80), int(5, 25)));
                Value::Null
            }
            "getname" => v_str(self.comp(index).map_or("", |c| c.name.as_str())),
            "gettype" => v_str(self.comp(index).map_or("", |c| c.type_name.as_str())),
            "getcompx" => v_int(self.comp(index).map_or(0, |c| c.x)),
            "getcompy" => v_int(self.comp(index).map_or(0, |c| c.y)),
            "getcompw" => v_int(self.comp(index).map_or(0, |c| c.w)),
            "getcomph" => v_int(self.comp(index).map_or(0, |c| c.h)),
            // SetProp(Index, Name, Value) / GetProp(Index, Name)
            "setprop" => {
                let (prop, value) = (text(1).to_lowercase(), text(2));
                if let Some(c) = self.comp_mut(index) {
                    c.props.insert(prop, value);
                }
                Value::Null
            }
            "getprop" => v_str(self.comp(index).and_then(|c| c.prop(&text(1).to_lowercase())).unwrap_or("")),
            // SetCompBounds(Index, X, Y, W, H)
            "setcompbounds" => {
                let b = (int(1, 0), int(2, 0), int(3, 80), int(4, 25));
                if let Some(c) = self.comp_mut(index) {
                    (c.x, c.y, c.w, c.h) = b;
                }
                Value::Null
            }
            "setname" => {
                let name = text(1);
                if let Some(c) = self.comp_mut(index) {
                    c.name = name;
                }
                Value::Null
            }
            "selectcomp" => {
                self.selected = index;
                Value::Null
            }
            "removecomponent" => {
                self.remove(index);
                Value::Null
            }
            "clearall" => {
                self.clear();
                Value::Null
            }
            "count" => v_int(self.components.len() as i64),
            _ => return None,
        })
    }

    // -------------------------------------------------------- drawing --

    /// The surface drawn at `w` × `h` (its own pixels) as FLTK's runtime
    /// draws it: white with grey dots on the grid, each component as a
    /// placeholder of its type (its Caption, Color, FontColor, Font where
    /// FLTK shows them), the selected one framed in blue with its handles.
    pub fn ops(&self, w: i64, h: i64) -> Vec<Op> {
        let mut d = Draw::default();
        d.fill((0, 0, w, h), WHITE);
        for gx in (0..w.max(0)).step_by(GRID as usize) {
            for gy in (0..h.max(0)).step_by(GRID as usize) {
                d.fill((gx, gy, 1, 1), 0xC8C8C8);
            }
        }
        for (i, c) in self.components.iter().enumerate() {
            d.component(c);
            if self.selection() == Some(i) {
                d.rect(c.bounds(), 0x0078D7);
                d.handles(c.bounds());
            }
        }
        d.ops
    }
}

const WHITE: u32 = 0xFFFFFF;
const BLACK: u32 = 0x000000;

/// FLTK's Helvetica at `px` pixels (Arial's metrics: Liberation Sans).
fn sans(px: i64) -> Font {
    Font { name: "Arial".into(), size: -px, ..Font::default() }
}

fn mono(px: i64) -> Font {
    Font { name: "Courier New".into(), size: -px, ..Font::default() }
}

/// A colour lighter (`d` > 0) or darker by `d` per channel (saturating).
fn shade(c: u32, d: i32) -> u32 {
    let ch = |s: u32| ((((c >> s) & 0xFF) as i32 + d).clamp(0, 255) as u32) << s;
    ch(16) | ch(8) | ch(0)
}

/// Ops as FLTK's drawing calls make them.
#[derive(Default)]
struct Draw {
    ops: Vec<Op>,
}

impl Draw {
    fn fill(&mut self, rect: Rect, color: u32) {
        if rect.2 > 0 && rect.3 > 0 {
            self.ops.push(Op::Fill { rect, color });
        }
    }

    /// A one-pixel outline inside `rect` (fl_rect).
    fn rect(&mut self, rect: Rect, color: u32) {
        if rect.2 > 0 && rect.3 > 0 {
            self.ops.push(Op::Edge { rect, light: vec![color], dark: vec![color] });
        }
    }

    /// A line through the pixels from (x0, y0) to (x1, y1), both included
    /// (fl_line).
    fn line(&mut self, (x0, y0): (i64, i64), (x1, y1): (i64, i64), color: u32) {
        self.ops.push(Op::Line { from: (x0 as f64 + 0.5, y0 as f64 + 0.5), to: (x1 as f64 + 0.5, y1 as f64 + 0.5), color });
    }

    fn text(&mut self, rect: Rect, text: &str, font: Font, color: u32, place: Place) {
        if !text.is_empty() {
            self.ops.push(Op::Text { rect, text: text.to_string(), font, color, angle: 0, place });
        }
    }

    /// The points of an ellipse inscribed in `rect`, from `from` to `to`
    /// degrees (counter-clockwise from 3 o'clock, as fl_pie / fl_arc).
    fn ellipse((x, y, w, h): Rect, from: f64, to: f64) -> Vec<(f64, f64)> {
        let (cx, cy, rx, ry) = (x as f64 + w as f64 / 2.0, y as f64 + h as f64 / 2.0, w as f64 / 2.0, h as f64 / 2.0);
        let steps = ((rx.max(ry) * 4.0).ceil() as usize).clamp(12, 96);
        (0..=steps)
            .map(|k| {
                let a = (from + (to - from) * k as f64 / steps as f64).to_radians();
                (cx + rx * a.cos(), cy - ry * a.sin())
            })
            .collect()
    }

    /// A filled ellipse (fl_pie, the whole turn).
    fn pie(&mut self, rect: Rect, color: u32) {
        let mut points = Self::ellipse(rect, 0.0, 360.0);
        points.pop();
        self.ops.push(Op::Shape(Shape { points, fill: Some(color), stroke: Some(color) }));
    }

    /// An ellipse's outline from `from` to `to` degrees (fl_arc).
    fn arc(&mut self, rect: Rect, from: f64, to: f64, color: u32) {
        let points = Self::ellipse(rect, from, to);
        for pair in points.windows(2) {
            self.ops.push(Op::Line { from: pair[0], to: pair[1], color });
        }
    }

    /// A raised box in `bg` (the designer's buttons): lighter top / left,
    /// darker bottom / right.
    fn raised(&mut self, (x, y, w, h): Rect, bg: u32) {
        self.fill((x, y, w, h), bg);
        let (light, dark) = (shade(bg, 30), shade(bg, -85));
        self.line((x, y), (x + w - 1, y), light);
        self.line((x, y), (x, y + h - 1), light);
        self.line((x + w - 1, y), (x + w - 1, y + h - 1), dark);
        self.line((x, y + h - 1), (x + w - 1, y + h - 1), dark);
    }

    /// The selection's handles: the corners and the sides' middles.
    fn handles(&mut self, (x, y, w, h): Rect) {
        let half = HANDLE / 2;
        for (hx, hy) in [(x, y), (x + w, y), (x, y + h), (x + w, y + h), (x + w / 2, y), (x + w / 2, y + h), (x, y + h / 2), (x + w, y + h / 2)] {
            self.fill((hx - half, hy - half, HANDLE, HANDLE), 0x0000FF);
        }
    }

    /// One designed component, as its type's placeholder.
    fn component(&mut self, c: &DesignComp) {
        let (x, y, w, h) = c.bounds();
        let label = c.caption();
        let font_color = c.color("fontcolor").unwrap_or(BLACK);
        let grey = 0x828282;
        match c.type_name.to_ascii_uppercase().as_str() {
            "RBUTTON" => {
                self.raised((x, y, w, h), c.color("color").unwrap_or(0xE1E1E1));
                self.text((x, y, w, h), label, c.font(), font_color, Place::Center);
            }
            "RLABEL" => self.text((x + 2, y, w - 4, h), label, c.font(), font_color, Place::Left),
            "REDIT" => {
                self.fill((x, y, w, h), WHITE);
                self.line((x, y), (x + w - 1, y), grey);
                self.line((x, y), (x, y + h - 1), grey);
                self.line((x + w - 1, y), (x + w - 1, y + h - 1), 0xF5F5F5);
                self.line((x, y + h - 1), (x + w - 1, y + h - 1), 0xF5F5F5);
                self.text((x + 4, y, w - 8, h), c.prop("text").unwrap_or(&c.name), sans(12), BLACK, Place::Left);
            }
            "RCHECKBOX" => {
                let (bx, by) = (x + 2, y + (h - 13) / 2);
                self.fill((bx, by, 13, 13), WHITE);
                self.rect((bx, by, 13, 13), grey);
                if c.on("checked") {
                    self.line((bx + 2, by + 6), (bx + 5, by + 10), BLACK);
                    self.line((bx + 5, by + 10), (bx + 11, by + 2), BLACK);
                }
                self.text((x + 18, y, w - 20, h), label, sans(12), BLACK, Place::Left);
            }
            "RRADIOBUTTON" => {
                let ry = y + h / 2;
                self.pie((x + 2, ry - 6, 13, 13), WHITE);
                self.arc((x + 2, ry - 6, 13, 13), 0.0, 360.0, grey);
                self.text((x + 18, y, w - 20, h), label, sans(12), BLACK, Place::Left);
            }
            "RCOMBOBOX" => {
                self.fill((x, y, w - 18, h), WHITE);
                self.rect((x, y, w, h), grey);
                self.fill((x + w - 18, y + 1, 17, h - 2), 0xE1E1E1);
                let (ax, ay) = (x + w - 12, y + h / 2 - 1);
                for k in 0..3 {
                    self.line((ax - 3 + k, ay + k), (ax + 3 - k, ay + k), BLACK);
                }
                self.text((x + 4, y, w - 22, h), &c.name, sans(12), BLACK, Place::Left);
            }
            "RLISTBOX" => {
                self.fill((x, y, w, h), WHITE);
                self.rect((x, y, w, h), grey);
                self.text((x + 4, y + 2, w - 8, 16), "(ListBox)", sans(11), 0xB4B4B4, Place::Left);
            }
            kind @ ("RPANEL" | "RGROUPBOX") => {
                self.fill((x, y, w, h), c.color("color").unwrap_or(0xF0F0F0));
                if kind == "RGROUPBOX" {
                    let font = sans(11);
                    let tw = super::text::text_size(label, &font).0 + 8;
                    let line = 0xA0A0A0;
                    self.line((x, y + 8), (x + 6, y + 8), line);
                    self.line((x + 6 + tw, y + 8), (x + w - 1, y + 8), line);
                    self.line((x, y + 8), (x, y + h - 1), line);
                    self.line((x + w - 1, y + 8), (x + w - 1, y + h - 1), line);
                    self.line((x, y + h - 1), (x + w - 1, y + h - 1), line);
                    self.text((x + 10, y, tw, 16), label, font, BLACK, Place::Left);
                } else {
                    self.rect((x, y, w, h), 0xB4B4B4);
                }
            }
            "RPROGRESSBAR" => {
                self.fill((x, y, w, h), 0xE6E6E6);
                self.fill((x + 1, y + 1, w / 3, h - 2), 0x3C82C8);
                self.rect((x, y, w, h), 0xA0A0A0);
            }
            "RTIMER" => {
                self.fill((x, y, w, h), 0xF0F0FF);
                self.rect((x, y, w, h), 0x6464C8);
                self.text((x, y, w, h), &c.name, sans(10), 0x3C3CA0, Place::Center);
                self.text((x, y + h / 2 + 2, w, h / 2), "[Timer]", sans(8), 0x3C3CA0, Place::TopCenter);
            }
            "RRICHEDIT" | "RMEMO" => {
                self.fill((x, y, w, h), WHITE);
                self.rect((x, y, w, h), grey);
                self.text((x + 4, y + 2, w - 8, 16), "(RichEdit)", mono(11), 0xB4B4B4, Place::Left);
            }
            "RCANVAS" => {
                self.fill((x, y, w, h), c.color("color").unwrap_or(WHITE));
                self.rect((x, y, w, h), 0xA0A0A0);
                self.line((x + w / 2, y), (x + w / 2, y + h), 0xD2D2D2);
                self.line((x, y + h / 2), (x + w, y + h / 2), 0xD2D2D2);
                self.text((x, y, w, h), &c.name, sans(10), grey, Place::Center);
            }
            "RIMAGE" => {
                self.fill((x, y, w, h), 0xF5F5F5);
                self.rect((x, y, w, h), 0xB4B4B4);
                self.line((x, y), (x + w, y + h), 0xB4B4B4);
                self.line((x + w, y), (x, y + h), 0xB4B4B4);
                self.text((x, y, w, h), &c.name, sans(10), grey, Place::Center);
            }
            "RTREEVIEW" => {
                self.fill((x, y, w, h), WHITE);
                self.rect((x, y, w, h), grey);
                self.text((x + 6, y + 4, w - 12, 14), "+ Item 1", sans(10), 0x646464, Place::Left);
                self.text((x + 6, y + 18, w - 12, 14), "+ Item 2", sans(10), 0x646464, Place::Left);
            }
            "RTRACKBAR" => {
                let track = y + h / 2;
                self.fill((x + 4, track - 2, w - 8, 4), 0xB4B4B4);
                let thumb = x + w / 3;
                self.fill((thumb - 5, y + 4, 10, h - 8), 0xC8C8C8);
                self.rect((thumb - 5, y + 4, 10, h - 8), grey);
            }
            "RSTRINGGRID" => {
                self.fill((x, y, w, h), WHITE);
                self.fill((x, y, w, 20), 0xC8D2E6);
                self.rect((x, y, w, h), 0xA0A0A0);
                self.line((x + w / 2, y), (x + w / 2, y + h), 0xA0A0A0);
                for row in 0..4 {
                    self.line((x, y + row * 20), (x + w, y + row * 20), 0xA0A0A0);
                }
                self.text((x + 2, y + 2, w - 4, 16), &c.name, sans(10), BLACK, Place::Left);
            }
            kind @ ("RMYSQL" | "RSQLITE") => {
                self.fill((x, y, w, h), 0xFFF5E6);
                self.rect((x, y, w, h), 0xB48C50);
                self.text((x, y + 2, w, h / 2), if kind == "RMYSQL" { "MySQL" } else { "SQLite" }, sans(10), 0x78501E, Place::Center);
                self.text((x, y + h / 2, w, h / 2), &c.name, sans(9), 0x78501E, Place::Center);
            }
            "RCOOLBTN" => {
                if c.on("down") {
                    self.fill((x, y, w, h), 0xC8D2E6);
                    self.rect((x, y, w, h), grey);
                } else if !c.on("flat") {
                    self.raised((x, y, w, h), 0xE1E1E1);
                } else {
                    self.fill((x, y, w, h), 0xF0F0F0);
                }
                self.text((x, y, w, h), label, c.font(), font_color, Place::Center);
            }
            "ROVALBTN" => {
                self.pie((x, y, w, h), c.color("color").unwrap_or(0xDCDCDC));
                self.arc((x, y, w, h), 45.0, 225.0, WHITE);
                self.arc((x, y, w, h), 225.0, 405.0, 0x808080);
                self.text((x, y, w, h), label, c.font(), font_color, Place::Center);
            }
            _ => {
                self.fill((x, y, w, h), 0xECECEC);
                self.rect((x, y, w, h), 0xA0A0A0);
                self.text((x + 2, y + 2, w - 4, h - 4), label, sans(11), BLACK, Place::Center);
                self.text((x + 2, y + 1, w - 4, 12), &c.type_name, sans(9), 0x646464, Place::TopLeft);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn surface() -> DesignSurface {
        let mut d = DesignSurface::default();
        d.call("addcomponent", &[v_str("RBUTTON"), v_str("Button1"), v_int(16), v_int(16), v_int(80), v_int(24)]);
        d.call("addcomponent", &[v_str("RLABEL"), v_str("Label1"), v_int(48), v_int(32), v_int(64), v_int(16)]);
        d
    }

    #[test]
    fn methods_and_properties_answer_as_the_designer_expects() {
        let mut d = surface();
        assert_eq!(d.get("compcount"), Some(v_int(2)));
        assert_eq!(d.selected, 1, "the newest is selected");
        assert_eq!(d.call("getname", &[v_int(0)]), Some(v_str("Button1")));
        assert_eq!(d.call("gettype", &[v_int(1)]), Some(v_str("RLABEL")));
        assert_eq!(d.call("getprop", &[v_int(0), v_str("Caption")]), Some(v_str("Button1")), "Caption starts as the name");
        d.call("setprop", &[v_int(0), v_str("Color"), v_str("&H0000FF")]);
        assert_eq!(d.call("getprop", &[v_int(0), v_str("color")]), Some(v_str("&H0000FF")));
        assert_eq!(d.components[0].color("color"), Some(0xFF0000), "RapidQ's BGR");
        assert_eq!(d.call("getprop", &[v_int(5), v_str("color")]), Some(v_str("")));
        d.call("setcompbounds", &[v_int(1), v_int(1), v_int(2)]);
        assert_eq!(d.components[1].bounds(), (1, 2, 80, 25), "missing sizes default");
        d.call("setname", &[v_int(1), v_str("Title")]);
        assert_eq!(d.call("getname", &[v_int(1)]), Some(v_str("Title")));
        assert_eq!(d.call("getcompw", &[v_int(9)]), Some(v_int(0)));
        d.call("selectcomp", &[v_int(7)]);
        assert_eq!((d.selected, d.selection()), (7, None), "any number is kept, nothing selected");
        d.call("removecomponent", &[v_int(0)]);
        assert_eq!((d.get("count"), d.selected), (Some(v_int(1)), -1));
        assert!(d.set("formcaption", &v_str("Main")));
        assert_eq!(d.get("formcaption"), Some(v_str("Main")));
        d.call("clearall", &[]);
        assert_eq!(d.call("count", &[]), Some(v_int(0)));
        assert_eq!(d.call("show", &[]), None, "the host's");
    }

    #[test]
    fn the_mouse_selects_moves_and_resizes_on_the_grid() {
        let mut d = surface();
        // a press on the label (on top), then on the button below it
        assert_eq!(d.mouse_down(50, 34, false), Some(DesignEvent::Select(1)));
        d.mouse_up();
        assert_eq!(d.mouse_down(20, 20, false), Some(DesignEvent::Select(0)));
        // dragged by (+13, +6): snapped to the grid
        assert_eq!(d.mouse_drag(33, 26), Some(DesignEvent::Move { index: 0, x: 32, y: 24, w: 80, h: 24 }));
        assert_eq!(d.mouse_drag(-50, 26), Some(DesignEvent::Move { index: 0, x: 0, y: 24, w: 80, h: 24 }), "kept on the surface");
        d.mouse_up();
        assert_eq!(d.mouse_drag(40, 40), None, "no drag once let go");
        // the corner handle (no event for the press), then a resize
        assert_eq!(d.grip_at(83, 47), Some(Grip::Corner));
        assert_eq!(d.mouse_down(83, 47, false), None);
        assert_eq!(d.mouse_drag(101, 61), Some(DesignEvent::Move { index: 0, x: 0, y: 24, w: 104, h: 40 }));
        assert_eq!(d.mouse_drag(2, 2), Some(DesignEvent::Move { index: 0, x: 0, y: 24, w: 16, h: 16 }), "never smaller than 16");
        d.mouse_up();
        // the right and bottom middles
        assert_eq!(d.grip_at(16, 32), Some(Grip::Right));
        assert_eq!(d.grip_at(8, 40), Some(Grip::Bottom));
        assert_eq!(d.grip_at(60, 60), None);
        // a double click, then the background
        assert_eq!(d.mouse_down(5, 30, true), Some(DesignEvent::DblClick(0)));
        d.mouse_up();
        assert_eq!(d.mouse_down(300, 200, false), Some(DesignEvent::BgClick(300, 200)));
        assert_eq!(d.selection(), None);
        assert_eq!(d.mouse_drag(310, 210), None);
        assert_eq!(DesignEvent::Move { index: 2, x: 1, y: 2, w: 3, h: 4 }.args(), vec![2, 1, 2, 3, 4]);
        assert_eq!(DesignEvent::BgClick(5, 6).event(), "onbgclick");
    }

    #[test]
    fn colours_parse_every_way_the_designer_stores_them() {
        assert_eq!(parse_color("#102030"), Some(0x102030));
        assert_eq!(parse_color("rgb(1, 2, 3)"), Some(0x010203));
        assert_eq!(parse_color("&HFF0000"), Some(0x0000FF));
        assert_eq!(parse_color("255"), Some(0xFF0000), "a QCOLORDIALOG's Color: red");
        assert_eq!(parse_color("-5"), None);
        assert_eq!(parse_color("blue"), None);
    }

    #[test]
    fn the_drawing_is_fltks_placeholders_with_the_selection_on_top() {
        let mut d = surface();
        d.call("addcomponent", &[v_str("RQUUX"), v_str("Odd1"), v_int(100), v_int(100), v_int(60), v_int(40)]);
        let ops = d.ops(64, 40);
        assert_eq!(ops[0], Op::Fill { rect: (0, 0, 64, 40), color: WHITE });
        let dots = ops.iter().filter(|o| matches!(o, Op::Fill { rect: (_, _, 1, 1), color: 0xC8C8C8 })).count();
        assert_eq!(dots, 8 * 5, "a dot every 8 pixels");
        let texts: Vec<&str> = ops.iter().filter_map(|o| if let Op::Text { text, .. } = o { Some(text.as_str()) } else { None }).collect();
        assert_eq!(texts, ["Button1", "Label1", "Odd1", "RQUUX"]);
        let handles = ops.iter().filter(|o| matches!(o, Op::Fill { rect: (_, _, 5, 5), color: 0x0000FF })).count();
        assert_eq!(handles, 8, "only the selected one's");
        assert!(matches!(ops.last(), Some(Op::Fill { rect: (158, 118, 5, 5), .. })), "the handles last: {:?}", ops.last());
        // a set font size is in points, as the designed program shows it
        d.call("setprop", &[v_int(0), v_str("font.size"), v_str("10")]);
        assert_eq!(d.components[0].font().pixel_size(), 13);
        assert_eq!(d.components[1].font().pixel_size(), 12);
    }
}
