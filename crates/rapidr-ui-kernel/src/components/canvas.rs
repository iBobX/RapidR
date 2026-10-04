//! Drawing surfaces: QCANVAS and a form's own surface (`Form.TextOut`,
//! `Form.Line` …). Both are CPU bitmaps in the shared model
//! (`rapidr_value::objects::bitmap`): the program draws there and reads
//! back the low-resolution pixels (`Pixel`, `.BMP`), while every drawing
//! call also draws the high-DPI layer at the screen's scale — what the
//! kernel shows, so lines and text are sharp at 2×. The kernel emits an
//! `Op::Image` named after the bitmap's id with its drawing revision
//! (`Bitmap::revision`); the pixels are converted again only when the
//! revision changes ([`Shown`]), and the host keeps what it uploaded by the
//! same key.
//!
//! OnPaint is runtime-core's (the first one after the window shows,
//! Repaint, a resize): what a handler drew stays in the bitmap, so painting
//! a frame never calls the program.

use std::sync::Arc;

use rapidr_value::objects::a11y::AccessNode;
use rapidr_value::objects::bitmap::{display_scale, Bitmap};
use rapidr_value::objects::ops::Rect;

use super::{ComponentKind, Cx, MouseIn, MouseKind, MouseOut};
use crate::display::Picture;
use crate::paint::Painter;
use crate::store::Store;
use crate::text::bgr_to_rgb;
use crate::tree::FormUi;

/// A bitmap's screen pixels as last converted: kept while its revision
/// (and the screen's scale) stay the same.
#[derive(Clone, Debug)]
pub struct Shown {
    revision: u64,
    scale: usize,
    pub picture: Arc<Picture>,
    /// Its size in logical pixels (the bitmap's own).
    pub size: (i64, i64),
}

impl Shown {
    pub fn revision(&self) -> u64 {
        self.revision
    }
}

/// What `b` shows now (`None`: nothing, an empty bitmap), from `cache`
/// when its revision didn't change.
pub fn shown(cache: &mut Option<Shown>, b: &mut Bitmap) -> Option<Shown> {
    let revision = b.display_revision();
    let scale = display_scale();
    if let Some(c) = cache.as_ref().filter(|c| c.revision == revision && c.scale == scale) {
        return Some(c.clone());
    }
    let (w, h) = (b.img.width as i64, b.img.height as i64);
    let (pw, ph, rgba, _) = b.display_rgba();
    *cache = (pw > 0 && ph > 0).then(|| Shown { revision, scale, picture: Arc::new(Picture { width: pw, height: ph, rgba }), size: (w, h) });
    cache.clone()
}

/// Draws what a bitmap shows into `rect` (logical pixels), named by its
/// object id.
pub fn draw_shown(p: &mut Painter, id: &str, s: &Shown, rect: Rect) {
    p.picture(id, s.revision, s.picture.clone(), rect);
}

/// A QCANVAS's surface at the control's size (`with_canvas` gives it that
/// size, as FLTK's and the web's drawing do), as shown.
fn canvas_shown(cache: &mut Option<Shown>, id: &str, w: i64, h: i64) -> Option<(Shown, u32)> {
    rapidr_value::objects::with_canvas(id, w, h, |b| shown(cache, b).map(|s| (s, b.background))).flatten()
}

/// A form's own drawing surface under its components (made the first time
/// the program draws on the form), its client area's size: where it
/// shows the form's Color the form shows through.
pub fn paint_form_surface(f: &mut FormUi, p: &mut Painter) {
    if !rapidr_value::objects::is_form_surface(&f.form) {
        return;
    }
    let (w, h) = f.client;
    if w <= 0 || h <= 0 {
        return;
    }
    let form = f.form.clone();
    let shown = rapidr_value::objects::with_canvas(&form, w, h, |b| shown(&mut f.surface, b)).flatten();
    if let Some(s) = shown {
        let (sw, sh) = s.size;
        p.clipped((0, f.menu_offset, w, h), |p| draw_shown(p, &form, &s, (0, f.menu_offset, sw, sh)));
    }
}

