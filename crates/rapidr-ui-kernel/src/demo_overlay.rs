//! The demo overlay: what a recorded tutorial draws over the program's
//! windows (`rapidr_ui_app::demo`, docs/video-pipeline.md), in the order
//! it is painted —
//!
//! 1. a **highlight** around a control: a ring in the accent with a soft
//!    glow, or a **spotlight** (everything else dimmed, the control left
//!    lit);
//! 2. the demo's **pointer**: the virtual cursor's arrow
//!    ([`crate::virtual_cursor::ARROW`]: the theme's text colour on a rim of
//!    its window colour, the accent's dot at its tail that says "driven"),
//!    a little bigger for a video and lifted by a soft shadow; a click's
//!    ring grows and fades from where it pressed;
//! 3. the **caption strip**: the line being said, at the bottom, on a card
//!    with the accent's 4-pixel mark (the design guide's question strip),
//!    in Inter;
//! 4. a **title card** over everything (the video's first and last
//!    seconds): a background picture, the logo, a title and its lines.
//!
//! Everything is in the frame's logical pixels; the colours are the
//! theme's tokens (light, dark and high contrast alike). Nothing here
//! reads input or a clock: the demo player decides what shows (its
//! alphas fade things in and out), the kernel draws it.

use std::sync::Arc;

use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::{Op, Place};

use crate::display::Picture;
use crate::paint::Painter;
use crate::Rect;

/// What the overlay shows now.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Overlay {
    /// The frame (logical pixels).
    pub size: (i64, i64),
    pub highlight: Option<Highlight>,
    pub pointer: Option<Pointer>,
    pub caption: Option<Caption>,
    pub card: Option<Card>,
}

impl Overlay {
    /// Nothing to draw.
    pub fn is_empty(&self) -> bool {
        self.highlight.is_none() && self.pointer.is_none() && self.caption.is_none() && self.card.is_none()
    }
}

/// A control singled out.
#[derive(Clone, Debug, PartialEq)]
pub struct Highlight {
    pub rect: Rect,
    /// Everything else dimmed (else a ring alone).
    pub spotlight: bool,
    pub alpha: u8,
}

/// The demo's pointer.
#[derive(Clone, Debug, PartialEq)]
pub struct Pointer {
    /// Its tip.
    pub at: (f64, f64),
    pub alpha: u8,
    /// A button is held (the arrow a little smaller).
    pub pressed: bool,
    /// A click's ring: where, and how far it has grown (0 to 1).
    pub ripple: Option<((f64, f64), f64)>,
    /// How big (1: the virtual cursor's size).
    pub size: f64,
}

/// The line being said, wrapped (at most a few lines).
#[derive(Clone, Debug, PartialEq)]
pub struct Caption {
    pub lines: Vec<String>,
    /// The text's height (pixels) and the widest line's width.
    pub px: i64,
    pub width: i64,
    pub alpha: u8,
}

/// A full-frame card: the video's title, its end.
#[derive(Clone, Debug, PartialEq)]
pub struct Card {
    pub title: String,
    pub lines: Vec<String>,
    /// Smaller lines at the bottom (credits).
    pub small: Vec<String>,
    /// A picture filling the frame (else the theme's page colour).
    pub background: Option<(Arc<Picture>, u64)>,
    /// The logo over the title and its size (logical pixels).
    pub logo: Option<(Arc<Picture>, u64, (i64, i64))>,
    /// Light text (on a dark background picture) or the theme's.
    pub light_text: bool,
    pub alpha: u8,
}

/// The caption's font: Inter at `px` pixels (`bold` for a card's title).
pub fn font(px: i64, bold: bool) -> Font {
    Font { name: "Inter".into(), size: -px.max(6), color: 0, styles: u8::from(bold) }
}

