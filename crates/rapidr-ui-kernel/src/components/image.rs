//! QIMAGE: its picture (a bitmap in the shared model — BMPHandle from a
//! `$RESOURCE`, a BMP / SVG / icon file, or what the program drew on it),
//! as FLTK's `picture_refresh` and the web show it: at the top left, in
//! the middle (Center) or scaled to the control (Stretch); with
//! Transparent (or an image with soft edges) what's behind shows through
//! — like Delphi's TImage, a graphic control, it paints no background of
//! its own. The picture is what the bitmap shows on screen (its high-DPI
//! layer: an SVG drawn again at the screen's scale), converted only when
//! its drawing revision changes (`canvas::Shown`).
//!
//! The mouse: OnMouseDown / OnMouseMove / OnMouseUp (the kernel's generic
//! routing, X and Y in the image) and OnClick for a press and release on
//! it. It never takes the focus.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use rapidr_value::objects::a11y::{mnemonic, node_id, AccessNode, Role};
use rapidr_value::objects::ops::{Place, Rect};
use rapidr_value::objects::text::text_size;
use rapidr_value::Value;

use super::canvas::{draw_shown, shown};
use super::{ComponentKind, Cx, MouseIn, MouseOut};
use crate::display::Picture;
use crate::paint::Painter;
use crate::store::{self, Store};

pub struct Image;

impl ComponentKind for Image {
    fn name(&self) -> &'static str {
        "RIMAGE"
    }

    fn focusable(&self, _store: &dyn Store, _id: &str) -> bool {
        false
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        let cache = &mut cx.ui.surface;
        let Some(Some(s)) = rapidr_value::objects::with_picture(cx.id, |b| if b.img.pixels.is_empty() { None } else { shown(cache, b) }) else {
            return;
        };
        let (sw, sh) = s.size;
        let rect = if store::flag(cx.store, cx.id, "stretch", false) {
            (0, 0, w, h)
        } else if store::flag(cx.store, cx.id, "center", false) {
            ((w - sw) / 2, (h - sh) / 2, sw, sh)
        } else {
            (0, 0, sw, sh)
        };
        draw_shown(p, cx.id, &s, rect);
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        // (the input lane's: OnClick, OnDblClick in the VCL's order)
        super::canvas::click_or_double(cx, m)
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = AccessNode::new(node_id(cx.id), Role::Image);
        n.name = store::string(cx.store, cx.id, "hint");
        n.bounds = cx.rect;
        n
    }
}

// ------------------------------------------------ speed buttons' glyphs --

/// Which of a QCOOLBTN's / QOVALBTN's glyphs shows (manual: "the first
/// bitmap appears when the button is up, the second when disabled, the
/// third when clicked, the fourth when it stays down"), for a strip of
/// `count` (NumBMPs): down without a fourth shows the clicked one (as
/// Delphi's TSpeedButton), a state without its own image the up one.
pub fn glyph_frame(count: i64, enabled: bool, pressed: bool, down: bool) -> i64 {
    let frame = if !enabled {
        1
    } else if pressed {
        2
    } else if down {
        if count >= 4 {
            3
        } else {
            2
        }
    } else {
        0
    };
    if frame < count {
        frame
    } else {
        0
    }
}

/// A frame of a glyph strip: its picture (what the screen shows) and its
/// size in logical pixels.
type Frame = (Arc<Picture>, (i64, i64));

/// A glyph strip as loaded (what the screen shows of it) and its frames
/// cut out as they're needed.
struct Strip {
    size: (i64, i64),
    shown: (usize, usize, Vec<u8>, usize),
    frames: Vec<Option<Frame>>,
}

thread_local! {
    /// By BMP / BMPHandle value and the screen's scale.
    static STRIPS: RefCell<HashMap<(String, usize), Option<Strip>>> = RefCell::new(HashMap::new());
}

