//! QCOLORDIALOG — RAPIDQ2.INC's QColorDialog over Windows' ChooseColor (the
//! built-in RapidQ has none) — the same on every runtime: its properties,
//! the dialog's state and how the user changes it, its layout and the
//! pictures it shows. The kernel draws it from this and the web builds its
//! page dialog from it.
//!
//! - `Color` (&HBBGGRR, 0 at first) is the colour chosen; `Execute` returns
//!   1 when the user picked one (OK), else 0.
//! - `Colors(1 TO 16)` are the custom colours (RAPIDQ2.INC's constructor
//!   fills them: [`DEFAULT_CUSTOM`]); what the user adds there (Add to
//!   Custom Colors) stays, OK or Cancel, as ChooseColor's array does.
//! - `Style`: cdNormal 0 and cdFullOpen 1 open the custom colour editor
//!   (CC_FULLOPEN), cdNoFullOpen 2 (RAPIDQ2.INC's default) can't open it
//!   (CC_PREVENTFULLOPEN). Any other value: the compact dialog whose
//!   "Define Custom Colors >>" opens the editor (neither flag).
//! - `Caption` titles it ("Color" when empty).
//!
//! The editor works in Windows' HLS (hue 0–239, saturation and luminance
//! 0–240): the hue / saturation field at luminance 120, the luminance bar
//! for the chosen hue and saturation, and the Hue / Sat / Lum, Red / Green
//! / Blue boxes.

/// Style: the editor open (CC_FULLOPEN; cdNormal too, as RAPIDQ2.INC).
pub const CD_NORMAL: i64 = 0;
pub const CD_FULL_OPEN: i64 = 1;
/// Style: no editor (CC_PREVENTFULLOPEN).
pub const CD_NO_FULL_OPEN: i64 = 2;

/// RAPIDQ2.INC's QColorDialog constructor's `Colors(1 TO 16)` (&HBBGGRR).
pub const DEFAULT_CUSTOM: [i64; 16] = [
    0x000000, 0x808080, 0x000080, 0x008080, 0x008000, 0x808000, 0x800000, 0x800080, //
    0xFFFFFF, 0xC0C0C0, 0x0000FF, 0x00FFFF, 0x00FF00, 0xFFFF00, 0xFF0000, 0xFF00FF,
];

/// Windows' ChooseColor basic colours (0xRRGGBB), by rows of eight.
pub const BASIC_COLORS: [u32; 48] = [
    0xFF8080, 0xFFFF80, 0x80FF80, 0x00FF80, 0x80FFFF, 0x0080FF, 0xFF80C0, 0xFF80FF, //
    0xFF0000, 0xFFFF00, 0x80FF00, 0x00FF40, 0x00FFFF, 0x0080C0, 0x8080C0, 0xFF00FF, //
    0x804040, 0xFF8040, 0x00FF00, 0x008080, 0x004080, 0x8080FF, 0x800040, 0xFF0080, //
    0x800000, 0xFF8000, 0x008000, 0x008040, 0x0000FF, 0x0000A0, 0x800080, 0x8000FF, //
    0x400000, 0x804000, 0x004000, 0x004040, 0x000080, 0x000040, 0x400040, 0x400080, //
    0x000000, 0x808000, 0x808040, 0x808080, 0x408080, 0xC0C0C0, 0x400040, 0xFFFFFF,
];

/// How the dialog opens for `Style`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// With the custom colour editor (cdNormal, cdFullOpen).
    Full,
    /// Without it, "Define Custom Colors >>" opening it.
    Compact,
    /// Without it, for good (cdNoFullOpen).
    Prevented,
}

pub fn mode(style: i64) -> Mode {
    match style {
        CD_NORMAL | CD_FULL_OPEN => Mode::Full,
        CD_NO_FULL_OPEN => Mode::Prevented,
        _ => Mode::Compact,
    }
}

/// 0xRRGGBB from &HBBGGRR (and back: the same swap).
pub fn swap_rb(c: i64) -> i64 {
    let c = c & 0xFF_FFFF;
    (c & 0xFF) << 16 | (c & 0xFF00) | (c >> 16 & 0xFF)
}

const HLSMAX: i64 = 240;
const RGBMAX: i64 = 255;
/// The hue of a grey (its saturation 0), as Windows shows it.
const UNDEFINED_HUE: i64 = HLSMAX * 2 / 3;

