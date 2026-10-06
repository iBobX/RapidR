//! A window's frame — its border, title bar and title bar buttons — as
//! the kernel draws it: a web page's windows (`rapidr-ui-host-web`'s
//! frame, the whole window drawn on the page) and QFORMMDI's child windows
//! (`components/mdi.rs`) are drawn here, so a window looks the same
//! wherever the kernel draws one. (On the desktop a form's window is the
//! system's, as an RC.EXE program's is; only its inside is the kernel's.)
//!
//! - **RapidR's look** (light, dark): RapidR Studio's windows — a light
//!   window whose title bar is its surface, touched by the accent while
//!   it's active (the theme's caption colours), a hairline frame rounded
//!   7 pixels with a thin accent line on top of the active one, the title
//!   in the chrome font, the buttons as thin glyphs with no bevel (under
//!   the mouse: a soft fill, the close button red). High contrast: the
//!   theme's strong title bar, every edge framed.
//! - **Classic**: Windows 98 / 2000's at 1× ([`Metrics::classic`]: an
//!   18-pixel title bar, 16 × 14 buttons — minimize and maximize side by
//!   side, the close button two pixels apart — a 4-pixel raised border, 3
//!   when the window can't be resized), scaled whole at 2×.
//!
//! A frame never changes a form's sizes: Width / Height and ClientWidth /
//! ClientHeight are `rapidr_value::layout::form_frame`'s in every look;
//! what a host draws around the inside is only drawn.

use std::sync::Arc;

use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::Place;
use rapidr_value::theme::Theme;

use crate::display::Picture;
use crate::paint::Painter;
use crate::Rect;

/// A title bar button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Close,
    Maximize,
    Minimize,
}

/// A frame's sizes, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    /// The border around the window, each side.
    pub border: i64,
    /// The title bar's height (below the border).
    pub caption: i64,
    /// A button's size, and its place: `right` pixels from the inside's
    /// right edge (the close button's), `gap` between the close button and
    /// the others, `spacing` between those, `top` below the title bar's
    /// top.
    pub button: (i64, i64),
    pub right: i64,
    pub gap: i64,
    pub spacing: i64,
    pub top: i64,
    /// The corners' radius (RapidR's look; 0 maximized or classic).
    pub radius: f64,
}

impl Metrics {
    /// Windows 98 / 2000's window frame at 96 dpi (SM_CYCAPTION 19: an
    /// 18-pixel title bar; SM_CXSIZE × SM_CYSIZE 18 × 18: 16 × 14 buttons;
    /// SM_CXFRAME 4, SM_CXFIXEDFRAME 3).
    pub fn classic(resizable: bool) -> Metrics {
        Metrics {
            border: if resizable { 4 } else { 3 },
            caption: 18,
            button: (16, 14),
            right: 2,
            gap: 2,
            spacing: 0,
            top: 2,
            radius: 0.0,
        }
    }

    /// RapidR's window frame: the 1-pixel border and 29-pixel title bar
    /// every runtime accounts a form with (`layout::form_frame`), buttons
    /// 40 wide across the title bar's height.
    pub fn rapidr(maximized: bool) -> Metrics {
        let caption = rapidr_value::layout::FORM_CAPTION;
        Metrics {
            border: rapidr_value::layout::FORM_BORDER,
            caption,
            button: (40, caption),
            right: 0,
            gap: 0,
            spacing: 0,
            top: 0,
            radius: if maximized { 0.0 } else { 7.0 },
        }
    }

    /// A QFORMMDI child's frame (`rapidr_value::mdi`'s border and title
    /// bar, the same in every look).
    pub fn mdi() -> Metrics {
        use rapidr_value::mdi::{BORDER, TITLE_HEIGHT};
        Metrics {
            border: BORDER,
            caption: TITLE_HEIGHT - 2,
            button: (TITLE_HEIGHT - 5, TITLE_HEIGHT - 6),
            right: 2,
            gap: 3,
            spacing: 3,
            top: 2,
            radius: 7.0,
        }
    }

    /// A top-level window's frame in `t`.
    pub fn window(t: &Theme, resizable: bool, maximized: bool) -> Metrics {
        if t.fluent() {
            Metrics::rapidr(maximized)
        } else {
            Metrics::classic(resizable)
        }
    }

    /// Where the inside starts: (left, top).
    pub fn inset(&self) -> (i64, i64) {
        (self.border, self.border + self.caption)
    }

