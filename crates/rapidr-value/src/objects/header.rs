//! QHEADER (manual, Appendix A): column headers the user resizes and
//! clicks. Its surface is a canvas (`Sender.TextOut`, `FillRect`, … draw on
//! it, as on a QCANVAS); this model keeps the sections, lays them out, turns
//! the mouse into RapidQ's events, and paints each section's face — an
//! owner-drawn one (`Style = hsOwnerDraw`) then gets OnDrawSection(Index,
//! Pressed, Rect) to draw on it. The same on every runtime.

use super::bitmap::Bitmap;
use super::font::Font;
use crate::{v_int, v_str, Value};

pub const HS_TEXT: i64 = 0;
pub const HS_OWNER_DRAW: i64 = 1;
/// OnSectionTrack's State: tsTrackBegin, tsTrackMove, tsTrackEnd.
pub const TS_BEGIN: i64 = 0;
pub const TS_MOVE: i64 = 1;
pub const TS_END: i64 = 2;
/// How near a section's right edge (pixels) the mouse resizes it.
const GRIP: i64 = 3;

/// Whether method `m` changes the sections (the header is painted again;
/// a drawing on it is not).
pub fn changes_sections(m: &str) -> bool {
    matches!(m, "addsections" | "addsection" | "clear") || (m.starts_with("section") && m.ends_with('='))
}

#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    pub caption: String,
    pub width: i64,
    pub min_width: i64,
    pub max_width: i64,
    /// taLeftJustify 0, taRightJustify 1, taCenter 2.
    pub alignment: i64,
    pub allow_click: bool,
    pub style: i64,
}

impl Section {
    fn new(caption: String) -> Section {
        Section { caption, width: 50, min_width: 0, max_width: 10_000, alignment: 0, allow_click: true, style: HS_TEXT }
    }
}

/// An owner-drawn section to draw: (index, pressed, (left, top, right,
/// bottom)) — OnDrawSection's arguments.
pub type OwnerDrawn = (usize, bool, (i64, i64, i64, i64));

/// What the user did, for the runtime to fire.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// OnSectionClick(Index).
    Click(usize),
    /// OnSectionTrack(Index, Width, State); at the end OnSectionResize(Index).
    Track(usize, i64, i64),
}

#[derive(Debug, Clone, Default)]
pub struct Header {
    pub sections: Vec<Section>,
    pub hot_track: bool,
    /// The section held down (drawn pressed).
    pub pressed: Option<usize>,
    /// The section whose edge is dragged: (index, where the drag began,
    /// its width then).
    drag: Option<(usize, i64, i64)>,
}

impl Header {
    /// Each section's left and right edge (right excluded).
    pub fn spans(&self) -> Vec<(i64, i64)> {
        let mut x = 0;
        self.sections
            .iter()
            .map(|s| {
                let span = (x, x + s.width.max(0));
                x = span.1;
                span
            })
            .collect()
    }

    /// The section at x, and whether x is on its right edge's grip.
    pub fn hit(&self, x: i64) -> Option<(usize, bool)> {
        let spans = self.spans();
        // (the grip reaches a little past the edge, onto the next section)
        if let Some(i) = spans.iter().position(|&(_, r)| (x - r).abs() <= GRIP) {
            return Some((i, true));
        }
        spans.iter().position(|&(l, r)| x >= l && x < r).map(|i| (i, false))
    }

    /// Whether a section's edge is being dragged.
    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// Whether x is where the mouse resizes (for the cursor).
    pub fn on_grip(&self, x: i64) -> bool {
        self.drag.is_some() || self.hit(x).is_some_and(|(_, grip)| grip)
    }