/// Windows' HLS (hue 0–239, luminance, saturation 0–240) of &HBBGGRR, as
/// ChooseColor computes it (the "RGB to HLS" of Windows' samples).
pub fn to_hls(bgr: i64) -> (i64, i64, i64) {
    let (r, g, b) = (bgr & 0xFF, bgr >> 8 & 0xFF, bgr >> 16 & 0xFF);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let l = ((max + min) * HLSMAX + RGBMAX) / (2 * RGBMAX);
    if max == min {
        return (UNDEFINED_HUE, l, 0);
    }
    let d = max - min;
    let s = if l <= HLSMAX / 2 { (d * HLSMAX + (max + min) / 2) / (max + min) } else { (d * HLSMAX + (2 * RGBMAX - max - min) / 2) / (2 * RGBMAX - max - min) };
    let delta = |c: i64| ((max - c) * (HLSMAX / 6) + d / 2) / d;
    let (rd, gd, bd) = (delta(r), delta(g), delta(b));
    let mut h = if r == max {
        bd - gd
    } else if g == max {
        HLSMAX / 3 + rd - bd
    } else {
        2 * HLSMAX / 3 + gd - rd
    };
    if h < 0 {
        h += HLSMAX;
    }
    if h >= HLSMAX {
        h -= HLSMAX;
    }
    (h, l, s)
}

fn hue_to_rgb(n1: i64, n2: i64, mut hue: i64) -> i64 {
    if hue < 0 {
        hue += HLSMAX;
    }
    if hue > HLSMAX {
        hue -= HLSMAX;
    }
    if hue < HLSMAX / 6 {
        n1 + ((n2 - n1) * hue + HLSMAX / 12) / (HLSMAX / 6)
    } else if hue < HLSMAX / 2 {
        n2
    } else if hue < HLSMAX * 2 / 3 {
        n1 + ((n2 - n1) * (HLSMAX * 2 / 3 - hue) + HLSMAX / 12) / (HLSMAX / 6)
    } else {
        n1
    }
}

/// &HBBGGRR of Windows' HLS (the "HLS to RGB" of Windows' samples).
pub fn from_hls(h: i64, l: i64, s: i64) -> i64 {
    let (h, l, s) = (h.clamp(0, HLSMAX - 1), l.clamp(0, HLSMAX), s.clamp(0, HLSMAX));
    let (r, g, b) = if s == 0 {
        let v = l * RGBMAX / HLSMAX;
        (v, v, v)
    } else {
        let m2 = if l <= HLSMAX / 2 { (l * (HLSMAX + s) + HLSMAX / 2) / HLSMAX } else { l + s - (l * s + HLSMAX / 2) / HLSMAX };
        let m1 = 2 * l - m2;
        let c = |hue: i64| ((hue_to_rgb(m1, m2, hue) * RGBMAX + HLSMAX / 2) / HLSMAX).clamp(0, 255);
        (c(h + HLSMAX / 3), c(h), c(h - HLSMAX / 3))
    };
    b << 16 | g << 8 | r
}

/// The hue / saturation field as RGBA, `w` × `h` pixels: the hue across
/// (0 at the left), the saturation up (240 at the top), luminance 120.
pub fn spectrum_rgba(w: usize, h: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        let s = HLSMAX - (y as i64 * HLSMAX) / (h.max(2) as i64 - 1);
        for x in 0..w {
            let hue = (x as i64 * (HLSMAX - 1)) / (w.max(2) as i64 - 1);
            let c = from_hls(hue, HLSMAX / 2, s);
            out.extend_from_slice(&[(c & 0xFF) as u8, (c >> 8 & 0xFF) as u8, (c >> 16 & 0xFF) as u8, 255]);
        }
    }
    out
}

/// The luminance bar for `hue` and `sat` as RGBA, `w` × `h` pixels: white
/// at the top (240), black at the bottom.
pub fn lum_rgba(w: usize, h: usize, hue: i64, sat: i64) -> Vec<u8> {
    let mut out = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        let l = HLSMAX - (y as i64 * HLSMAX) / (h.max(2) as i64 - 1);
        let c = from_hls(hue, l, sat);
        let px = [(c & 0xFF) as u8, (c >> 8 & 0xFF) as u8, (c >> 16 & 0xFF) as u8, 255];
        for _ in 0..w {
            out.extend_from_slice(&px);
        }
    }
    out
}

