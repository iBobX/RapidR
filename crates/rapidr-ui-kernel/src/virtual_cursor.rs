//! The virtual cursor: the pointer an AI (RapidR Studio's AI apps, RAI) or
//! the program's own RAppHands moves over the program's windows while it
//! drives them (`rapidr_ui_app::hands`). It is drawn by the kernel, on top
//! of everything in the window it is in — never the system's mouse, which
//! stays where the person left it and keeps working.
//!
//! An arrow in the theme's text colour with a rim in its window colour (so
//! it reads on light and dark), and a dot in the accent at its tail that
//! says "this one is driven"; a click shows as a ring in the accent that
//! grows and fades from where it pressed. Where the person asked the
//! system for reduced motion the cursor jumps instead of gliding and the
//! ring shows once, still (the driver decides: [`VirtualCursor`]'s fields
//! are what to draw now).
//!
//! One cursor for the process, in one window at a time ([`set`]); the
//! window it is in draws it at the end of [`crate::FormUi::paint`] (or of
//! `paint_popups`, for a host that draws pop-ups apart), so it is over the
//! open menus and drop-down lists too.

use std::cell::RefCell;

use rapidr_value::objects::ops::Op;

use crate::paint::Painter;

/// What the virtual cursor shows now.
#[derive(Clone, Debug, PartialEq)]
pub struct VirtualCursor {
    /// The form (lowercase) whose window it is in.
    pub form: String,
    /// Its tip, in the window's inside (logical pixels; an in-window menu
    /// bar included).
    pub at: (f64, f64),
    /// How opaque it is (255: fully; it fades out when the driving is
    /// over).
    pub alpha: u8,
    /// A click's ring: where it pressed and how far it has grown (0 to 1;
    /// fading as it grows).
    pub ripple: Option<((f64, f64), f64)>,
    /// A button is held (the arrow drawn a little smaller, as pressed).
    pub pressed: bool,
}

thread_local! {
    static CURSOR: RefCell<Option<VirtualCursor>> = const { RefCell::new(None) };
}

/// The virtual cursor from now on (`None`: hidden).
pub fn set(cursor: Option<VirtualCursor>) {
    CURSOR.with(|c| *c.borrow_mut() = cursor);
}

/// The virtual cursor as it shows now.
pub fn get() -> Option<VirtualCursor> {
    CURSOR.with(|c| c.borrow().clone())
}

/// The arrow's outline, its tip at (0, 0) (logical pixels, the system
/// arrow's proportions).
pub const ARROW: [(f64, f64); 7] = [(0.0, 0.0), (0.0, 16.0), (4.0, 12.5), (7.0, 18.5), (9.5, 17.3), (6.6, 11.6), (11.5, 11.6)];

/// Draws the cursor if it is in `form`'s window (`p` at the window's
/// inside).
pub(crate) fn paint(form: &str, p: &mut Painter) {
    let Some(c) = get() else { return };
    if !c.form.eq_ignore_ascii_case(form) || c.alpha == 0 {
        return;
    }
    let t = p.theme();
    if c.alpha < 255 {
        p.op(Op::Fade { alpha: c.alpha });
    }
    // (the click's ring, under the arrow)
    if let Some(((x, y), k)) = c.ripple {
        let k = k.clamp(0.0, 1.0);
        let r = 6.0 + 16.0 * k;
        let alpha = (255.0 * (1.0 - k)).round().clamp(0.0, 255.0) as u8;
        if alpha > 0 {
            p.op(Op::Fade { alpha });
            let d = (2.0 * r).round() as i64;
            let rect = ((x - r).round() as i64, (y - r).round() as i64, d, d);
            p.ring(rect, r, t.accent, 2.5);
            p.op(Op::FadePop);
        }
    }
    let shrink = if c.pressed { 0.85 } else { 1.0 };
    let (x, y) = c.at;
    let pts: Vec<(f64, f64)> = ARROW.iter().map(|(ax, ay)| (x + ax * shrink, y + ay * shrink)).collect();
    // (its rim: the outline stroked wide in the window's colour, then the
    // arrow filled on it)
    let mut rim = pts.clone();
    rim.push(pts[0]);
    p.stroke(&rim, t.window, 3.0);
    p.op(Op::Polygon { points: pts, color: t.text });
    // (the accent's dot at its tail: driven, not the person's mouse)
    let (dx, dy) = (x + 12.5 * shrink, y + 17.5 * shrink);
    let r = 3.5;
    p.round(((dx - r).round() as i64, (dy - r).round() as i64, 7, 7), r, Some(t.accent), Some(t.window), 1.25);
    if c.alpha < 255 {
        p.op(Op::FadePop);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::{DisplayList, Item};

    fn drawn(form: &str) -> DisplayList {
        let mut list = DisplayList { size: (200, 100), scale: 1.0, ..Default::default() };
        let mut p = Painter::new(&mut list);
        paint(form, &mut p);
        list
    }

    #[test]
    fn it_shows_only_in_its_own_window_and_fades() {
        set(Some(VirtualCursor { form: "form1".into(), at: (40.0, 30.0), alpha: 255, ripple: None, pressed: false }));
        let here = drawn("Form1");
        assert!(here.items.iter().any(|i| matches!(i, Item::Op { op: Op::Polygon { .. }, .. })), "{}", here.dump());
        assert!(drawn("form2").items.is_empty());
        // (fading: one layer)
        set(Some(VirtualCursor { form: "form1".into(), at: (40.0, 30.0), alpha: 100, ripple: Some(((40.0, 30.0), 0.5)), pressed: true }));
        let faded = drawn("form1");
        let fades = faded.items.iter().filter(|i| matches!(i, Item::Op { op: Op::Fade { .. }, .. })).count();
        let pops = faded.items.iter().filter(|i| matches!(i, Item::Op { op: Op::FadePop, .. })).count();
        assert_eq!((fades, pops), (2, 2), "{}", faded.dump());
        set(None);
        assert!(drawn("form1").items.is_empty());
    }
}
