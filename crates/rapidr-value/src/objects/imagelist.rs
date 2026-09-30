//! QIMAGELIST (manual, Appendix A): a list of same-sized images. A bitmap
//! wider than the list is split into several images, as RapidQ does for
//! toolbar strips; `MaskColor` becomes each image's transparent color.

use super::bitmap::Bitmap;
use crate::{v_int, Value};

#[derive(Debug, Clone)]
pub struct ImageList {
    pub width: i64,
    pub height: i64,
    pub masked: bool,
    pub images: Vec<Bitmap>,
}

impl Default for ImageList {
    fn default() -> Self {
        Self { width: 16, height: 16, masked: true, images: Vec::new() }
    }
}

impl ImageList {
    pub fn get(&self, prop: &str) -> Option<Value> {
        Some(match prop {
            "count" => v_int(self.images.len() as i64),
            "width" => v_int(self.width),
            "height" => v_int(self.height),
            "masked" => v_int(if self.masked { -1 } else { 0 }),
            _ => return None,
        })
    }

    pub fn set(&mut self, prop: &str, val: &Value) -> bool {
        match prop {
            "width" => self.width = val.to_i64().clamp(1, 4096),
            "height" => self.height = val.to_i64().clamp(1, 4096),
            "masked" => self.masked = val.to_bool(),
            _ => return false,
        }
        true
    }

    /// Adds `src` at `index` (split into list-sized images when it's a
    /// strip), with `mask` as the transparent color. Returns how many
    /// images were added.
    pub fn insert(&mut self, index: usize, src: &Bitmap, mask: Option<u32>) -> usize {
        let (w, h) = (self.width as usize, self.height as usize);
        let count = if src.img.width > w && src.img.width.is_multiple_of(w) { src.img.width / w } else { 1 };
        let index = index.min(self.images.len());
        // An SVG of the list's size is kept whole (drawn again at the
        // screen's scale).
        if count == 1 && src.is_svg() && (src.img.width, src.img.height) == (w, h) {
            let mut image = src.clone();
            if let (Some(mask), true) = (mask, self.masked) {
                image.transparent = true;
                image.transparent_color = mask;
            }
            self.images.insert(index, image);
            return 1;
        }
        for i in 0..count {
            let mut image = Bitmap::default();
            image.resize(w as i64, h as i64);
            let x = (i * w) as i64;
            image.copy_rect((0, 0, w as i64, h as i64), src, (x, 0, x + w as i64, h as i64));
            if let (Some(mask), true) = (mask, self.masked) {
                image.transparent = true;
                image.transparent_color = mask;
            }
            self.images.insert(index + i, image);
        }
        count
    }

    pub fn call(&mut self, method: &str, args: &[Value]) -> Option<Value> {
        match method {
            "clear" => self.images.clear(),
            "delete" => {
                let i = args.first().map_or(-1, Value::to_i64);
                if i >= 0 && (i as usize) < self.images.len() {
                    self.images.remove(i as usize);
                }
            }
            _ => return None,
        }
        Some(Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_split_and_masks() {
        let mut strip = Bitmap::default();
        strip.resize(32, 16);
        strip.fill_rect(16, 0, 32, 16, 0xFF);
        let mut list = ImageList::default();
        assert_eq!(list.insert(0, &strip, Some(0xFF)), 2);
        assert_eq!(list.get("count").unwrap().to_i64(), 2);
        assert_eq!(list.images[1].pixel(0, 0), Some(0xFF));
        assert!(list.images[1].transparent);
        list.call("delete", &[v_int(0)]);
        assert_eq!(list.images.len(), 1);
    }
}