/// The caption strip's box for a frame of `size` (logical pixels): centred,
/// above the bottom edge.
pub fn caption_box(c: &Caption, size: (i64, i64)) -> Rect {
    let pad_x = (c.px as f64 * 0.9).round() as i64 + 4;
    let pad_y = (c.px as f64 * 0.55).round() as i64;
    let line_h = line_height(c.px);
    let w = (c.width + 2 * pad_x + 4).min(size.0 - 32).max(80);
    let h = line_h * c.lines.len().max(1) as i64 + 2 * pad_y;
    let bottom = (size.1 as f64 * 0.045).round() as i64 + 8;
    ((size.0 - w) / 2, size.1 - bottom - h, w, h)
}

/// A line's height for text of `px` pixels.
pub fn line_height(px: i64) -> i64 {
    (px as f64 * 1.35).round() as i64
}

/// Draws the overlay (the painter's origin at the frame's top left).
pub fn paint(p: &mut Painter, o: &Overlay) {
    if let Some(h) = &o.highlight {
        paint_highlight(p, h, o.size);
    }
    if let Some(ptr) = &o.pointer {
        paint_pointer(p, ptr);
    }
    if let Some(c) = &o.caption {
        paint_caption(p, c, o.size);
    }
    if let Some(card) = &o.card {
        paint_card(p, card, o.size);
    }
}

fn faded(p: &mut Painter, alpha: u8, f: impl FnOnce(&mut Painter)) {
    if alpha == 0 {
        return;
    }
    if alpha < 255 {
        p.op(Op::Fade { alpha });
    }
    f(p);
    if alpha < 255 {
        p.op(Op::FadePop);
    }
}

fn paint_highlight(p: &mut Painter, h: &Highlight, size: (i64, i64)) {
    let t = p.theme();
    let (accent, shadow) = (t.accent, t.shadow_ink);
    let (x, y, w, hh) = h.rect;
    let m = 6;
    let ring = (x - m, y - m, w + 2 * m, hh + 2 * m);
    faded(p, h.alpha, |p| {
        if h.spotlight {
            // (everything around the control dimmed: four bands)
            let (rx, ry, rw, rh) = ring;
            p.op(Op::Fade { alpha: 120 });
            p.fill((0, 0, size.0, ry.max(0)), shadow);
            p.fill((0, ry + rh, size.0, (size.1 - ry - rh).max(0)), shadow);
            p.fill((0, ry, rx.max(0), rh), shadow);
            p.fill((rx + rw, ry, (size.0 - rx - rw).max(0), rh), shadow);
            p.op(Op::FadePop);
        }
        // (a soft glow, then the ring)
        p.op(Op::Fade { alpha: 70 });
        p.ring((ring.0 - 3, ring.1 - 3, ring.2 + 6, ring.3 + 6), 12.0, accent, 6.0);
        p.op(Op::FadePop);
        p.ring(ring, 9.0, accent, 3.0);
    });
}

fn paint_pointer(p: &mut Painter, c: &Pointer) {
    let t = p.theme();
    let (text, window, accent, shadow) = (t.text, t.window, t.accent, t.shadow_ink);
    faded(p, c.alpha, |p| {
        if let Some(((x, y), k)) = c.ripple {
            let k = k.clamp(0.0, 1.0);
            let r = (8.0 + 22.0 * k) * c.size.max(0.5);
            let alpha = (220.0 * (1.0 - k)).round().clamp(0.0, 255.0) as u8;
            if alpha > 0 {
                p.op(Op::Fade { alpha });
                let d = (2.0 * r).round() as i64;
                p.ring(((x - r).round() as i64, (y - r).round() as i64, d, d), r, accent, 3.0);
                p.op(Op::FadePop);
            }
        }
        let s = c.size.max(0.5) * if c.pressed { 0.88 } else { 1.0 };
        let (x, y) = c.at;
        let pts: Vec<(f64, f64)> = crate::virtual_cursor::ARROW.iter().map(|(ax, ay)| (x + ax * s, y + ay * s)).collect();
        // (a soft shadow under it)
        let off: Vec<(f64, f64)> = pts.iter().map(|(px, py)| (px + 1.6 * s, py + 2.2 * s)).collect();
        p.op(Op::Fade { alpha: 70 });
        p.op(Op::Polygon { points: off, color: shadow });
        p.op(Op::FadePop);
        let mut rim = pts.clone();
        rim.push(pts[0]);
        p.stroke(&rim, window, 3.0 * s.max(1.0));
        p.op(Op::Polygon { points: pts, color: text });
        let (dx, dy) = (x + 12.5 * s, y + 17.5 * s);
        let r = 3.5 * s;
        let d = (2.0 * r).round() as i64;
        p.round(((dx - r).round() as i64, (dy - r).round() as i64, d, d), r, Some(accent), Some(window), 1.25);
    });
}