/// QCANVAS (RapidQ's TPaintBox-like control): its surface, filled with its
/// Color where nothing is drawn. It never takes the focus; a click is a
/// press and a release on it (OnClick before OnMouseUp, as the kernel's
/// other controls).
pub struct Canvas;

impl ComponentKind for Canvas {
    fn name(&self) -> &'static str {
        "RCANVAS"
    }

    fn focusable(&self, _store: &dyn Store, _id: &str) -> bool {
        false
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        if w <= 0 || h <= 0 {
            return;
        }
        match canvas_shown(&mut cx.ui.surface, cx.id, w, h) {
            Some((s, background)) => {
                p.fill((0, 0, w, h), bgr_to_rgb(i64::from(background)));
                let (sw, sh) = s.size;
                draw_shown(p, cx.id, &s, (0, 0, sw, sh));
            }
            None => {
                // (not a canvas the model knows: its Color)
                let color = rapidr_value::objects::form_color(&cx.store.get(cx.id, "color"));
                p.fill((0, 0, w, h), bgr_to_rgb(color));
            }
        }
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        click_on_release(cx, m)
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        super::shared_describe(cx, self.name())
    }
}

/// A graphic control's click: the left button pressed on it and released
/// over it. Every click is one — RapidQ's QCANVAS has no OnDblClick ("does
/// not work", its manual says), so a double click is two clicks.
pub fn click_on_release(cx: &mut Cx, m: &MouseIn) -> MouseOut {
    if m.kind == MouseKind::Up && m.button == rapidr_value::input::Button::Left && m.inside && m.captured {
        cx.click();
    }
    MouseOut { press: false, focus: Some(false) }
}