    /// The mouse went down at x.
    pub fn press(&mut self, x: i64) -> Vec<Action> {
        match self.hit(x) {
            Some((i, true)) => {
                self.drag = Some((i, x, self.sections[i].width));
                vec![Action::Track(i, self.sections[i].width, TS_BEGIN)]
            }
            Some((i, false)) if self.sections[i].allow_click => {
                self.pressed = Some(i);
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    /// The mouse moved to x (with the button held).
    pub fn drag_to(&mut self, x: i64) -> Vec<Action> {
        let Some((i, from, width)) = self.drag else { return Vec::new() };
        let s = &mut self.sections[i];
        let w = (width + x - from).clamp(s.min_width.max(0), s.max_width.max(s.min_width.max(0)));
        if w == s.width {
            return Vec::new();
        }
        s.width = w;
        vec![Action::Track(i, w, TS_MOVE)]
    }

    /// The mouse went up at x: a click on the section it went down on, or
    /// the end of a resize.
    pub fn release(&mut self, x: i64) -> Vec<Action> {
        if let Some((i, _, _)) = self.drag.take() {
            return vec![Action::Track(i, self.sections[i].width, TS_END)];
        }
        match self.pressed.take() {
            Some(i) if self.hit(x).is_some_and(|(j, _)| j == i) => vec![Action::Click(i)],
            _ => Vec::new(),
        }
    }

    /// Paints the sections' faces on `surface` (its height the header's):
    /// raised buttons (sunken while pressed) with their captions; returns
    /// the owner-drawn sections as (index, pressed, rect) for OnDrawSection.
    pub fn paint(&self, surface: &mut Bitmap, font: &Font) -> Vec<OwnerDrawn> {
        self.paint_scrolled(surface, font, 0)
    }

    /// [`Header::paint`] with the sections `dx` pixels to the left (a list
    /// view's header follows its columns when they scroll sideways).
    pub fn paint_scrolled(&self, surface: &mut Bitmap, font: &Font, dx: i64) -> Vec<OwnerDrawn> {
        let h = surface.img.height as i64;
        let w = surface.img.width as i64;
        let face = 0xF0F0F0;
        surface.fill_rect(0, 0, w, h, face);
        let mut owner = Vec::new();
        let spans: Vec<(i64, i64)> = self.spans().into_iter().map(|(l, r)| (l - dx, r - dx)).collect();
        for (i, (s, &(l, r))) in self.sections.iter().zip(spans.iter()).enumerate() {
            let pressed = self.pressed == Some(i);
            let (light, dark) = if pressed { (0x808080, 0xFFFFFF) } else { (0xFFFFFF, 0x808080) };
            surface.line(l, 0, r - 1, 0, light);
            surface.line(l, 0, l, h - 1, light);
            surface.line(l, h - 1, r - 1, h - 1, dark);
            surface.line(r - 1, 0, r - 1, h - 1, dark);
            let rect = (l, 0, r, h);
            if s.style == HS_OWNER_DRAW {
                owner.push((i, pressed, rect));
                continue;
            }
            let (tw, th) = super::text::text_size(&s.caption, font);
            let inner = (r - l - 8).max(0);
            let tx = match s.alignment {
                1 => l + 4 + (inner - tw).max(0),
                2 => l + 4 + (inner - tw).max(0) / 2,
                _ => l + 4,
            } + i64::from(pressed);
            let ty = (h - th) / 2 + i64::from(pressed);
            let color = font.color as u32 & 0xFFFFFF;
            super::text::text_out(surface, tx, ty, &s.caption, font, color, None);
        }
        // What's past the last section: the header's face (drawn raised).
        let end = spans.last().map_or(0, |s| s.1);
        if end < w {
            surface.line(end, h - 1, w - 1, h - 1, 0x808080);
        }
        owner
    }

    pub fn get(&self, prop: &str) -> Option<Value> {
        Some(match prop {
            "sectionscount" | "sectioncount" => v_int(self.sections.len() as i64),
            "hottrack" => v_int(if self.hot_track { -1 } else { 0 }),
            _ => return None,
        })
    }

    pub fn set(&mut self, prop: &str, val: &Value) -> bool {
        match prop {
            "hottrack" => self.hot_track = val.to_bool(),
            _ => return false,
        }
        true
    }

    /// AddSections, Clear, and `Sections(i).Caption` / `.Width` / … (the
    /// methods `sections.caption` / `sections.caption=` with the index first).
    pub fn call(&mut self, method: &str, args: &[Value]) -> Option<Value> {
        match method {
            "addsections" | "addsection" => {
                self.sections.extend(args.iter().map(|a| Section::new(a.to_string_val())));
                return Some(Value::Null);
            }
            "clear" => {
                self.sections.clear();
                self.pressed = None;
                self.drag = None;
                return Some(Value::Null);
            }
            _ => {}
        }
        let member = method.strip_prefix("sections.").or_else(|| method.strip_prefix("section."))?;
        let setter = member.ends_with('=');
        let member = member.trim_end_matches('=');
        let i = args.first().map(Value::to_i64).and_then(|i| usize::try_from(i).ok()).filter(|&i| i < self.sections.len());
        if setter {
            let Some(i) = i else { return Some(Value::Null) };
            let val = args.get(1).cloned().unwrap_or(Value::Null);
            let s = &mut self.sections[i];
            match member {
                "caption" | "text" => s.caption = val.to_string_val(),
                "width" => s.width = val.to_i64().clamp(s.min_width.max(0), s.max_width.max(0)),
                "minwidth" => s.min_width = val.to_i64().max(0),
                "maxwidth" => s.max_width = val.to_i64().max(0),
                "alignment" => s.alignment = val.to_i64(),
                "allowclick" => s.allow_click = val.to_bool(),
                "style" => s.style = val.to_i64(),
                _ => return None,
            }
            return Some(Value::Null);
        }
        let Some(i) = i else { return Some(Value::Null) };
        let s = &self.sections[i];
        Some(match member {
            "caption" | "text" => v_str(&s.caption),
            "width" => v_int(s.width),
            "minwidth" => v_int(s.min_width),
            "maxwidth" => v_int(s.max_width),
            "alignment" => v_int(s.alignment),
            "allowclick" => v_int(if s.allow_click { -1 } else { 0 }),
            "style" => v_int(s.style),
            "left" => v_int(self.spans()[i].0),
            "right" => v_int(self.spans()[i].1),
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header() -> Header {
        let mut h = Header::default();
        h.call("addsections", &[v_str("1"), v_str("2"), v_str("3")]);
        h.call("sections.width=", &[v_int(0), v_int(100)]);
        h.call("sections.allowclick=", &[v_int(1), v_int(0)]);
        h
    }

    #[test]
    fn layout_and_hits() {
        let mut h = header();
        assert_eq!(h.spans(), vec![(0, 100), (100, 150), (150, 200)]);
        assert_eq!(h.hit(10), Some((0, false)));
        assert_eq!(h.hit(101), Some((0, true)));
        assert_eq!(h.hit(120), Some((1, false)));
        assert_eq!(h.hit(500), None);
        assert_eq!(h.call("sections.caption", &[v_int(2)]).unwrap().to_string_val(), "3");
        assert_eq!(h.get("sectionscount").unwrap().to_i64(), 3);
    }

    #[test]
    fn clicks_and_resizes() {
        let mut h = header();
        assert!(h.press(20).is_empty());
        assert_eq!(h.pressed, Some(0));
        assert_eq!(h.release(30), vec![Action::Click(0)]);
        // AllowClick = False: no click.
        h.press(120);
        assert!(h.release(120).is_empty());
        // Dragging section 0's edge from 100 to 140, then to -50 (MinWidth 0).
        assert_eq!(h.press(100), vec![Action::Track(0, 100, TS_BEGIN)]);
        assert_eq!(h.drag_to(140), vec![Action::Track(0, 140, TS_MOVE)]);
        assert!(h.drag_to(140).is_empty());
        assert_eq!(h.release(140), vec![Action::Track(0, 140, TS_END)]);
        assert_eq!(h.spans()[1], (140, 190));
        h.call("sections.minwidth=", &[v_int(0), v_int(30)]);
        h.press(140);
        h.drag_to(-50);
        assert_eq!(h.release(-50), vec![Action::Track(0, 30, TS_END)]);
    }

    #[test]
    fn paints_faces_and_owner_drawn() {
        let mut h = header();
        h.call("sections.style=", &[v_int(2), v_int(HS_OWNER_DRAW)]);
        let mut b = Bitmap::new_canvas();
        b.fit(260, 20);
        let owner = h.paint(&mut b, &Font::default());
        assert_eq!(owner, vec![(2, false, (150, 0, 200, 20))]);
        // A raised face: light top-left, dark bottom-right; text in section 0.
        assert_eq!((b.pixel(0, 0), b.pixel(99, 19)), (Some(0xFFFFFF), Some(0x808080)));
        assert!((1..99).any(|x| (1..19).any(|y| b.pixel(x, y).is_some_and(|c| c < 0x808080))), "a caption");
    }
}
