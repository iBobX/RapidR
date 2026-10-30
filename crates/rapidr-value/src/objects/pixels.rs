//! `SetPixels`, `SetPixels8` and `GetPixels` (RapidR's, on an RBitmap, an
//! RCanvas and an RImage's picture): a whole block of a picture to or from
//! an array in one call, for programs that draw an image from data — an
//! emulator's video memory, an image filter, a heat map, a game of life.
//! `Pset` costs a call a pixel (1 to 3 µs in the interpreter: 50 to 150 ms
//! for a 256 × 192 screen); these cost one call a frame, a row of copies in
//! Rust (well under a millisecond).
//!
//! Colours are the integers `Pset` takes: `RGB(r, g, b)`, that is
//! `&HBBGGRR` (red in the low byte), and a system colour as always. The
//! array's elements are taken in order, row by row, from its first element
//! whatever its lower bound is: a one-dimensional array's index
//! `y * Width + x` is the pixel (x, y); a two-dimensional `(Height - 1,
//! Width - 1)` array — RapidQ stores the last subscript fastest — is
//! `Pixels(y, x)`. A call that can't be answered stops the program with a
//! message that says why (never a quietly wrong picture): not an array, an
//! array too short, a block that does not fit the bitmap, an index that
//! is not in the palette.

use super::bitmap::Bitmap;
use super::codec::MAX_PIXELS;
use crate::{exception, BasicArray, Value};
use std::cell::RefCell;
use std::rc::Rc;

/// Whether `method` is one of these (the registry's names, lowercase).
pub fn is_pixel_block_method(method: &str) -> bool {
    matches!(method, "setpixels" | "setpixels8" | "getpixels")
}

type Array = Rc<RefCell<BasicArray>>;

fn fail(method: &str, why: String) -> String {
    exception(&format!("{method}: {why}"))
}

fn kind(v: Option<&Value>) -> &'static str {
    match v {
        None => "nothing",
        Some(Value::Integer(_) | Value::Double(_)) => "a number",
        Some(Value::String(_)) => "a string",
        Some(Value::Boolean(_)) => "a boolean",
        Some(Value::Object(_)) => "an object",
        Some(Value::Array(_)) => "an array",
        Some(Value::Null) => "nothing",
    }
}

/// An array argument.
fn array<'a>(method: &str, what: &str, v: Option<&'a Value>) -> Result<&'a Array, String> {
    match v {
        Some(Value::Array(a)) => Ok(a),
        other => Err(fail(method, format!("{what} must be an array (DIM {what}(…) AS LONG), not {}", kind(other)))),
    }
}

/// A whole number argument, `min` or more.
fn number(method: &str, what: &str, v: Option<&Value>, min: i64) -> Result<i64, String> {
    let Some(v) = v else {
        return Err(fail(method, format!("{what} is missing")));
    };
    let n = v.to_i64();
    if n < min {
        return Err(fail(method, format!("{what} is {n}; it must be {min} or more")));
    }
    Ok(n)
}

/// The colour an array element holds: `Pset`'s own reading of a number.
#[inline]
fn colour(v: &Value) -> u32 {
    let n = match v {
        Value::Integer(n) => *n,
        other => other.to_i64(),
    };
    let c = n as u32;
    if c <= 0x00FF_FFFF {
        c
    } else {
        super::color_bgr(n)
    }
}

/// The block (x, y, w, h) must lie in the bitmap; `w * h`, how many
/// elements it takes.
fn fit(method: &str, b: &Bitmap, (x, y, w, h): (i64, i64, i64, i64)) -> Result<usize, String> {
    let (bw, bh) = (b.img.width as i64, b.img.height as i64);
    if bw == 0 || bh == 0 {
        return Err(fail(method, "the bitmap has no pixels yet (set its Width and Height first)".into()));
    }
    if x < 0 || y < 0 || x.saturating_add(w) > bw || y.saturating_add(h) > bh {
        return Err(fail(method, format!("a {w} x {h} block at ({x}, {y}) does not fit in the {bw} x {bh} bitmap")));
    }
    let count = (w as usize).checked_mul(h as usize).filter(|n| *n <= MAX_PIXELS);
    count.ok_or_else(|| fail(method, format!("a {w} x {h} block is too large")))
}