/// (the input lane's) A VCL control's clicks (csClickEvents and
/// csDoubleClicks: QPANEL, QLABEL, QGROUPBOX, QSCROLLBOX, QIMAGE): a double
/// click's second press is OnDblClick, before its OnMouseDown (Windows'
/// WM_LBUTTONDBLCLK), and its release no click; a single click let go over
/// it is OnClick, before OnMouseUp.
pub fn click_or_double(cx: &mut Cx, m: &MouseIn) -> MouseOut {
    let left = m.button == rapidr_value::input::Button::Left;
    match m.kind {
        MouseKind::Down if left && m.double() => cx.events.push(crate::input::KernelEvent::DblClick(cx.id.to_string())),
        MouseKind::Up if left && m.inside && m.captured && !m.double() => cx.click(),
        _ => {}
    }
    MouseOut { press: false, focus: Some(false) }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rapidr_value::input::Button;
    use rapidr_value::objects::bitmap::set_display_scale;
    use rapidr_value::{v_int, v_str, Value};

    use crate::{FormUi, Item, KernelEvent, MemStore, Mods, Op, TextSystem};

    fn place(s: &mut MemStore, id: &str, (l, t, w, h): (i64, i64, i64, i64)) {
        s.set(id, "left", v_int(l)).set(id, "top", v_int(t)).set(id, "width", v_int(w)).set(id, "height", v_int(h));
    }

    /// A drawing method, the canvas's size read from the store (as the
    /// runtimes call it).
    fn draw(s: &MemStore, id: &str, method: &str, args: &[Value]) {
        use crate::Store;
        let reader = |i: &str, p: &str| s.get(i, p);
        rapidr_value::objects::call(id, method, args, &reader);
    }

    type ImageOp = (String, u64, (i64, i64), (i64, i64, i64, i64));

    /// The `Op::Image`s of a display list: source, revision, origin, rect.
    fn images(list: &crate::DisplayList) -> Vec<ImageOp> {
        list.items
            .iter()
            .filter_map(|i| match i {
                Item::Op { origin, op: Op::Image { source, revision, rect } } => Some((source.clone(), *revision, *origin, *rect)),
                _ => None,
            })
            .collect()
    }

    /// A canvas shows its surface at the screen's scale (the high-DPI
    /// layer: twice the pixels at 2×), converted again only when something
    /// is drawn; a click on it is a press and a release, and it never takes
    /// the focus.
    #[test]
    fn canvas_surface_by_revision() {
        set_display_scale(2.0);
        let mut s = MemStore::new();
        s.add("cform", "RFORM", None);
        s.add("cv", "RCANVAS", Some("cform"));
        place(&mut s, "cv", (10, 20, 50, 30));
        s.set("cv", "color", v_int(0x00FF00));
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(&s, "cform", false);
        drop(f.paint(&s, &mut ts, 2.0));
        draw(&s, "cv", "fillrect", &[v_int(0), v_int(0), v_int(10), v_int(10), v_int(0xFF)]);
        let a = f.paint(&s, &mut ts, 2.0);
        let ims = images(&a);
        assert_eq!(ims.len(), 1, "{}", a.dump());
        let (source, rev, origin, rect) = ims[0].clone();
        assert_eq!((source.as_str(), origin, rect), ("cv", (10, 20), (0, 0, 50, 30)));
        let pic = a.images["cv"].clone();
        assert_eq!((pic.width, pic.height), (100, 60));
        // (red where drawn, at device resolution; RGBA)
        assert_eq!(&pic.rgba[..4], &[0xFF, 0, 0, 0xFF]);
        let at = (25 * 100 + 50) * 4;
        assert_eq!(&pic.rgba[at..at + 4], &[0, 0xFF, 0, 0xFF]);
        // Nothing drawn: the same picture, the same revision.
        let b = f.paint(&s, &mut ts, 2.0);
        assert_eq!(images(&b)[0].1, rev);
        assert!(Arc::ptr_eq(&b.images["cv"], &pic));
        // Drawn on: a new revision.
        draw(&s, "cv", "line", &[v_int(0), v_int(29), v_int(49), v_int(29), v_int(0)]);
        let c = f.paint(&s, &mut ts, 2.0);
        assert_ne!(images(&c)[0].1, rev);
        // A click: OnClick on release, no focus.
        f.mouse_down(&s, &mut ts, 15.0, 25.0, Button::Left, Mods::NONE);
        f.mouse_up(&s, &mut ts, 15.0, 25.0, Button::Left, Mods::NONE);
        assert!(f.take_events().contains(&KernelEvent::Click("cv".into())));
        assert_eq!(f.focus, None);
        set_display_scale(1.0);
    }

    /// A form drawn on gets its surface under its components, the size of
    /// its client area; the form's colour shows through where nothing is
    /// drawn.
    #[test]
    fn form_surface_under_the_components() {
        set_display_scale(1.0);
        let mut s = MemStore::new();
        s.add("sform", "RFORM", None);
        s.set("sform", "width", v_int(200)).set("sform", "height", v_int(150));
        s.add("sbtn", "RBUTTON", Some("sform"));
        s.set("sbtn", "caption", v_str("OK"));
        rapidr_value::objects::remove("sform");
        rapidr_value::objects::create_form_surface("sform", 0xE0E0E0);
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(&s, "sform", false);
        let (cw, ch) = f.client;
        let reader = |_: &str, p: &str| match p {
            "clientwidth" => v_int(cw),
            "clientheight" => v_int(ch),
            _ => Value::Null,
        };
        rapidr_value::objects::call("sform", "fillrect", &[v_int(10), v_int(10), v_int(60), v_int(40), v_int(0xFF)], &reader);
        let list = f.paint(&s, &mut ts, 1.0);
        let ims = images(&list);
        assert_eq!(ims.len(), 1, "{}", list.dump());
        assert_eq!(ims[0].0, "sform");
        assert_eq!(ims[0].3, (0, 0, cw, ch));
        // (before the button's ops)
        let at = |pred: &dyn Fn(&Item) -> bool| list.items.iter().position(pred).unwrap();
        assert!(at(&|i| matches!(i, Item::Op { op: Op::Image { .. }, .. })) < at(&|i| matches!(i, Item::Op { op: Op::Text { .. }, .. })));
        let pic = &list.images["sform"];
        let px = |x: usize, y: usize| pic.rgba[(y * pic.width + x) * 4..(y * pic.width + x) * 4 + 4].to_vec();
        assert_eq!(px(20, 20), vec![0xFF, 0, 0, 0xFF]);
        // (the form's colour: transparent)
        assert_eq!(px(100, 100)[3], 0);
    }

    /// A QIMAGE's picture: at the top left, centred, or stretched; an image
    /// without one draws nothing.
    #[test]
    fn image_placement() {
        set_display_scale(1.0);
        let mut s = MemStore::new();
        s.add("iform", "RFORM", None);
        s.add("img", "RIMAGE", Some("iform"));
        place(&mut s, "img", (0, 0, 40, 30));
        s.add("empty", "RIMAGE", Some("iform"));
        place(&mut s, "empty", (50, 0, 40, 30));
        rapidr_value::objects::with_picture("img", |b| b.resize(20, 10));
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(&s, "iform", false);
        let mut rect = |s: &MemStore| {
            let l = f.paint(s, &mut ts, 1.0);
            let ims = images(&l);
            assert_eq!(ims.len(), 1, "{}", l.dump());
            ims[0].3
        };
        assert_eq!(rect(&s), (0, 0, 20, 10));
        s.set("img", "center", v_int(-1));
        assert_eq!(rect(&s), (10, 10, 20, 10));
        s.set("img", "stretch", v_int(-1));
        assert_eq!(rect(&s), (0, 0, 40, 30));
    }

    /// A speed button's glyph strip (NumBMPs): the frame for its state,
    /// left of the caption, the two centred together.
    #[test]
    fn cool_button_glyph() {
        use super::super::image::glyph_frame;
        assert_eq!(glyph_frame(1, false, false, true), 0);
        assert_eq!(glyph_frame(2, false, false, false), 1);
        assert_eq!(glyph_frame(3, true, false, true), 2);
        assert_eq!(glyph_frame(4, true, false, true), 3);
        assert_eq!(glyph_frame(4, true, true, true), 2);
        set_display_scale(1.0);
        // A 32 × 16 strip of two 16 × 16 glyphs (a QBITMAP by id).
        let mut s = MemStore::new();
        s.add("gform", "RFORM", None);
        s.add("strip", "RBITMAP", None);
        s.set("strip", "width", v_int(32)).set("strip", "height", v_int(16));
        s.add("cb", "RCOOLBTN", Some("gform"));
        place(&mut s, "cb", (0, 0, 80, 30));
        s.set("cb", "bmp", v_str("strip")).set("cb", "numbmps", v_int(2));
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(&s, "gform", false);
        let l = f.paint(&s, &mut ts, 1.0);
        let ims = images(&l);
        assert_eq!(ims.len(), 1, "{}", l.dump());
        assert_eq!((ims[0].0.as_str(), ims[0].3 .2, ims[0].3 .3), ("cb#glyph", 16, 16));
        // (no caption: centred)
        assert_eq!((ims[0].3 .0, ims[0].3 .1), (32, 7));
        s.set("cb", "caption", v_str("Go"));
        let l = f.paint(&s, &mut ts, 1.0);
        let gx = images(&l)[0].3 .0;
        let text_x = l.items.iter().find_map(|i| match i {
            Item::Op { origin, op: Op::Text { rect, .. } } => Some(origin.0 + rect.0),
            _ => None,
        });
        assert!(gx < 32 && text_x == Some(gx + 16 + 4), "{}", l.dump());
    }
}
