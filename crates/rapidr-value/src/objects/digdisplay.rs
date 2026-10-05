//! QDIGDISPLAY, RapidQ's seven-segment LED display (the manual's Appendix
//! A; Peter Molloy's `QDigDisplay.inc`, a TYPE extending QCANVAS). RapidR's
//! built-in is that canvas with the include's rules: `Display` (default
//! "0") is shown one 12 × 24 character cell after another, characters 32
//! (space) to 64 (`@`) — the include's 33 pictures — and the control is
//! 12 × LEN(Display) wide and 24 high. The cells are drawn as the
//! include's pictures look (a black cell, lit segments cyan, unlit ones a
//! dark green dither), by this code: the pictures aren't RapidR's to ship.
//! A program that includes `QDigDisplay.inc` gets the include's own TYPE
//! (with its own bitmaps) instead.
//!
//! Differences, by choice: the include draws its digits at its own Left /
//! Top inside itself (so a display not at 0, 0 cuts them off) and only from
//! its OnPaint (a program's OnPaint replaces it); the built-in draws them
//! at the cell's corner whenever Display changes or the canvas is painted,
//! before the program's OnPaint.

use super::bitmap::Bitmap;

/// A character cell's size.
pub const CELL_W: i64 = 12;
pub const CELL_H: i64 = 24;

/// The colours (RapidQ's &HBBGGRR): the cell, a lit segment, an unlit one.
pub const BACK: u32 = 0x000000;
pub const LIT: u32 = 0xFFFF00;
pub const UNLIT: u32 = 0x008000;

/// The control's size for `display`.
pub fn size(display: &str) -> (i64, i64) {
    (CELL_W * display.chars().count() as i64, CELL_H)
}

/// A segment's pixels as (x, first row, last row) columns or
/// (y, first column, last column) rows — beveled ends, three pixels thick.
fn segment(s: char) -> Vec<(i64, i64)> {
    let mut px = Vec::new();
    let row = |y: i64, x1: i64, x2: i64, px: &mut Vec<(i64, i64)>| px.extend((x1..=x2).map(|x| (x, y)));
    match s {
        'a' => {
            row(2, 2, 9, &mut px);
            row(3, 3, 8, &mut px);
            row(4, 4, 7, &mut px);
        }
        'g' => {
            row(11, 3, 8, &mut px);
            row(12, 2, 9, &mut px);
            row(13, 3, 8, &mut px);
        }
        'd' => {
            row(20, 4, 7, &mut px);
            row(21, 3, 8, &mut px);
            row(22, 2, 9, &mut px);
        }
        _ => {
            // (the four uprights: x of the outer, middle and inner column)
            let (xs, top) = match s {
                'f' => ([1, 2, 3], 3),
                'b' => ([10, 9, 8], 3),
                'e' => ([1, 2, 3], 13),
                _ => ([10, 9, 8], 13), // 'c'
            };
            for (i, x) in xs.iter().enumerate() {
                let i = i as i64;
                px.extend((top + i..=top + 8 - i).map(|y| (*x, y)));
            }
        }
    }
    px
}

/// The lit segments and extra marks of character `ch` (`None`: outside
/// 32 … 64, drawn as nothing).
/// A mark: (x, y, width, height) lit.
type Marks = &'static [(i64, i64, i64, i64)];

fn glyph(ch: char) -> Option<(&'static str, Marks)> {
    // (marks: x, y, width, height)
    const DOT: &[(i64, i64, i64, i64)] = &[(4, 20, 3, 3)];
    const COMMA: &[(i64, i64, i64, i64)] = &[(5, 19, 2, 3), (4, 22, 2, 1)];
    const COLON: &[(i64, i64, i64, i64)] = &[(4, 6, 3, 3), (4, 16, 3, 3)];
    const SEMI: &[(i64, i64, i64, i64)] = &[(4, 6, 3, 3), (5, 16, 2, 3), (4, 19, 2, 1)];
    const BANG: &[(i64, i64, i64, i64)] = &[(8, 20, 3, 3)];
    Some(match ch {
        ' ' | '#' | '&' | '@' => ("", &[]),
        '!' => ("b", BANG),
        '"' => ("bf", &[]),
        '$' => ("acdfg", &[]),
        '%' => ("bceg", &[]),
        '\'' => ("f", &[]),
        '(' => ("adef", &[]),
        ')' => ("abcd", &[]),
        '*' | '+' | '-' => ("g", &[]),
        ',' => ("", COMMA),
        '.' => ("", DOT),
        '/' => ("beg", &[]),
        '0' => ("abcdef", &[]),
        '1' => ("bc", &[]),
        '2' => ("abdeg", &[]),
        '3' => ("abcdg", &[]),
        '4' => ("bcfg", &[]),
        '5' => ("acdfg", &[]),
        '6' => ("acdefg", &[]),
        '7' => ("abc", &[]),
        '8' => ("abcdefg", &[]),
        '9' => ("abcdfg", &[]),
        ':' => ("", COLON),
        ';' => ("", SEMI),
        '<' => ("deg", &[]),
        '=' => ("dg", &[]),
        '>' => ("cdg", &[]),
        '?' => ("abeg", &[]),
        _ => return None,
    })
}

/// Draws `display` onto a digit display's canvas (already its size).
pub fn draw(b: &mut Bitmap, display: &str) {
    for (i, ch) in display.chars().enumerate() {
        let x0 = i as i64 * CELL_W;
        let Some((lit, marks)) = glyph(ch) else { continue };
        b.fill_rect(x0, 0, x0 + CELL_W, CELL_H, BACK);
        for s in "abcdefg".chars() {
            let on = lit.contains(s);
            for (x, y) in segment(s) {
                if on {
                    b.pset(x0 + x, y, LIT);
                } else if (x + y) % 2 == 0 {
                    b.pset(x0 + x, y, UNLIT);
                }
            }
        }
        for &(x, y, w, h) in marks {
            b.fill_rect(x0 + x, y, x0 + x + w, y + h, LIT);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_eight_lights_every_segment_and_a_space_none() {
        let mut b = Bitmap::new_canvas();
        b.fit(24, 24);
        draw(&mut b, "8 ");
        // a's middle, g's middle, f's middle column
        assert_eq!(b.pixel(5, 2), Some(LIT));
        assert_eq!(b.pixel(5, 12), Some(LIT));
        assert_eq!(b.pixel(1, 7), Some(LIT));
        // the space: black, the segments dithered
        assert_eq!(b.pixel(12 + 5, 12), Some(BACK));
        assert_eq!(b.pixel(12 + 4, 12), Some(UNLIT));
        assert_eq!(size("12:30"), (60, 24));
    }
}