/// A rectangle (x, y, width, height), logical pixels.
pub type Rect = (i64, i64, i64, i64);

/// Where the dialog's parts go, as Windows' ChooseColor places them
/// (logical pixels; the compact dialog is the left part).
pub mod layout {
    use super::Rect;

    /// The window's inside, compact and with the editor.
    pub const COMPACT: (i64, i64) = (220, 285);
    pub const FULL: (i64, i64) = (442, 285);
    pub const BASIC_LABEL: Rect = (6, 6, 200, 14);
    pub const CUSTOM_LABEL: Rect = (6, 162, 200, 14);
    /// Basic colour `i`'s swatch (eight a row, 20 × 16 in a 26 × 22 cell).
    pub fn basic(i: usize) -> Rect {
        let (c, r) = ((i % 8) as i64, (i / 8) as i64);
        (8 + c * 26, 24 + r * 22, 20, 16)
    }
    /// Custom colour `i`'s swatch.
    pub fn custom(i: usize) -> Rect {
        let (c, r) = ((i % 8) as i64, (i / 8) as i64);
        (8 + c * 26, 180 + r * 22, 20, 16)
    }
    /// The frame around the chosen swatch.
    pub fn frame(swatch: Rect) -> Rect {
        (swatch.0 - 2, swatch.1 - 2, swatch.2 + 4, swatch.3 + 4)
    }
    pub const DEFINE: Rect = (6, 226, 208, 23);
    pub const OK: Rect = (6, 256, 66, 23);
    pub const CANCEL: Rect = (82, 256, 66, 23);
    // (the editor, right of the compact part)
    pub const SPECTRUM: Rect = (226, 6, 175, 180);
    /// The luminance bar and its arrow (the bar 10 wide at its left, the
    /// arrow right of it); the bar from 3 to 3 + SPECTRUM's height.
    pub const LUM: Rect = (407, 3, 24, 186);
    pub const LUM_BAR_W: i64 = 10;
    pub const PREVIEW: Rect = (226, 192, 60, 38);
    /// ("Color|Solid", centred under the sample)
    pub const PREVIEW_LABEL: Rect = (220, 232, 72, 14);
    /// The Hue, Sat, Lum and Red, Green, Blue boxes' labels and edits.
    pub fn field(i: usize) -> (Rect, Rect) {
        let (col, row) = ((i / 3) as i64, (i % 3) as i64);
        let y = 192 + row * 21;
        let x = 292 + col * 76;
        ((x, y + 3, 40, 14), (x + 40, y, 30, 19))
    }
    pub const ADD: Rect = (226, 256, 210, 23);
}

/// The Hue … Blue boxes' captions, in [`layout::field`]'s order.
pub const FIELDS: [&str; 6] = ["Hu&e:", "&Sat:", "&Lum:", "&Red:", "&Green:", "Bl&ue:"];

/// A colour dialog's state: what the user sees and changes.
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    /// The colour chosen (&HBBGGRR).
    pub color: i64,
    /// Its HLS (kept apart: a grey keeps the hue the user gave it).
    pub hls: (i64, i64, i64),
    pub custom: [i64; 16],
    /// The custom colour "Add to Custom Colors" fills next.
    pub slot: usize,
    pub mode: Mode,
}

impl State {
    pub fn new(color: i64, custom: [i64; 16], style: i64) -> State {
        let color = color & 0xFF_FFFF;
        State { color, hls: to_hls(color), custom, slot: 0, mode: mode(style) }
    }

    pub fn full(&self) -> bool {
        self.mode == Mode::Full
    }

    /// "Define Custom Colors >>" (when allowed): the editor opens.
    pub fn define(&mut self) -> bool {
        if self.mode != Mode::Compact {
            return false;
        }
        self.mode = Mode::Full;
        true
    }

    pub fn set_color(&mut self, bgr: i64) {
        self.color = bgr & 0xFF_FFFF;
        self.hls = to_hls(self.color);
    }

    /// A basic colour's swatch clicked.
    pub fn pick_basic(&mut self, i: usize) {
        if let Some(rgb) = BASIC_COLORS.get(i) {
            self.set_color(swap_rb(i64::from(*rgb)));
        }
    }