    /// The frame around the inside: (left + right, top + bottom).
    pub fn around(&self) -> (i64, i64) {
        (2 * self.border, 2 * self.border + self.caption)
    }

    /// The title bar's rectangle in a `w` wide window.
    pub fn title_bar(&self, w: i64) -> Rect {
        (
            self.border,
            self.border,
            (w - 2 * self.border).max(0),
            self.caption,
        )
    }

    /// The buttons' rectangles in a `w` wide window, from the right: the
    /// close button, then maximize and minimize.
    pub fn buttons(&self, w: i64, shown: &[Button]) -> Vec<(Button, Rect)> {
        let (bw, bh) = self.button;
        let mut x = w - self.border - self.right;
        let y = self.border + self.top;
        shown
            .iter()
            .map(|b| {
                x -= bw;
                let r = (x, y, bw, bh);
                x -= if *b == Button::Close {
                    self.gap
                } else {
                    self.spacing
                };
                (*b, r)
            })
            .collect()
    }
}

/// What a frame shows.
#[derive(Clone, Debug, Default)]
pub struct Chrome {
    pub title: String,
    /// The window with the keyboard.
    pub active: bool,
    pub maximized: bool,
    /// The buttons from the right (close, maximize, minimize), and whether
    /// each is enabled.
    pub buttons: Vec<(Button, bool)>,
    /// The button under the mouse, and the one held down.
    pub hot: Option<Button>,
    pub pressed: Option<Button>,
    /// The window's icon (16 × 16 at the title bar's left): a picture's
    /// source name, revision and pixels.
    pub icon: Option<(String, u64, Arc<Picture>)>,
    /// The title's font (bold drawn bold): the classic look's is the
    /// window's (MS Sans Serif 8, bold); RapidR's the chrome font.
    pub font: Font,
}

/// Red under the mouse on a close button (Windows 11's, RapidR Studio's),
/// and the white it's crossed in.
const CLOSE_HOT: u32 = 0xC42B1C;
const CLOSE_PRESSED: u32 = 0xC84031;

