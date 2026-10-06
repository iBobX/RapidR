//! `Form.ShapeForm(Filename$|Resource, TransparentColor&)`: the form's
//! outline made of a bitmap's pixels that aren't the transparent colour —
//! the model every runtime shows (the desktop host gives the window that
//! outline, the web host clips the window's element to it).
//!
//! What RC.EXE's programs do (a probe in the Windows VM reading the
//! window's region back): the bitmap's pixel (x, y) is the window's
//! (x, y) — counted from the window's top left corner, its frame included,
//! not from the client area; everything outside the bitmap is cut away;
//! the form's Width, Height, ClientWidth, ClientHeight and BorderStyle
//! don't change, and the outline stays when the form is resized. A later
//! ShapeForm replaces it. A file that can't be read is an error.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::objects::bitmap::Bitmap;

/// A form's outline: per row of the bitmap, the runs of pixels kept
/// (`x0..x1`, end exclusive).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shape {
    pub width: i64,
    pub height: i64,
    pub rows: Vec<Vec<(i64, i64)>>,
}

impl Shape {
    /// The outline of `bitmap`'s pixels that aren't `transparent` (a
    /// RapidQ colour, &HBBGGRR).
    pub fn of(bitmap: &Bitmap, transparent: u32) -> Shape {
        let img = &bitmap.img;
        let transparent = transparent & 0xFF_FFFF;
        let rows = (0..img.height)
            .map(|y| {
                let row = &img.pixels[y * img.width..(y + 1) * img.width];
                let mut runs = Vec::new();
                let mut start = None;
                for (x, &c) in row.iter().enumerate() {
                    let keep = c & 0xFF_FFFF != transparent;
                    match (keep, start) {
                        (true, None) => start = Some(x as i64),
                        (false, Some(s)) => {
                            runs.push((s, x as i64));
                            start = None;
                        }
                        _ => {}
                    }
                }
                if let Some(s) = start {
                    runs.push((s, img.width as i64));
                }
                runs
            })
            .collect();
        Shape { width: img.width as i64, height: img.height as i64, rows }
    }

    /// Whether the window's pixel (x, y) (from its top left corner) shows.
    pub fn contains(&self, x: i64, y: i64) -> bool {
        y >= 0 && y < self.height && self.rows[y as usize].iter().any(|&(a, b)| x >= a && x < b)
    }

    /// The outline as rectangles (x, y, width, height), rows of equal runs
    /// merged — what a window system's region or a clip path is made of.
    pub fn rects(&self) -> Vec<(i64, i64, i64, i64)> {
        let mut out: Vec<(i64, i64, i64, i64)> = Vec::new();
        // (the rectangles still growing downwards: run → index in `out`)
        let mut open: HashMap<(i64, i64), usize> = HashMap::new();
        for (y, runs) in self.rows.iter().enumerate() {
            let y = y as i64;
            let mut next = HashMap::new();
            for &(a, b) in runs {
                match open.get(&(a, b)) {
                    Some(&i) if out[i].1 + out[i].3 == y => {
                        out[i].3 += 1;
                        next.insert((a, b), i);
                    }
                    _ => {
                        out.push((a, y, b - a, 1));
                        next.insert((a, b), out.len() - 1);
                    }
                }
            }
            open = next;
        }
        out
    }

    /// The outline as an SVG path (`M x y h w v h h -w z` per rectangle),
    /// moved by (`dx`, `dy`) — a CSS `clip-path: path(…)`.
    pub fn svg_path(&self, dx: i64, dy: i64) -> String {
        let mut s = String::new();
        for (x, y, w, h) in self.rects() {
            s.push_str(&format!("M{} {}h{}v{}h{}z", x + dx, y + dy, w, h, -w));
        }
        s
    }
}

thread_local! {
    static SHAPES: RefCell<HashMap<String, Rc<Shape>>> = RefCell::new(HashMap::new());
}

/// Form `form`'s outline from now on (`None`: its whole window again).
pub fn set(form: &str, shape: Option<Shape>) {
    let form = form.to_ascii_lowercase();
    SHAPES.with(|s| match shape {
        Some(shape) => {
            s.borrow_mut().insert(form, Rc::new(shape));
        }
        None => {
            s.borrow_mut().remove(&form);
        }
    });
}

/// Form `form`'s outline, if ShapeForm gave it one.
pub fn get(form: &str) -> Option<Rc<Shape>> {
    SHAPES.with(|s| s.borrow().get(&form.to_ascii_lowercase()).cloned())
}

/// `Form.ShapeForm(source, transparent)`: the outline of the bitmap
/// `source` names (a file, a `$RESOURCE` handle, a QBITMAP), kept for
/// form `form`. The error RapidQ's runtime raises when it can't be read.
pub fn shape_form(form: &str, source: &crate::Value, transparent: i64) -> Result<Rc<Shape>, String> {
    let bitmap = crate::objects::load_image(source).map_err(|_| format!("Cannot open file {}.", source.to_string_val()))?;
    let shape = Shape::of(&bitmap, transparent as u32);
    set(form, Some(shape));
    Ok(get(form).expect("just set"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::codec::Pixels;

    fn bitmap(w: usize, h: usize, f: impl Fn(usize, usize) -> u32) -> Bitmap {
        let pixels = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).map(|(x, y)| f(x, y)).collect();
        Bitmap { img: Pixels { width: w, height: h, pixels }, ..Bitmap::default() }
    }

    #[test]
    fn outline_of_the_pixels_kept() {
        // (RC.EXE's probe: a red ellipse on white — white transparent keeps
        // the ellipse; red transparent keeps the white around it)
        let b = bitmap(10, 6, |x, y| if (2..8).contains(&x) && (1..5).contains(&y) { 0x0000FF } else { 0xFFFFFF });
        let s = Shape::of(&b, 0xFFFFFF);
        assert!(s.contains(5, 3) && !s.contains(1, 1) && !s.contains(5, 0) && !s.contains(20, 3));
        assert_eq!(s.rects(), vec![(2, 1, 6, 4)]);
        assert_eq!(s.svg_path(1, 30), "M3 31h6v4h-6z");
        let r = Shape::of(&b, 0x0000FF);
        assert!(!r.contains(5, 3) && r.contains(1, 1) && r.contains(9, 5) && !r.contains(10, 5));
        assert_eq!(r.rects().iter().map(|r| r.2 * r.3).sum::<i64>(), 60 - 24);
    }

    #[test]
    fn kept_per_form() {
        let b = bitmap(2, 2, |x, _| if x == 0 { 1 } else { 0 });
        set("F", Some(Shape::of(&b, 0)));
        assert!(get("f").is_some_and(|s| s.contains(0, 1) && !s.contains(1, 1)));
        set("f", None);
        assert!(get("f").is_none());
    }
}