/// How many elements an array must hold for `need`, said when it does not.
fn enough(method: &str, what: &str, held: usize, need: usize, w: i64, h: i64) -> Result<(), String> {
    if held < need {
        return Err(fail(method, format!("{what} holds {held} values, but a {w} x {h} block needs {need}")));
    }
    Ok(())
}

/// `SetPixels(Pixels, Width, Height [, X, Y])`: the `Width` × `Height`
/// colours of `Pixels` (row by row) onto the bitmap, top-left corner at
/// (X, Y) — 0, 0 if left out.
pub fn set_pixels(b: &mut Bitmap, args: &[Value]) -> Result<(), String> {
    const M: &str = "SetPixels";
    if args.len() < 3 {
        return Err(fail(M, "needs Pixels, Width and Height (and, if wanted, X and Y)".into()));
    }
    let pixels = array(M, "Pixels", args.first())?;
    let (w, h) = (number(M, "Width", args.get(1), 0)?, number(M, "Height", args.get(2), 0)?);
    let (x, y) = (number(M, "X", args.get(3).or(Some(&Value::Integer(0))), i64::MIN)?, number(M, "Y", args.get(4).or(Some(&Value::Integer(0))), i64::MIN)?);
    let count = fit(M, b, (x, y, w, h))?;
    let block: Vec<u32> = {
        let a = pixels.borrow();
        enough(M, "Pixels", a.data.len(), count, w, h)?;
        a.data[..count].iter().map(colour).collect()
    };
    b.put_block(x as usize, y as usize, w as usize, h as usize, &block);
    Ok(())
}

/// `SetPixels8(Indexes, Palette, Width, Height [, X, Y])`: like
/// `SetPixels`, but each element of `Indexes` (0 to 255, a BYTE array) is
/// the number of a colour in `Palette` — the first element of `Palette` is
/// colour 0. One lookup a pixel, in Rust.
pub fn set_pixels8(b: &mut Bitmap, args: &[Value]) -> Result<(), String> {
    const M: &str = "SetPixels8";
    if args.len() < 4 {
        return Err(fail(M, "needs Indexes, Palette, Width and Height (and, if wanted, X and Y)".into()));
    }
    let indexes = array(M, "Indexes", args.first())?;
    let palette = array(M, "Palette", args.get(1))?;
    let (w, h) = (number(M, "Width", args.get(2), 0)?, number(M, "Height", args.get(3), 0)?);
    let (x, y) = (number(M, "X", args.get(4).or(Some(&Value::Integer(0))), i64::MIN)?, number(M, "Y", args.get(5).or(Some(&Value::Integer(0))), i64::MIN)?);
    let count = fit(M, b, (x, y, w, h))?;
    let colours: Vec<u32> = palette.borrow().data.iter().map(colour).collect();
    let block: Vec<u32> = {
        let a = indexes.borrow();
        enough(M, "Indexes", a.data.len(), count, w, h)?;
        let mut out = Vec::with_capacity(count);
        for (i, v) in a.data[..count].iter().enumerate() {
            let n = match v {
                Value::Integer(n) => *n,
                other => other.to_i64(),
            };
            match usize::try_from(n).ok().and_then(|n| colours.get(n)) {
                Some(&c) => out.push(c),
                None => {
                    let (px, py) = (i as i64 % w.max(1), i as i64 / w.max(1));
                    return Err(fail(M, format!("the pixel ({px}, {py}) of the block has index {n}, but Palette has {} colours (indexes 0 to {})", colours.len(), colours.len() as i64 - 1)));
                }
            }
        }
        out
    };
    b.put_block(x as usize, y as usize, w as usize, h as usize, &block);
    Ok(())
}