fn paint_caption(p: &mut Painter, c: &Caption, size: (i64, i64)) {
    if c.lines.is_empty() {
        return;
    }
    let t = p.theme();
    let card = rapidr_value::ide_theme::color_by_name(t, "card").unwrap_or(t.window);
    let edge = rapidr_value::ide_theme::color_by_name(t, "card.border").unwrap_or(t.border);
    let ink = rapidr_value::ide_theme::color_by_name(t, "page.text").unwrap_or(t.text);
    let accent = t.accent;
    let rect = caption_box(c, size);
    let (x, y, w, h) = rect;
    let radius = 12.0;
    let pad_y = (c.px as f64 * 0.55).round() as i64;
    let line_h = line_height(c.px);
    let f = font(c.px, false);
    faded(p, c.alpha, |p| {
        // (lifted off the window by the theme's shadow; none in high
        // contrast, where the border says it)
        p.elevate(rect, radius, 18.0);
        p.round(rect, radius, Some(card), Some(edge), 1.0);
        // (the accent's mark at its left, inside the rounding)
        p.clipped(rect, |p| p.fill((x, y, 4, h), accent));
        for (i, line) in c.lines.iter().enumerate() {
            let ly = y + pad_y + i as i64 * line_h;
            p.text((x + 8, ly, w - 8, line_h), line, &f, ink, Place::TopCenter);
        }
    });
}

