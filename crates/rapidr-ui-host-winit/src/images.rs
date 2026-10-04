//! The host's pictures, kept between frames (docs/desktop-host-plan.md §5):
//! an `Op::Image` names its picture (`source`: a bitmap's object id, or a
//! component's own picture) and the bitmap's drawing revision; what the
//! renderers made of it (vello's `ImageData`, whose blob vello keeps on the
//! GPU while it stays the same; vello_cpu's pixmap) is reused while the
//! revision stays the same, so a canvas nobody draws on is never converted
//! or uploaded again. Revision 0 means "no revision" (a picture made while
//! painting): kept only while the kernel hands over the very same picture.
//! Pictures not drawn for a while are dropped.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use rapidr_ui_kernel::display::Picture;
use vello::kurbo::Rect as KRect;
use vello::peniko::{Blob, ImageAlphaType, ImageData, ImageFormat, ImageQuality};
use vello_cpu::Pixmap;

/// Frames (display lists drawn) a picture is kept without being drawn.
const KEEP_FRAMES: u64 = 600;

struct Entry {
    revision: u64,
    /// The picture it was made from (revision 0: kept only for it).
    picture: Arc<Picture>,
    gpu: Option<ImageData>,
    cpu: Option<Arc<Pixmap>>,
    used: u64,
}

#[derive(Default)]
struct Cache {
    entries: HashMap<String, Entry>,
    frame: u64,
    /// Conversions made (tests).
    made: u64,
}

thread_local! {
    static CACHE: RefCell<Cache> = RefCell::new(Cache::default());
}

/// The entry for `source` as `picture` at `revision`, made again when it
/// changed; `f` gets it.
fn with_entry<R>(source: &str, revision: u64, picture: &Arc<Picture>, f: impl FnOnce(&mut Entry, &mut u64) -> R) -> R {
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        let frame = c.frame;
        let Cache { entries, made, .. } = &mut *c;
        let same = |e: &Entry| {
            if revision == 0 {
                Arc::ptr_eq(&e.picture, picture)
            } else {
                e.revision == revision && e.picture.width == picture.width && e.picture.height == picture.height
            }
        };
        let e = entries.entry(source.to_string()).or_insert_with(|| Entry { revision, picture: picture.clone(), gpu: None, cpu: None, used: frame });
        if !same(e) {
            *e = Entry { revision, picture: picture.clone(), gpu: None, cpu: None, used: frame };
        }
        e.used = frame;
        f(e, made)
    })
}

/// A picture's pixels as vello's blob, without a copy.
struct Rgba(Arc<Picture>);

impl AsRef<[u8]> for Rgba {
    fn as_ref(&self) -> &[u8] {
        &self.0.rgba
    }
}

/// Whether a picture is sound (its pixels fill it).
fn sound(p: &Picture) -> bool {
    p.width > 0 && p.height > 0 && p.rgba.len() == p.width * p.height * 4
}

/// vello's image of a picture (the GPU renderer).
pub fn gpu_image(source: &str, revision: u64, picture: &Arc<Picture>) -> Option<ImageData> {
    if !sound(picture) {
        return None;
    }
    with_entry(source, revision, picture, |e, made| {
        if e.gpu.is_none() {
            *made += 1;
            e.gpu = Some(ImageData {
                data: Blob::new(Arc::new(Rgba(picture.clone()))),
                format: ImageFormat::Rgba8,
                alpha_type: ImageAlphaType::Alpha,
                width: picture.width as u32,
                height: picture.height as u32,
            });
        }
        e.gpu.clone()
    })
}

/// vello_cpu's pixmap of a picture (the CPU renderer, captures).
pub fn cpu_pixmap(source: &str, revision: u64, picture: &Arc<Picture>) -> Option<Arc<Pixmap>> {
    let (w, h) = (picture.width, picture.height);
    if !sound(picture) || w > usize::from(u16::MAX) || h > usize::from(u16::MAX) {
        return None;
    }
    with_entry(source, revision, picture, |e, made| {
        if e.cpu.is_none() {
            *made += 1;
            let meta = vello_cpu::PixelMetadata { may_have_transparency: true, alpha_type: vello_cpu::peniko::ImageAlphaType::Alpha };
            e.cpu = Some(Arc::new(Pixmap::from_parts(picture.rgba.clone(), w as u16, h as u16, meta)));
        }
        e.cpu.clone()
    })
}

/// A display list was drawn: pictures not drawn for a while go.
pub fn end_frame() {
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        c.frame += 1;
        let frame = c.frame;
        c.entries.retain(|_, e| e.used + KEEP_FRAMES >= frame);
    });
}

/// How many conversions the cache has made (tests).
pub fn conversions() -> u64 {
    CACHE.with(|c| c.borrow().made)
}

/// How a picture `w` × `h` is sampled into `rect` (device pixels): pixel
/// for pixel at its own size (crisp), smoothly when enlarged (bicubic),
/// averaged when made smaller.
pub fn quality(rect: &KRect, w: usize, h: usize) -> ImageQuality {
    let (sx, sy) = (rect.width() / w as f64, rect.height() / h as f64);
    if (rect.width() - w as f64).abs() < 0.5 && (rect.height() - h as f64).abs() < 0.5 {
        ImageQuality::Low
    } else if sx >= 1.0 && sy >= 1.0 {
        ImageQuality::High
    } else {
        ImageQuality::Medium
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rapidr_ui_kernel::display::{DisplayList, Item, Picture};
    use rapidr_ui_kernel::{FormUi, MemStore, TextSystem};
    use rapidr_value::objects::ops::Op;

    use super::conversions;

    fn list(source: &str, revision: u64, picture: &Arc<Picture>) -> DisplayList {
        let mut l = DisplayList { size: (20, 10), scale: 2.0, ..Default::default() };
        l.items.push(Item::Op { origin: (0, 0), op: Op::Image { source: source.into(), revision, rect: (0, 0, 10, 5) } });
        l.images.insert(source.into(), picture.clone());
        l
    }

    /// A picture is converted once per revision; one without a revision
    /// once per picture handed over.
    #[test]
    fn pictures_are_kept_by_revision() {
        let mut s = MemStore::new();
        s.add("imgform", "RFORM", None);
        let form = FormUi::build(&s, "imgform", false);
        let mut ts = TextSystem::new();
        let red = |w: usize, h: usize| Arc::new(Picture { width: w, height: h, rgba: [0xFF, 0, 0, 0xFF].repeat(w * h) });
        let pic = red(20, 10);
        let before = conversions();
        let px = crate::cpu::capture(&list("cv", 7, &pic), &mut ts, &form);
        assert_eq!(px.pixels[0], 0x0000FF);
        crate::cpu::capture(&list("cv", 7, &red(20, 10)), &mut ts, &form);
        assert_eq!(conversions(), before + 1);
        crate::cpu::capture(&list("cv", 8, &red(20, 10)), &mut ts, &form);
        assert_eq!(conversions(), before + 2);
        // (no revision: the same picture kept, a new one converted)
        crate::cpu::capture(&list("p#0", 0, &pic), &mut ts, &form);
        crate::cpu::capture(&list("p#0", 0, &pic), &mut ts, &form);
        assert_eq!(conversions(), before + 3);
        crate::cpu::capture(&list("p#0", 0, &red(20, 10)), &mut ts, &form);
        assert_eq!(conversions(), before + 4);
    }
}