/// `GetPixels(Pixels, X, Y, Width, Height)`: the `Width` × `Height` block
/// of the bitmap with its top-left corner at (X, Y), row by row, into the
/// first `Width * Height` elements of `Pixels`.
pub fn get_pixels(b: &Bitmap, args: &[Value]) -> Result<(), String> {
    const M: &str = "GetPixels";
    if args.len() < 5 {
        return Err(fail(M, "needs Pixels, X, Y, Width and Height".into()));
    }
    let pixels = array(M, "Pixels", args.first())?;
    let (x, y) = (number(M, "X", args.get(1), i64::MIN)?, number(M, "Y", args.get(2), i64::MIN)?);
    let (w, h) = (number(M, "Width", args.get(3), 0)?, number(M, "Height", args.get(4), 0)?);
    let count = fit(M, b, (x, y, w, h))?;
    let mut a = pixels.borrow_mut();
    enough(M, "Pixels", a.data.len(), count, w, h)?;
    let block = b.read_block(x as usize, y as usize, w as usize, h as usize);
    for (slot, c) in a.data[..count].iter_mut().zip(block) {
        *slot = Value::Integer(i64::from(c));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arr(n: usize, f: impl Fn(usize) -> i64) -> Value {
        let mut a = BasicArray::new(vec![(0, n as i64 - 1)], Value::Integer(0)).unwrap();
        for (i, slot) in a.data.iter_mut().enumerate() {
            *slot = Value::Integer(f(i));
        }
        Value::Array(Rc::new(RefCell::new(a)))
    }

    fn bitmap(w: i64, h: i64) -> Bitmap {
        let mut b = Bitmap::default();
        b.resize(w, h);
        b
    }

    fn msg(e: String) -> String {
        crate::exception_message(&e).unwrap_or(&e).to_string()
    }

    #[test]
    fn a_block_is_put_where_asked_and_read_back() {
        let mut b = bitmap(8, 6);
        let px = arr(6, |i| 100 + i as i64);
        set_pixels(&mut b, &[px, Value::Integer(3), Value::Integer(2), Value::Integer(2), Value::Integer(1)]).unwrap();
        assert_eq!((b.pixel(2, 1), b.pixel(4, 1), b.pixel(2, 2), b.pixel(4, 2)), (Some(100), Some(102), Some(103), Some(105)));
        assert_eq!((b.pixel(1, 1), b.pixel(5, 1), b.pixel(2, 0), b.pixel(2, 3)), (Some(0xFFFFFF), Some(0xFFFFFF), Some(0xFFFFFF), Some(0xFFFFFF)));
        let back = arr(6, |_| 0);
        get_pixels(&b, &[back.clone(), Value::Integer(2), Value::Integer(1), Value::Integer(3), Value::Integer(2)]).unwrap();
        let Value::Array(a) = back else { unreachable!() };
        let got: Vec<i64> = a.borrow().data.iter().map(Value::to_i64).collect();
        assert_eq!(got, [100, 101, 102, 103, 104, 105]);
    }

    #[test]
    fn the_whole_bitmap_is_the_default_place() {
        let mut b = bitmap(4, 2);
        set_pixels(&mut b, &[arr(8, |i| i as i64 * 3), Value::Integer(4), Value::Integer(2)]).unwrap();
        assert_eq!((b.pixel(0, 0), b.pixel(3, 0), b.pixel(0, 1), b.pixel(3, 1)), (Some(0), Some(9), Some(12), Some(21)));
        // a colour is Pset's own: 24 bits, a system colour, -1 white
        set_pixels(&mut b, &[arr(8, |i| if i == 0 { -1 } else { 0x1_00_00_00_FF }), Value::Integer(4), Value::Integer(2)]).unwrap();
        assert_eq!((b.pixel(0, 0), b.pixel(1, 0)), (Some(0xFFFFFF), Some(0xFF)));
    }

    #[test]
    fn indexed_pixels_look_their_colour_up() {
        let mut b = bitmap(4, 2);
        let pal = arr(3, |i| [0x0000FF, 0x00FF00, 0xFF0000][i]);
        set_pixels8(&mut b, &[arr(4, |i| [2, 1, 0, 1][i]), pal.clone(), Value::Integer(2), Value::Integer(2), Value::Integer(2), Value::Integer(0)]).unwrap();
        assert_eq!((b.pixel(2, 0), b.pixel(3, 0), b.pixel(2, 1), b.pixel(3, 1), b.pixel(1, 0)), (Some(0xFF0000), Some(0x00FF00), Some(0x0000FF), Some(0x00FF00), Some(0xFFFFFF)));
        let e = set_pixels8(&mut b, &[arr(4, |i| [0, 3, 0, 0][i]), pal, Value::Integer(2), Value::Integer(2)]).unwrap_err();
        assert_eq!(msg(e), "SetPixels8: the pixel (1, 0) of the block has index 3, but Palette has 3 colours (indexes 0 to 2)");
    }

    #[test]
    fn mistakes_are_said_plainly() {
        let mut b = bitmap(4, 4);
        let n = |v| Value::Integer(v);
        let e = |r: Result<(), String>| msg(r.unwrap_err());
        assert_eq!(e(set_pixels(&mut b, &[Value::Integer(5), n(2), n(2)])), "SetPixels: Pixels must be an array (DIM Pixels(…) AS LONG), not a number");
        assert_eq!(e(set_pixels(&mut b, &[arr(3, |_| 0), n(2), n(2)])), "SetPixels: Pixels holds 3 values, but a 2 x 2 block needs 4");
        assert_eq!(e(set_pixels(&mut b, &[arr(4, |_| 0), n(2), n(2), n(3), n(0)])), "SetPixels: a 2 x 2 block at (3, 0) does not fit in the 4 x 4 bitmap");
        assert_eq!(e(set_pixels(&mut b, &[arr(4, |_| 0), n(2), n(2), n(-1), n(0)])), "SetPixels: a 2 x 2 block at (-1, 0) does not fit in the 4 x 4 bitmap");
        assert_eq!(e(set_pixels(&mut b, &[arr(4, |_| 0), n(-2), n(2)])), "SetPixels: Width is -2; it must be 0 or more");
        assert_eq!(e(set_pixels(&mut b, &[arr(4, |_| 0)])), "SetPixels: needs Pixels, Width and Height (and, if wanted, X and Y)");
        assert_eq!(e(get_pixels(&b, &[arr(2, |_| 0), n(0), n(0), n(2), n(2)])), "GetPixels: Pixels holds 2 values, but a 2 x 2 block needs 4");
        assert_eq!(e(get_pixels(&b, &[arr(4, |_| 0), n(3), n(3), n(2), n(2)])), "GetPixels: a 2 x 2 block at (3, 3) does not fit in the 4 x 4 bitmap");
        let mut empty = Bitmap::default();
        assert_eq!(e(set_pixels(&mut empty, &[arr(4, |_| 0), n(2), n(2)])), "SetPixels: the bitmap has no pixels yet (set its Width and Height first)");
        // (nothing was drawn by any of them)
        assert!((0..4).all(|y| (0..4).all(|x| b.pixel(x, y) == Some(0xFFFFFF))));
        // an empty block is nothing, not an error
        set_pixels(&mut b, &[arr(1, |_| 0), n(0), n(0)]).unwrap();
    }

    /// On a Retina screen the bitmap keeps what it shows finer: a block is
    /// put there too, so what is shown is the pixels, not the old picture.
    #[test]
    fn what_the_screen_shows_follows() {
        super::super::bitmap::set_display_scale(2.0);
        let mut b = bitmap(3, 2);
        b.fill_rect(0, 0, 3, 2, 0x112233);
        let _ = b.display_rgba();
        set_pixels(&mut b, &[arr(4, |i| 10 + i as i64), Value::Integer(2), Value::Integer(2), Value::Integer(1), Value::Integer(0)]).unwrap();
        let (w, h, rgba, scale) = b.display_rgba();
        assert_eq!((w, h, scale), (6, 4, 2));
        let at = |x: usize, y: usize| rgba[(y * w + x) * 4] as u32;
        // pixel (1, 0) is 10: its 2 x 2 square; pixel (0, 0) is untouched
        assert_eq!((at(2, 0), at(3, 0), at(2, 1), at(3, 1)), (10, 10, 10, 10));
        assert_eq!((at(4, 2), at(5, 3)), (13, 13));
        assert_eq!(at(0, 0), 0x33);
        super::super::bitmap::set_display_scale(1.0);
    }
}