fn paint_card(p: &mut Painter, card: &Card, size: (i64, i64)) {
    let t = p.theme();
    let page = rapidr_value::ide_theme::color_by_name(t, "page").unwrap_or(t.window);
    let (ink, dim) = if card.light_text {
        (0xFFFFFF, 0xC9D2E3)
    } else {
        (rapidr_value::ide_theme::color_by_name(t, "page.text").unwrap_or(t.text), rapidr_value::ide_theme::color_by_name(t, "page.dim").unwrap_or(t.gray_text))
    };
    let accent = t.accent;
    let (w, h) = size;
    faded(p, card.alpha, |p| {
        p.fill((0, 0, w, h), page);
        if let Some((pic, rev)) = &card.background {
            p.picture("demo:card-bg", *rev, pic.clone(), (0, 0, w, h));
        }
        // (the block centred: the logo, the title, its lines)
        let title_px = (h as f64 * 0.062).round() as i64;
        let line_px = (h as f64 * 0.033).round() as i64;
        let small_px = (h as f64 * 0.024).round() as i64;
        let logo_h = card.logo.as_ref().map_or(0, |(_, _, (_, lh))| *lh);
        let gap = (h as f64 * 0.045).round() as i64;
        let block = logo_h + if logo_h > 0 { gap } else { 0 } + line_height(title_px) + card.lines.len() as i64 * (line_height(line_px) + 6);
        let mut y = (h - block) / 2 - (h as f64 * 0.04).round() as i64;
        if let Some((pic, rev, (lw, lh))) = &card.logo {
            p.picture("demo:card-logo", *rev, pic.clone(), ((w - lw) / 2, y, *lw, *lh));
            y += lh + gap;
        } else {
            // (no logo: the accent's short bar over the title)
            p.round(((w - 96) / 2, y - 24, 96, 6), 3.0, Some(accent), None, 0.0);
        }
        let tf = font(title_px, true);
        p.text((0, y, w, line_height(title_px)), &card.title, &tf, ink, Place::TopCenter);
        y += line_height(title_px) + 10;
        let lf = font(line_px, false);
        for line in &card.lines {
            p.text((0, y, w, line_height(line_px)), line, &lf, dim, Place::TopCenter);
            y += line_height(line_px) + 6;
        }
        let sf = font(small_px, false);
        let n = card.small.len() as i64;
        let mut sy = h - (h as f64 * 0.07).round() as i64 - n * (line_height(small_px) + 4);
        for line in &card.small {
            p.text((0, sy, w, line_height(small_px)), line, &sf, dim, Place::TopCenter);
            sy += line_height(small_px) + 4;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::{DisplayList, Item};

    fn drawn(o: &Overlay) -> DisplayList {
        let mut list = DisplayList { size: o.size, scale: 1.0, ..Default::default() };
        let mut p = Painter::new(&mut list);
        paint(&mut p, o);
        list
    }

    #[test]
    fn nothing_set_draws_nothing() {
        let o = Overlay { size: (1280, 720), ..Default::default() };
        assert!(o.is_empty());
        assert!(drawn(&o).items.is_empty());
    }

    #[test]
    fn the_pointer_and_its_ripple_are_drawn_and_fade() {
        let mut o = Overlay { size: (1280, 720), ..Default::default() };
        o.pointer = Some(Pointer { at: (100.0, 80.0), alpha: 255, pressed: false, ripple: Some(((100.0, 80.0), 0.5)), size: 1.25 });
        let list = drawn(&o);
        assert!(list.items.iter().any(|i| matches!(i, Item::Op { op: Op::Polygon { .. }, .. })), "{}", list.dump());
        // (faded: one more layer around it all)
        o.pointer.as_mut().unwrap().alpha = 100;
        let faded = drawn(&o);
        let fades = |l: &DisplayList| l.items.iter().filter(|i| matches!(i, Item::Op { op: Op::Fade { .. }, .. })).count();
        assert_eq!(fades(&faded), fades(&list) + 1);
        // (gone: alpha 0)
        o.pointer.as_mut().unwrap().alpha = 0;
        assert!(drawn(&o).items.is_empty());
    }

    #[test]
    fn the_caption_sits_centred_above_the_bottom() {
        let c = Caption { lines: vec!["Drag a button onto the form.".into()], px: 22, width: 300, alpha: 255 };
        let (x, y, w, h) = caption_box(&c, (1280, 720));
        assert_eq!(x * 2 + w, 1280, "centred");
        assert!(y + h < 720 && y + h > 640, "near the bottom: {y} {h}");
        let o = Overlay { size: (1280, 720), caption: Some(c), ..Default::default() };
        let list = drawn(&o);
        assert!(list.items.iter().any(|i| matches!(i, Item::Op { op: Op::Text { text, .. }, .. } if text.starts_with("Drag a button"))), "{}", list.dump());
    }

    #[test]
    fn a_card_covers_the_frame_with_its_title() {
        let o = Overlay { size: (1280, 720), card: Some(Card { title: "Your first app".into(), lines: vec!["RapidR Studio".into()], small: vec!["Voice: AI-generated (Qwen3-TTS)".into()], background: None, logo: None, light_text: false, alpha: 255 }), ..Default::default() };
        let list = drawn(&o);
        let texts: Vec<String> = list.items.iter().filter_map(|i| if let Item::Op { op: Op::Text { text, .. }, .. } = i { Some(text.clone()) } else { None }).collect();
        assert_eq!(texts, vec!["Your first app", "RapidR Studio", "Voice: AI-generated (Qwen3-TTS)"]);
    }
}