/// Frame `chrome` drawn in a `w` × `h` window (or MDI child) at the
/// painter's origin.
pub fn paint(p: &mut Painter, (w, h): (i64, i64), chrome: &Chrome, m: &Metrics) {
    let t = p.theme();
    let (bx, by, bw, bh) = m.title_bar(w);
    let (mut bar, mut ink) = if chrome.active {
        (t.caption, t.caption_text)
    } else {
        (t.inactive_caption, t.inactive_caption_text)
    };
    if !t.fluent() {
        p.fill((0, 0, w, h), t.face);
        // (a window's raised border)
        p.raised_edge((0, 0, w, h));
        p.fill((bx, by, bw, bh), bar);
    } else if t.contrast {
        p.fill((0, 0, w, h), t.face);
        p.frame(
            (0, 0, w, h),
            if chrome.active { t.caption } else { t.border },
        );
        // (the title bar reaches the frame)
        p.fill((1, 1, w - 2, bh + m.border - 1), bar);
    } else {
        // (a light window, its title bar the window's surface; the accent
        // only in the active one's frame and a thin line on top)
        let surface = t.face;
        let radius = if chrome.maximized { 0.0 } else { m.radius };
        let line = if chrome.active {
            rapidr_value::theme::mix(t.accent, surface, 250)
        } else {
            t.border
        };
        p.round((0, 0, w, h), radius, Some(surface), None, 1.0);
        p.clipped((0, 0, w, bh + m.border), |p| {
            p.round((0, 0, w, h), radius, Some(bar), None, 1.0)
        });
        p.fill(
            (1, bh + m.border, w - 2, 1),
            rapidr_value::theme::mix(t.border, surface, 300),
        );
        p.round((0, 0, w, h), radius, None, Some(line), 1.0);
        if chrome.active && !chrome.maximized {
            p.clipped((0, 0, w, 2), |p| {
                p.round((0, 0, w, h), radius, Some(t.accent), None, 1.0)
            });
        }
        bar = surface;
        if !chrome.active {
            ink = t.inactive_caption_text;
        }
    }
    let buttons: Vec<Button> = chrome.buttons.iter().map(|(b, _)| *b).collect();
    let rects = m.buttons(w, &buttons);
    // (the icon, then the title after it, up to the buttons)
    let mut text_x = bx + if t.fluent() { 8 } else { 2 };
    if let Some((source, revision, picture)) = &chrome.icon {
        let side = 16.min(bh);
        p.picture(
            source,
            *revision,
            picture.clone(),
            (
                bx + if t.fluent() { 6 } else { 1 },
                by + (bh - side) / 2,
                side,
                side,
            ),
        );
        text_x += if t.fluent() { 20 } else { 17 };
    }
    let end = rects.iter().map(|(_, r)| r.0).min().unwrap_or(bx + bw) - 4;
    let room = (end - text_x).max(0);
    let font = Font {
        color: rapidr_value::theme::bgr(ink) as i64,
        ..chrome.font.clone()
    };
    p.clipped((text_x - 2, by, room + 2, bh), |p| {
        p.text(
            (text_x, by, room, bh),
            &chrome.title,
            &font,
            ink,
            Place::Left,
        )
    });
    for ((button, r), (_, enabled)) in rects.into_iter().zip(&chrome.buttons) {
        if r.0 <= text_x {
            continue;
        }
        let held = chrome.pressed == Some(button) && *enabled;
        let hot = chrome.hot == Some(button) && *enabled;
        if !t.fluent() {
            p.fill(r, t.face);
            if held {
                p.edge(r, &[t.dark_shadow, t.shadow], &[t.light, t.face]);
            } else {
                p.edge(r, &[t.light, t.face], &[t.dark_shadow, t.shadow]);
            }
            let r = if held {
                (r.0 + 1, r.1 + 1, r.2, r.3)
            } else {
                r
            };
            if !*enabled {
                // (a disabled glyph: embossed, white a pixel down-right)
                glyph(
                    p,
                    button,
                    (r.0 + 1, r.1 + 1, r.2, r.3),
                    chrome.maximized,
                    t.light,
                    false,
                );
            }
            glyph(
                p,
                button,
                r,
                chrome.maximized,
                if *enabled { t.text } else { t.gray_text },
                false,
            );
            continue;
        }
        let mut glyph_ink = if *enabled {
            ink
        } else {
            rapidr_value::theme::mix(ink, bar, 550)
        };
        if hot || held {
            let fill = match button {
                Button::Close if !t.contrast => {
                    glyph_ink = 0xFFFFFF;
                    if held {
                        CLOSE_PRESSED
                    } else {
                        CLOSE_HOT
                    }
                }
                _ if t.contrast => {
                    glyph_ink = t.highlight_text;
                    t.highlight
                }
                _ => rapidr_value::theme::mix(bar, t.text, if held { 140 } else { 80 }),
            };
            // (the top-right button meets the window's rounded corner)
            let corner =
                if button == Button::Close && m.right == 0 && m.radius > 0.0 && !chrome.maximized {
                    m.radius - 1.0
                } else {
                    0.0
                };
            if corner > 0.0 {
                p.clipped(r, |p| {
                    p.round(
                        (r.0 - 8, r.1, r.2 + 8, r.3 + 8),
                        corner,
                        Some(fill),
                        None,
                        1.0,
                    )
                });
            } else {
                p.fill(r, fill);
            }
        }
        glyph(p, button, r, chrome.maximized, glyph_ink, true);
    }
}

/// A title bar button's glyph: Windows' classic pixel glyphs (×, □ /
/// restore, _), or RapidR's thin vector ones (9 pixels).
fn glyph(
    p: &mut Painter,
    button: Button,
    (x, y, w, h): Rect,
    maximized: bool,
    ink: u32,
    thin: bool,
) {
    let (cx, cy) = (x + w / 2, y + h / 2);
    if thin {
        let (fx, fy) = (cx as f64, cy as f64);
        match button {
            Button::Close => {
                p.stroke(&[(fx - 4.0, fy - 4.0), (fx + 4.0, fy + 4.0)], ink, 1.0);
                p.stroke(&[(fx - 4.0, fy + 4.0), (fx + 4.0, fy - 4.0)], ink, 1.0);
            }
            Button::Maximize if maximized => {
                p.ring((cx - 4, cy - 2, 7, 7), 1.0, ink, 1.0);
                p.stroke(
                    &[
                        (fx - 1.5, fy - 3.5),
                        (fx + 4.5, fy - 3.5),
                        (fx + 4.5, fy + 2.5),
                    ],
                    ink,
                    1.0,
                );
            }
            Button::Maximize => p.ring((cx - 4, cy - 4, 9, 9), 1.5, ink, 1.0),
            Button::Minimize => p.fill((cx - 4, cy, 9, 1), ink),
        }
        return;
    }
    // (DrawFrameControl's caption glyphs in a 16 × 14 button: an 8 × 7 ×
    // two pixels thick, a 9 × 9 window with a two-pixel title, a 6 × 2
    // bar at the bottom)
    match button {
        Button::Close => {
            let (l, t) = ((cx - 4) as f64, (cy - 4) as f64);
            for d in [0.0, 1.0] {
                p.line((l + d + 0.5, t + 0.5), (l + d + 6.5, t + 6.5), ink);
                p.line((l + d + 0.5, t + 6.5), (l + d + 6.5, t + 0.5), ink);
            }
        }
        Button::Maximize if maximized => {
            // restore: two windows, the back one up and right
            for (bx, by) in [(cx - 2, cy - 5), (cx - 5, cy - 2)] {
                p.fill((bx, by, 6, 6), p.theme().face);
                p.edge((bx, by, 6, 6), &[ink], &[ink]);
                p.fill((bx, by + 1, 6, 1), ink);
            }
        }
        Button::Maximize => {
            p.edge((cx - 5, cy - 5, 9, 9), &[ink], &[ink]);
            p.fill((cx - 5, cy - 4, 9, 1), ink);
        }
        Button::Minimize => p.fill((cx - 4, cy + 2, 6, 2), ink),
    }
}

