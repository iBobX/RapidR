//! The colour editor (docs/code-helpers-plan.md H1): a hue wheel around a
//! saturation-and-brightness square, the Red, Green, Blue and Hex fields,
//! the colour's name in words (and its constant's), the old colour beside
//! the new, the project's own colours and the standard ones as a palette.
//! Colour is never the only cue: the numbers and the name are always there.
//!
//! Keys: Tab / Shift+Tab through the wheel, the square, the fields, the
//! palette and the buttons; on the wheel Left / Right (Up / Down) turn the
//! hue a degree (Shift: ten, Page Up / Down: thirty, Home / End: red /
//! almost red again); on the square Left / Right the saturation, Up / Down
//! the brightness (Shift: ten); in a field type the number; on the palette
//! the arrows and Space pick; Enter keeps the colour, Esc drops it.

use std::cell::RefCell;
use std::sync::Arc;

use rapidr_value::objects::a11y::{node_id, AccessNode, Numeric, Role};
use rapidr_value::objects::codehelp::{color_said, color_words};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::{Place, Rect};
use rapidr_value::objects::text::text_size;
use rapidr_value::panels::inspector::values::{color_hex, color_name, color_value, is_system_color, STANDARD_COLORS};

use super::field::{Accept, Field};
use super::{button, Look, Picked};
use crate::display::Picture;
use crate::paint::Painter;

pub const WIDTH: i64 = 340;
const WHEEL: i64 = 160;
const RING: f64 = 15.0;
const CELL: i64 = 24;
const PER_ROW: usize = 10;

/// The parts the focus moves through, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Wheel,
    Square,
    Field(usize),
    Palette,
    Cancel,
    Apply,
}

const ORDER: [Part; 9] = [Part::Wheel, Part::Square, Part::Field(0), Part::Field(1), Part::Field(2), Part::Field(3), Part::Palette, Part::Cancel, Part::Apply];

#[derive(Clone, Debug, PartialEq)]
pub struct ColorEd {
    /// The colour it opened with (RapidQ's BGR, or a system colour's).
    pub original: i64,
    /// The colour now.
    pub value: i64,
    pub h: f64,
    pub s: f64,
    pub v: f64,
    /// Red, Green, Blue, Hex.
    pub fields: Vec<Field>,
    /// The palette: the project's colours, then the standard ones.
    pub palette: Vec<i64>,
    pub project: usize,
    pub palette_at: usize,
    pub focus: usize,
    pub drag: Option<Part>,
}

/// 0xRRGGBB of RapidQ's BGR.
pub fn rgb_of(bgr: i64) -> u32 {
    let b = bgr as u32;
    ((b & 0xFF) << 16) | (b & 0xFF00) | ((b >> 16) & 0xFF)
}

fn bgr_of(rgb: u32) -> i64 {
    i64::from(((rgb & 0xFF) << 16) | (rgb & 0xFF00) | ((rgb >> 16) & 0xFF))
}

pub fn hsv_to_rgb(h: f64, s: f64, v: f64) -> u32 {
    let h = h.rem_euclid(360.0) / 60.0;
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    let ch = |f: f64| ((f + m) * 255.0).round().clamp(0.0, 255.0) as u32;
    (ch(r) << 16) | (ch(g) << 8) | ch(b)
}