/// Frame `frame` of `count` of the glyph strip `value` (BMP's file or
/// `data:` URL, a QBITMAP, BMPHandle's `$RESOURCE`). Its bottom-left
/// pixel's colour is transparent (as TSpeedButton's glyphs), unless it has
/// its own transparency.
fn glyph(value: &Value, count: i64, frame: i64) -> Option<Frame> {
    let scale = rapidr_value::objects::bitmap::display_scale();
    let key = (format!("{value:?}"), scale);
    STRIPS.with(|s| {
        let mut s = s.borrow_mut();
        let strip = s.entry(key).or_insert_with(|| {
            let mut b = rapidr_value::objects::load_image(value).ok()?;
            if b.img.pixels.is_empty() {
                return None;
            }
            if b.alpha_channel().is_none() && !b.transparent {
                b.transparent = true;
                b.transparent_color = b.pixel(0, b.img.height as i64 - 1)?;
                b.touch();
            }
            let size = (b.img.width as i64, b.img.height as i64);
            Some(Strip { size, shown: b.display_rgba(), frames: Vec::new() })
        });
        let strip = strip.as_mut()?;
        let count = count.clamp(1, strip.size.0.max(1));
        let frame = frame.clamp(0, count - 1) as usize;
        if strip.frames.len() < count as usize {
            strip.frames.resize(count as usize, None);
        }
        if strip.frames[frame].is_none() {
            let (pw, ph, rgba, sc) = &strip.shown;
            let fw = strip.size.0 / count;
            let (x0, dw) = (frame * fw as usize * sc, fw as usize * sc);
            if fw <= 0 || x0 + dw > *pw {
                return None;
            }
            let mut out = Vec::with_capacity(dw * ph * 4);
            for y in 0..*ph {
                let row = (y * pw + x0) * 4;
                out.extend_from_slice(&rgba[row..row + dw * 4]);
            }
            strip.frames[frame] = Some((Arc::new(Picture { width: dw, height: *ph, rgba: out }), (fw, strip.size.1)));
        }
        strip.frames[frame].clone()
    })
}

/// A speed button's glyph (BMP / BMPHandle, NumBMPs) drawn in `rect`, and
/// where its caption goes beside it (Layout: blBMPLeft 0, blBMPRight 1,
/// blBMPTop 2, blBMPBottom 3; Spacing between them, 4 by default), the two
/// centred together as TSpeedButton lays them out. `None` without a glyph
/// (the caption alone, centred).
pub fn paint_glyph(cx: &Cx, p: &mut Painter, rect: Rect, enabled: bool, pressed: bool, down: bool) -> Option<(Rect, Place)> {
    let mut value = cx.store.get(cx.id, "bmp");
    if value.to_string_val().is_empty() {
        value = cx.store.get(cx.id, "bmphandle");
    }
    let unset = match &value {
        Value::Null => true,
        Value::Integer(_) | Value::Double(_) => value.to_i64() == 0,
        v => v.to_string_val().is_empty(),
    };
    if unset {
        return None;
    }
    let count = store::int(cx.store, cx.id, "numbmps", 1).max(1);
    let (pic, (gw, gh)) = glyph(&value, count, glyph_frame(count, enabled, pressed, down))?;
    let (rx, ry, w, h) = rect;
    let text = mnemonic(&store::string(cx.store, cx.id, "caption")).0;
    let (tw, th) = if text.is_empty() { (0, 0) } else { text_size(&text, &cx.font) };
    let sp = if text.is_empty() { 0 } else { store::int(cx.store, cx.id, "spacing", 4) };
    let layout = store::int(cx.store, cx.id, "layout", 0);
    let (at, caption) = if layout >= 2 {
        let total = gh + sp + th;
        let y0 = ry + (h - total + 1) / 2;
        let gx = rx + (w - gw + 1) / 2;
        if layout == 2 {
            ((gx, y0), ((rx, y0 + gh + sp, w, th), Place::TopCenter))
        } else {
            ((gx, y0 + th + sp), ((rx, y0, w, th), Place::TopCenter))
        }
    } else {
        let total = gw + sp + tw;
        let x0 = rx + (w - total + 1) / 2;
        let gy = ry + (h - gh + 1) / 2;
        if layout == 1 {
            ((x0 + tw + sp, gy), ((x0, ry, tw, h), Place::Left))
        } else {
            ((x0, gy), ((x0 + gw + sp, ry, tw, h), Place::Left))
        }
    };
    p.picture(&format!("{}#glyph", cx.id), 0, pic, (at.0, at.1, gw, gh));
    Some(caption)
}