    /// A custom colour's swatch clicked: its colour, and the box "Add to
    /// Custom Colors" fills.
    pub fn pick_custom(&mut self, i: usize) {
        if let Some(c) = self.custom.get(i).copied() {
            self.set_color(c);
            self.slot = i;
        }
    }

    pub fn set_hls(&mut self, h: i64, l: i64, s: i64) {
        self.hls = (h.clamp(0, HLSMAX - 1), l.clamp(0, HLSMAX), s.clamp(0, HLSMAX));
        self.color = from_hls(self.hls.0, self.hls.1, self.hls.2);
    }

    /// A click at (x, y) of the hue / saturation field, `w` × `h`: its hue
    /// and saturation, the luminance kept.
    pub fn pick_spectrum(&mut self, x: f64, y: f64, w: f64, h: f64) {
        let hue = ((x / (w - 1.0).max(1.0)) * (HLSMAX - 1) as f64).round() as i64;
        let sat = HLSMAX - ((y / (h - 1.0).max(1.0)) * HLSMAX as f64).round() as i64;
        self.set_hls(hue, self.hls.1, sat);
    }

    /// A click at `y` of the luminance bar, `h` high.
    pub fn pick_lum(&mut self, y: f64, h: f64) {
        let lum = HLSMAX - ((y / (h - 1.0).max(1.0)) * HLSMAX as f64).round() as i64;
        self.set_hls(self.hls.0, lum, self.hls.2);
    }

    /// The Red, Green, Blue boxes' values.
    pub fn rgb(&self) -> (i64, i64, i64) {
        (self.color & 0xFF, self.color >> 8 & 0xFF, self.color >> 16 & 0xFF)
    }

    /// Box `i` ([`FIELDS`]' order) typed `v`.
    pub fn set_field(&mut self, i: usize, v: i64) {
        let (h, l, s) = self.hls;
        let (r, g, b) = self.rgb();
        match i {
            0 => self.set_hls(v, l, s),
            1 => self.set_hls(h, l, v),
            2 => self.set_hls(h, v, s),
            _ => {
                let v = v.clamp(0, 255);
                let (r, g, b) = match i {
                    3 => (v, g, b),
                    4 => (r, v, b),
                    _ => (r, g, v),
                };
                self.set_color(b << 16 | g << 8 | r);
            }
        }
    }

    /// The boxes' values, [`FIELDS`]' order.
    pub fn fields(&self) -> [i64; 6] {
        let (h, l, s) = self.hls;
        let (r, g, b) = self.rgb();
        [h, s, l, r, g, b]
    }

    /// "Add to Custom Colors": the colour into the next custom box.
    pub fn add_custom(&mut self) {
        self.custom[self.slot] = self.color;
        self.slot = (self.slot + 1) % 16;
    }

    /// Which swatch shows the chosen colour: a basic one first, else a
    /// custom one (`(false, i)` basic, `(true, i)` custom).
    pub fn marked(&self) -> Option<(bool, usize)> {
        let basic = BASIC_COLORS.iter().position(|rgb| swap_rb(i64::from(*rgb)) == self.color).map(|i| (false, i));
        basic.or_else(|| self.custom.iter().position(|c| *c == self.color).map(|i| (true, i)))
    }
}

/// The custom colours from a `Colors(i)` reader (1 to 16; a missing one:
/// RAPIDQ2.INC's default).
pub fn custom_colors(get: impl Fn(usize) -> Option<i64>) -> [i64; 16] {
    let mut out = DEFAULT_CUSTOM;
    for (i, c) in out.iter_mut().enumerate() {
        if let Some(v) = get(i + 1) {
            *c = v & 0xFF_FFFF;
        }
    }
    out
}

/// `RAPIDR_TEST_COLOR_DIALOG`'s answers: one per Execute, `;`-separated —
/// a colour (decimal, `&H…` or `#RRGGBB`) for OK, empty for Cancel.
pub fn parse_answers(list: &str) -> Vec<Option<i64>> {
    list.split(';').map(|a| parse_color(a.trim())).collect()
}