/// What's at (x, y) of a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Button(Button),
    /// The title bar (dragged: the window moves; double-clicked: it's
    /// maximized or restored).
    Title,
    None,
}

/// The part of a `w` wide frame at (x, y), logical; a disabled button is
/// the title bar's.
pub fn hit(m: &Metrics, w: i64, buttons: &[(Button, bool)], x: f64, y: f64) -> Part {
    let (xi, yi) = (x.floor() as i64, y.floor() as i64);
    let (bx, by, bw, bh) = m.title_bar(w);
    if !(by..by + bh).contains(&yi) || !(bx..bx + bw).contains(&xi) {
        return Part::None;
    }
    let shown: Vec<Button> = buttons.iter().map(|(b, _)| *b).collect();
    for ((b, (rx, ry, rw, rh)), (_, enabled)) in m.buttons(w, &shown).into_iter().zip(buttons) {
        if xi >= rx && xi < rx + rw && yi >= ry && yi < ry + rh {
            return if *enabled {
                Part::Button(b)
            } else {
                Part::Title
            };
        }
    }
    Part::Title
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [(Button, bool); 3] = [
        (Button::Close, true),
        (Button::Maximize, true),
        (Button::Minimize, true),
    ];

    #[test]
    fn classic_is_windows_98s() {
        // An 18-pixel title bar, 16 × 14 buttons two pixels in from the
        // right, minimize and maximize together, the close button apart.
        let m = Metrics::classic(true);
        assert_eq!(
            (m.border, m.caption, m.inset(), m.around()),
            (4, 18, (4, 22), (8, 26))
        );
        let r = m.buttons(300, &[Button::Close, Button::Maximize, Button::Minimize]);
        assert_eq!(
            r,
            vec![
                (Button::Close, (278, 6, 16, 14)),
                (Button::Maximize, (260, 6, 16, 14)),
                (Button::Minimize, (244, 6, 16, 14))
            ]
        );
        assert_eq!(Metrics::classic(false).border, 3);
        assert_eq!(hit(&m, 300, &ALL, 285.0, 10.0), Part::Button(Button::Close));
        assert_eq!(
            hit(&m, 300, &ALL, 277.0, 10.0),
            Part::Title,
            "between the buttons"
        );
        assert_eq!(hit(&m, 300, &ALL, 100.0, 10.0), Part::Title);
        assert_eq!(hit(&m, 300, &ALL, 100.0, 30.0), Part::None);
        assert_eq!(
            hit(
                &m,
                300,
                &[
                    (Button::Close, true),
                    (Button::Maximize, false),
                    (Button::Minimize, true)
                ],
                265.0,
                10.0
            ),
            Part::Title
        );
    }

    #[test]
    fn rapidrs_is_every_runtimes_frame() {
        // the frame every runtime accounts a form with: the window is its
        // Width × Height
        let m = Metrics::rapidr(false);
        assert_eq!(m.around(), rapidr_value::layout::form_frame(2));
        let r = m.buttons(320, &[Button::Close, Button::Maximize, Button::Minimize]);
        assert_eq!(r[0], (Button::Close, (279, 1, 40, 29)));
        assert_eq!(r[2], (Button::Minimize, (199, 1, 40, 29)));
        assert_eq!(Metrics::rapidr(true).radius, 0.0);
    }
}