pub fn rgb_to_hsv(rgb: u32) -> (f64, f64, f64) {
    let (r, g, b) = (((rgb >> 16) & 0xFF) as f64 / 255.0, ((rgb >> 8) & 0xFF) as f64 / 255.0, (rgb & 0xFF) as f64 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (h, if max == 0.0 { 0.0 } else { d / max }, max)
}

impl ColorEd {
    /// An editor of `value`, its palette the project's colours first.
    pub fn new(value: i64, project: &[i64]) -> ColorEd {
        let mut palette: Vec<i64> = Vec::new();
        for &c in project {
            if !palette.contains(&c) && palette.len() < PER_ROW {
                palette.push(c);
            }
        }
        let n_project = palette.len();
        palette.extend(STANDARD_COLORS.iter().filter_map(|n| color_value(n)));
        let shown = Self::shown_of(value);
        let (h, s, v) = rgb_to_hsv(shown);
        let mut e = ColorEd {
            original: value,
            value,
            h,
            s,
            v,
            fields: vec![Field::new("Red", "", Accept::Digits(3)), Field::new("Green", "", Accept::Digits(3)), Field::new("Blue", "", Accept::Digits(3)), Field::new("Hex", "", Accept::Hex)],
            palette,
            project: n_project,
            palette_at: 0,
            focus: 0,
            drag: None,
        };
        e.palette_at = e.palette.iter().position(|&c| c == value).unwrap_or(0);
        e.sync_fields();
        e
    }

    /// The colour a value shows as (a system colour: the theme's).
    fn shown_of(value: i64) -> u32 {
        if is_system_color(value) {
            rapidr_value::theme::current().system_color((value as u32) & 0xFF)
        } else {
            rgb_of(value)
        }
    }

    pub fn part(&self) -> Part {
        ORDER[self.focus.min(ORDER.len() - 1)]
    }

    fn sync_fields(&mut self) {
        let rgb = Self::shown_of(self.value);
        let vals = [(rgb >> 16) & 0xFF, (rgb >> 8) & 0xFF, rgb & 0xFF];
        for (k, v) in vals.iter().enumerate() {
            self.fields[k].set(&v.to_string());
        }
        self.fields[3].set(&format!("#{rgb:06X}"));
    }

    /// The wheel or the square moved: the colour from them.
    fn from_hsv(&mut self) {
        self.value = bgr_of(hsv_to_rgb(self.h, self.s, self.v));
        self.sync_fields();
    }

    /// A colour set (a field typed, a swatch picked): the wheel follows.
    pub fn set_value(&mut self, value: i64) {
        self.value = value;
        let (h, s, v) = rgb_to_hsv(Self::shown_of(value));
        // (grey has no hue: the wheel keeps where it was)
        if s > 0.0 {
            self.h = h;
        }
        self.s = s;
        self.v = v;
        self.sync_fields();
    }

    /// A field's text taken (as it is typed).
    fn from_field(&mut self, k: usize) {
        let t = self.fields[k].text.trim().to_string();
        if k == 3 {
            let hex = t.trim_start_matches('#');
            if hex.len() == 6 {
                if let Ok(rgb) = u32::from_str_radix(hex, 16) {
                    let keep = self.fields[3].clone();
                    self.set_value(bgr_of(rgb));
                    self.fields[3] = keep;
                }
            }
            return;
        }
        let Ok(n) = t.parse::<u32>() else { return };
        let rgb = Self::shown_of(self.value);
        let shift = [16, 8, 0][k];
        let rgb = (rgb & !(0xFF << shift)) | (n.min(255) << shift);
        let keep = self.fields[k].clone();
        self.set_value(bgr_of(rgb));
        self.fields[k] = keep;
    }

    /// The colour's words: its constant's name, else its hex, and what
    /// colour it is ("clRed · red", "&H1482FF · orange").
    pub fn name_line(&self) -> String {
        let words = color_words(Self::shown_of(self.value));
        match color_name(self.value) {
            Some(n) => format!("{n} · {words}"),
            None => format!("{} · {words}", color_hex(self.value)),
        }
    }

    pub fn height(&self) -> i64 {
        let rows = 2 + i64::from(self.project > 0);
        46 + WHEEL + 14 + rows * CELL + 12 + 30 + 12
    }

    // ---- geometry (relative to the popover's top left) ----

    fn wheel_rect(&self) -> Rect {
        (12, 46, WHEEL, WHEEL)
    }

    fn square_rect(&self) -> Rect {
        let inner = WHEEL as f64 / 2.0 - RING - 4.0;
        let side = (inner * std::f64::consts::SQRT_2).floor() as i64;
        let c = 12 + WHEEL / 2;
        (c - side / 2, 46 + WHEEL / 2 - side / 2, side, side)
    }

    fn field_rect(&self, k: usize) -> Rect {
        (12 + WHEEL + 16, 54 + k as i64 * 34, WIDTH - (12 + WHEEL + 16) - 12, 28)
    }

    /// Swatch `i`'s cell.
    fn cell(&self, i: usize) -> Rect {
        let top = 46 + WHEEL + 14;
        let (row, col) = if i < self.project {
            (0, i)
        } else {
            let k = i - self.project;
            (i64::from(self.project > 0) as usize + k / 8, k % 8)
        };
        (12 + 70 + col as i64 * CELL, top + row as i64 * CELL, CELL, CELL)
    }

    fn buttons(&self, h: i64) -> (Rect, Rect) {
        let y = h - 12 - 30;
        ((WIDTH - 12 - 84 - 8 - 84, y, 84, 30), (WIDTH - 12 - 84, y, 84, 30))
    }

    // ---- input ----

    /// A key: what it did.
    pub fn key(&mut self, vk: i64, text: &str, shift: bool) -> Option<Picked> {
        let step = if shift { 10.0 } else { 1.0 };
        match (self.part(), vk) {
            (Part::Wheel, 37 | 40) => self.h = (self.h - step).rem_euclid(360.0),
            (Part::Wheel, 39 | 38) => self.h = (self.h + step).rem_euclid(360.0),
            (Part::Wheel, 33) => self.h = (self.h + 30.0).rem_euclid(360.0),
            (Part::Wheel, 34) => self.h = (self.h - 30.0).rem_euclid(360.0),
            (Part::Wheel, 36) => self.h = 0.0,
            (Part::Wheel, 35) => self.h = 359.0,
            (Part::Square, 37) => self.s = (self.s - step / 100.0).max(0.0),
            (Part::Square, 39) => self.s = (self.s + step / 100.0).min(1.0),
            (Part::Square, 38) => self.v = (self.v + step / 100.0).min(1.0),
            (Part::Square, 40) => self.v = (self.v - step / 100.0).max(0.0),
            (Part::Field(k), 38 | 40) if k < 3 => {
                let n: i64 = self.fields[k].text.parse().unwrap_or(0);
                let n = (n + if vk == 38 { step as i64 } else { -(step as i64) }).clamp(0, 255);
                self.fields[k].set(&n.to_string());
                self.from_field(k);
                return Some(Picked::Nothing);
            }
            (Part::Field(k), _) => {
                return match self.fields[k].key(vk, text) {
                    Some(true) => {
                        self.from_field(k);
                        Some(Picked::Nothing)
                    }
                    Some(false) => Some(Picked::Nothing),
                    None => None,
                };
            }
            (Part::Palette, 37 | 39 | 38 | 40) => {
                let n = self.palette.len();
                let d: isize = match vk {
                    37 => -1,
                    39 => 1,
                    38 => -(PER_ROW.min(8) as isize),
                    _ => PER_ROW.min(8) as isize,
                };
                self.palette_at = (self.palette_at as isize + d).clamp(0, n as isize - 1) as usize;
                return Some(Picked::Nothing);
            }
            (Part::Palette, 32) => {
                let c = self.palette[self.palette_at];
                self.set_value(c);
                return Some(Picked::Nothing);
            }
            (Part::Cancel, 32 | 13) => return Some(Picked::Cancel),
            _ => return None,
        }
        self.from_hsv();
        Some(Picked::Nothing)
    }

    /// The focus to the next (previous) part: false past the ends.
    pub fn tab(&mut self, back: bool) {
        let n = ORDER.len();
        self.focus = if back { (self.focus + n - 1) % n } else { (self.focus + 1) % n };
        if let Part::Field(k) = self.part() {
            self.fields[k].focus();
        }
    }

    /// A press at (x, y) (relative to the popover), height `h`.
    pub fn mouse_down(&mut self, x: i64, y: i64, h: i64) -> Option<Picked> {
        let inside = |r: Rect| x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3;
        let (cancel, apply) = self.buttons(h);
        if inside(apply) {
            return Some(Picked::Color(self.value));
        }
        if inside(cancel) {
            return Some(Picked::Cancel);
        }
        for k in 0..4 {
            if inside(self.field_rect(k)) {
                self.focus = 2 + k;
                self.fields[k].focus();
                return Some(Picked::Nothing);
            }
        }
        for i in 0..self.palette.len() {
            if inside(self.cell(i)) {
                self.palette_at = i;
                self.focus = ORDER.iter().position(|p| *p == Part::Palette).unwrap_or(6);
                let c = self.palette[i];
                self.set_value(c);
                return Some(Picked::Nothing);
            }
        }
        let sq = self.square_rect();
        if inside(sq) {
            self.focus = 1;
            self.drag = Some(Part::Square);
            self.drag_to(x, y);
            return Some(Picked::Nothing);
        }
        let wr = self.wheel_rect();
        let (cx, cy) = (wr.0 as f64 + WHEEL as f64 / 2.0, wr.1 as f64 + WHEEL as f64 / 2.0);
        let d = ((x as f64 - cx).powi(2) + (y as f64 - cy).powi(2)).sqrt();
        if d <= WHEEL as f64 / 2.0 && d >= WHEEL as f64 / 2.0 - RING - 2.0 {
            self.focus = 0;
            self.drag = Some(Part::Wheel);
            self.drag_to(x, y);
            return Some(Picked::Nothing);
        }
        None
    }

    /// The mouse dragging the wheel's or the square's marker.
    pub fn drag_to(&mut self, x: i64, y: i64) {
        match self.drag {
            Some(Part::Square) => {
                let r = self.square_rect();
                self.s = ((x - r.0) as f64 / r.2 as f64).clamp(0.0, 1.0);
                self.v = 1.0 - ((y - r.1) as f64 / r.3 as f64).clamp(0.0, 1.0);
            }
            Some(Part::Wheel) => {
                let wr = self.wheel_rect();
                let (cx, cy) = (wr.0 as f64 + WHEEL as f64 / 2.0, wr.1 as f64 + WHEEL as f64 / 2.0);
                // (red at the top, the hue turning clockwise)
                self.h = (x as f64 - cx).atan2(-(y as f64 - cy)).to_degrees().rem_euclid(360.0);
            }
            _ => return,
        }
        self.from_hsv();
    }

    // ---- drawing ----

    pub fn paint(&self, p: &mut Painter, origin: (i64, i64), h: i64, look: &Look, font: &Font) {
        let (ox, oy) = origin;
        let at = |r: Rect| (ox + r.0, oy + r.1, r.2, r.3);
        let th = text_size("Ag", font).1;
        // the old colour beside the new, the name
        let old = Self::shown_of(self.original);
        let new = Self::shown_of(self.value);
        p.round((ox + 12, oy + 12, 30, 24), 3.0, Some(old), Some(look.border_strong), 1.0);
        p.round((ox + 42, oy + 12, 30, 24), 3.0, Some(new), Some(look.border_strong), 1.0);
        let bold = Font { styles: 1, ..font.clone() };
        p.text((ox + 82, oy + 10, WIDTH - 94, th + 2), &self.name_line(), &bold, look.text, Place::TopLeft);
        p.text((ox + 82, oy + 12 + th, WIDTH - 94, th + 2), "was · now", font, look.dim, Place::TopLeft);
        // the wheel and the square
        let scale = p.scale();
        let wr = at(self.wheel_rect());
        let side = ((WHEEL as f64) * scale).round() as usize;
        p.picture(&format!("value-edit/ring/{side}"), 1, ring(side, (RING * scale).round()), wr);
        let sq = at(self.square_rect());
        let sside = ((sq.2 as f64) * scale).round() as usize;
        let hue = self.h.round() as u32 % 360;
        p.picture(&format!("value-edit/square/{sside}/{hue}"), 1, square(sside, hue), sq);
        p.frame(sq, look.border_strong);
        // (the markers: a ring with a dark and a light edge reads on any colour)
        let (cx, cy) = (wr.0 as f64 + WHEEL as f64 / 2.0, wr.1 as f64 + WHEEL as f64 / 2.0);
        let rm = WHEEL as f64 / 2.0 - RING / 2.0;
        let a = self.h.to_radians();
        marker(p, cx + rm * a.sin(), cy - rm * a.cos(), self.part() == Part::Wheel, look);
        marker(p, sq.0 as f64 + self.s * sq.2 as f64, sq.1 as f64 + (1.0 - self.v) * sq.3 as f64, self.part() == Part::Square, look);
        if self.part() == Part::Wheel {
            p.round((wr.0 - 2, wr.1 - 2, wr.2 + 4, wr.3 + 4), WHEEL as f64 / 2.0 + 2.0, None, Some(look.focus), 2.0);
        }
        // the fields
        for (k, f) in self.fields.iter().enumerate() {
            f.paint(p, at(self.field_rect(k)), 46, self.part() == Part::Field(k), look, font);
        }
        // the palette
        let top = oy + 46 + WHEEL + 14;
        if self.project > 0 {
            p.text((ox + 12, top + (CELL - th) / 2, 68, th), "Project", font, look.dim, Place::TopLeft);
        }
        p.text((ox + 12, top + CELL * i64::from(self.project > 0) + (CELL - th) / 2, 68, th), "Standard", font, look.dim, Place::TopLeft);
        for (i, &c) in self.palette.iter().enumerate() {
            let r = at(self.cell(i));
            p.round((r.0 + 2, r.1 + 2, r.2 - 4, r.3 - 4), 3.0, Some(Self::shown_of(c)), Some(look.border_strong), 1.0);
            if c == self.value {
                p.round((r.0, r.1, r.2, r.3), 4.0, None, Some(look.text), 1.5);
            }
            if self.part() == Part::Palette && i == self.palette_at {
                p.round((r.0 - 1, r.1 - 1, r.2 + 2, r.3 + 2), 4.0, None, Some(look.focus), 2.0);
            }
        }
        let (cancel, apply) = self.buttons(h);
        button(p, at(cancel), "Cancel", false, self.part() == Part::Cancel, look, font);
        button(p, at(apply), "Apply", true, self.part() == Part::Apply, look, font);
    }

    /// Its parts for screen readers (ids under `base`), in `origin`'s place.
    pub fn access(&self, base: &str, origin: (i64, i64)) -> Vec<AccessNode> {
        let mut out = Vec::new();
        let node = |k: &str, role: Role, name: String| {
            let mut n = AccessNode::new(node_id(&format!("{base}${k}")), role);
            n.name = name;
            n.bounds = (origin.0, origin.1, 0, 0);
            n
        };
        let mut wheel = node("hue", Role::Slider, "Hue".into());
        wheel.numeric = Some(Numeric { value: self.h.round(), min: 0.0, max: 359.0, step: 1.0, jump: 30.0 });
        wheel.value = Some(format!("{} degrees", self.h.round()));
        wheel.states.focused = self.part() == Part::Wheel;
        out.push(wheel);
        let mut sq = node("sv", Role::Slider, "Saturation and brightness".into());
        sq.value = Some(format!("saturation {}, brightness {}", (self.s * 100.0).round(), (self.v * 100.0).round()));
        sq.description = "Left and Right: saturation; Up and Down: brightness".into();
        sq.states.focused = self.part() == Part::Square;
        out.push(sq);
        for (k, f) in self.fields.iter().enumerate() {
            let mut n = node(&format!("field{k}"), Role::TextInput, f.label.to_string());
            n.value = Some(f.text.clone());
            n.states.focused = self.part() == Part::Field(k);
            out.push(n);
        }
        let mut list = node("palette", Role::ListBox, "Palette".into());
        for (i, &c) in self.palette.iter().enumerate() {
            let mut o = node(&format!("palette{i}"), Role::ListBoxOption, color_said(c));
            o.states.selected = Some(i == self.palette_at);
            o.states.focused = self.part() == Part::Palette && i == self.palette_at;
            list.children.push(o);
        }
        out.push(list);
        let mut cancel = node("cancel", Role::Button, "Cancel".into());
        cancel.states.focused = self.part() == Part::Cancel;
        out.push(cancel);
        let mut apply = node("apply", Role::Button, "Apply".into());
        apply.states.focused = self.part() == Part::Apply;
        out.push(apply);
        out
    }

    /// What it shows, in words (the editor's text for tests and the
    /// dialog's description): "orange, 255 130 20 (was blue, 0 0 255)".
    pub fn said(&self) -> String {
        format!("{} (was {})", color_said(self.value), color_said(self.original))
    }
}

/// A marker: a ring, dark outside and light inside (focused: thicker).
fn marker(p: &mut Painter, x: f64, y: f64, focused: bool, look: &Look) {
    let r = if focused { 7 } else { 6 };
    let (xi, yi) = (x.round() as i64, y.round() as i64);
    p.round((xi - r, yi - r, 2 * r, 2 * r), r as f64, None, Some(0x000000), 2.0);
    p.round((xi - r + 2, yi - r + 2, 2 * r - 4, 2 * r - 4), (r - 2) as f64, None, Some(0xFFFFFF), 1.5);
    if focused {
        p.round((xi - r - 3, yi - r - 3, 2 * r + 6, 2 * r + 6), (r + 3) as f64, None, Some(look.focus), 2.0);
    }
}

thread_local! {
    static RING_PIC: RefCell<Option<(usize, Arc<Picture>)>> = const { RefCell::new(None) };
    static SQUARE: RefCell<Option<((usize, u32), Arc<Picture>)>> = const { RefCell::new(None) };
}

/// The hue ring `side` device pixels across, `width` wide (made once a
/// size): red at the top, the hues clockwise, its edges smooth.
fn ring(side: usize, width: f64) -> Arc<Picture> {
    RING_PIC.with(|r| {
        if let Some((s, pic)) = r.borrow().as_ref().filter(|(s, _)| *s == side) {
            let _ = s;
            return pic.clone();
        }
        let mut rgba = vec![0u8; side * side * 4];
        let c = side as f64 / 2.0;
        let (outer, inner) = (c - 0.5, c - 0.5 - width);
        for y in 0..side {
            for x in 0..side {
                let (dx, dy) = (x as f64 + 0.5 - c, y as f64 + 0.5 - c);
                let d = (dx * dx + dy * dy).sqrt();
                let a = (outer - d + 0.5).clamp(0.0, 1.0) * (d - inner + 0.5).clamp(0.0, 1.0);
                if a <= 0.0 {
                    continue;
                }
                let hue = dx.atan2(-dy).to_degrees().rem_euclid(360.0);
                let col = hsv_to_rgb(hue, 1.0, 1.0);
                let k = (y * side + x) * 4;
                rgba[k] = (col >> 16) as u8;
                rgba[k + 1] = (col >> 8) as u8;
                rgba[k + 2] = col as u8;
                rgba[k + 3] = (a * 255.0).round() as u8;
            }
        }
        let pic = Arc::new(Picture { width: side, height: side, rgba });
        *r.borrow_mut() = Some((side, pic.clone()));
        pic
    })
}

/// The saturation (across) and brightness (down) square of hue `hue`.
fn square(side: usize, hue: u32) -> Arc<Picture> {
    SQUARE.with(|q| {
        if let Some((_, pic)) = q.borrow().as_ref().filter(|(k, _)| *k == (side, hue)) {
            return pic.clone();
        }
        let side = side.max(1);
        let mut rgba = vec![0u8; side * side * 4];
        for y in 0..side {
            let v = 1.0 - y as f64 / (side - 1).max(1) as f64;
            for x in 0..side {
                let s = x as f64 / (side - 1).max(1) as f64;
                let col = hsv_to_rgb(f64::from(hue), s, v);
                let k = (y * side + x) * 4;
                rgba[k] = (col >> 16) as u8;
                rgba[k + 1] = (col >> 8) as u8;
                rgba[k + 2] = col as u8;
                rgba[k + 3] = 255;
            }
        }
        let pic = Arc::new(Picture { width: side, height: side, rgba });
        *q.borrow_mut() = Some(((side, hue), pic.clone()));
        pic
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsv_round_trips() {
        for rgb in [0xFF8214u32, 0x0000FF, 0x123456, 0xFFFFFF, 0] {
            let (h, s, v) = rgb_to_hsv(rgb);
            assert_eq!(hsv_to_rgb(h, s, v), rgb, "{rgb:06X}");
        }
    }

    #[test]
    fn keys_and_fields_edit_the_colour() {
        let orange = bgr_of(0xFF8214);
        let mut e = ColorEd::new(orange, &[255]);
        assert_eq!(e.fields[0].text, "255");
        assert_eq!(e.fields[3].text, "#FF8214");
        assert!(e.name_line().ends_with("orange"));
        // (Tab to Blue, type 255: a pinkish orange)
        e.tab(false);
        e.tab(false);
        e.tab(false);
        e.tab(false);
        assert_eq!(e.part(), Part::Field(2));
        e.key(0, "2", false);
        e.key(0, "5", false);
        e.key(0, "5", false);
        assert_eq!(rgb_of(e.value), 0xFF82FF);
        // (Hex: pure blue)
        e.tab(false);
        for c in "#0000FF".chars() {
            e.key(0, &c.to_string(), false);
        }
        assert_eq!(e.value, 0xFF0000, "RapidQ's BGR blue");
        assert_eq!(e.fields[2].text, "255");
        // (the palette: the project's colour first — clRed)
        e.tab(false);
        assert_eq!(e.part(), Part::Palette);
        e.palette_at = 0;
        e.key(32, " ", false);
        assert_eq!(e.value, 255);
        assert!(e.name_line().starts_with("clRed"));
        assert!(e.said().starts_with("clRed, red, 255 0 0 (was"));
    }

    #[test]
    fn the_wheel_turns_by_keys() {
        let mut e = ColorEd::new(bgr_of(0xFF0000), &[]);
        e.key(39, "", true);
        assert!((e.h - 10.0).abs() < 0.01);
        e.key(36, "", false);
        assert_eq!(e.h, 0.0);
        e.focus = 1;
        e.key(40, "", true);
        assert!((e.v - 0.9).abs() < 0.01);
    }
}