/// A colour as a test writes it: decimal, `&HBBGGRR`, or `#RRGGBB`.
pub fn parse_color(s: &str) -> Option<i64> {
    if s.is_empty() {
        return None;
    }
    if let Some(hex) = s.strip_prefix("&H").or_else(|| s.strip_prefix("&h")) {
        return i64::from_str_radix(hex, 16).ok().map(|c| c & 0xFF_FFFF);
    }
    if let Some(hex) = s.strip_prefix('#') {
        return i64::from_str_radix(hex, 16).ok().map(swap_rb);
    }
    s.parse::<i64>().ok().map(|c| c & 0xFF_FFFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styles() {
        assert_eq!(mode(CD_NORMAL), Mode::Full);
        assert_eq!(mode(CD_FULL_OPEN), Mode::Full);
        assert_eq!(mode(CD_NO_FULL_OPEN), Mode::Prevented);
        assert_eq!(mode(3), Mode::Compact);
        let mut s = State::new(0, DEFAULT_CUSTOM, 3);
        assert!(s.define() && s.full());
        assert!(!State::new(0, DEFAULT_CUSTOM, 2).define());
    }

    #[test]
    fn windows_hls() {
        // (what ChooseColor's boxes show: red 0/240/120, black 160/0/0)
        assert_eq!(to_hls(0x0000FF), (0, 120, 240));
        assert_eq!(to_hls(0x000000), (160, 0, 0));
        assert_eq!(to_hls(0xFFFFFF), (160, 240, 0));
        assert_eq!(to_hls(0x00FF00), (80, 120, 240));
        assert_eq!(to_hls(0xFF0000), (160, 120, 240));
        assert_eq!(to_hls(0x808080), (160, 120, 0));
        // every basic colour comes back from its HLS within a step
        for rgb in BASIC_COLORS {
            let bgr = swap_rb(i64::from(rgb));
            let (h, l, s) = to_hls(bgr);
            let back = from_hls(h, l, s);
            for k in [0, 8, 16] {
                assert!(((back >> k & 0xFF) - (bgr >> k & 0xFF)).abs() <= 3, "{rgb:06x} → {h},{l},{s} → {back:06x}");
            }
        }
    }

    #[test]
    fn pictures() {
        let p = spectrum_rgba(10, 5);
        assert_eq!(p.len(), 10 * 5 * 4);
        // the top left is pure red, the bottom row grey
        assert_eq!(&p[0..4], &[255, 0, 0, 255]);
        let bottom = &p[4 * 10 * 4..];
        assert!(bottom.chunks(4).all(|px| px[0] == px[1] && px[1] == px[2]));
        let l = lum_rgba(2, 3, 0, 240);
        assert_eq!(&l[0..4], &[255, 255, 255, 255]);
        // (the middle row: luminance 120, the pure hue)
        assert_eq!(&l[8..12], &[255, 0, 0, 255]);
        assert_eq!(&l[l.len() - 4..], &[0, 0, 0, 255]);
    }

    #[test]
    fn the_user_changes_it() {
        let mut s = State::new(0x0000FF, DEFAULT_CUSTOM, CD_FULL_OPEN);
        assert_eq!(s.marked(), Some((false, 8)));
        assert_eq!(s.fields(), [0, 240, 120, 255, 0, 0]);
        // the field's top right: hue 239, full saturation
        s.pick_spectrum(174.0, 0.0, 175.0, 180.0);
        assert_eq!((s.hls.0, s.hls.2), (239, 240));
        // the luminance bar's top: white
        s.pick_lum(0.0, 180.0);
        assert_eq!(s.color, 0xFFFFFF);
        s.set_field(3, 128);
        assert_eq!(s.rgb(), (128, 255, 255));
        s.set_field(2, 0);
        assert_eq!(s.color, 0);
        // added to the custom colours from the first box on; a custom
        // swatch clicked is the next box
        s.set_color(0x123456);
        s.add_custom();
        assert_eq!((s.custom[0], s.slot), (0x123456, 1));
        s.pick_custom(5);
        assert_eq!((s.color, s.slot), (DEFAULT_CUSTOM[5], 5));
        s.pick_basic(18);
        assert_eq!(s.color, 0x00FF00);
    }

    #[test]
    fn answers_and_colours() {
        assert_eq!(parse_answers("255;;&HFF00;#0000FF"), vec![Some(255), None, Some(0xFF00), Some(0xFF0000)]);
        let c = custom_colors(|i| (i == 3).then_some(0x123456));
        assert_eq!((c[0], c[2], c[15]), (0, 0x123456, 0xFF00FF));
        assert_eq!(swap_rb(0x0000FF), 0xFF0000);
    }
}
