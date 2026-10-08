//! QDXSCREEN (the DirectX lane's, docs/directx-plan.md): what its last
//! Flip showed. The program draws on the screen's back buffer in the
//! shared model (`rapidr_value::objects::directx`) and `Flip` copies it to
//! the front; the kernel emits the front as an `Op::Image` named by the
//! screen's id with its drawing revision (`canvas::shown` converts it again
//! only when a Flip changed it), so a frame that flipped nothing copies
//! nothing. With AllowStretch the picture fills the control, as DelphiX
//! stretched its surface onto the window; without (RapidQ's default: RC.EXE
//! reads 0 — AutoSize keeps the surface the control's size anyway), it shows
//! at its own size at the top left over black.
//!
//! It never takes the focus (a TCustomControl: keys go to the form, as
//! RapidQ's DirectX examples expect of Form.OnKeyDown); OnClick and
//! OnDblClick in the VCL's order, OnMouseDown / Move / Up by the kernel's
//! routing.

use rapidr_value::objects::a11y::AccessNode;

use super::canvas::{click_or_double, draw_shown, shown};
use super::{ComponentKind, Cx, MouseIn, MouseOut};
use crate::paint::Painter;
use crate::store::Store;

pub struct DxScreen;

/// Whether `id` is a QDXSCREEN with FullScreen on: in DirectDraw's
/// exclusive mode the surface had the whole display, the form's other
/// windows hidden under it — here it covers its form's whole client area,
/// over the form's other components ([`stacked`], `FormUi::sync`), and its
/// picture is scaled into it with its proportions kept.
pub fn full_screen(store: &dyn Store, id: &str, type_name: &str) -> bool {
    type_name.eq_ignore_ascii_case("RDXSCREEN") && crate::store::flag(store, id, "fullscreen", false)
}

/// A form's components with its full-screen QDXSCREENs last (drawn on top).
pub fn stacked(store: &dyn Store, children: Vec<(String, String)>) -> Vec<(String, String)> {
    let (full, mut out): (Vec<_>, Vec<_>) = children.into_iter().partition(|(id, t)| full_screen(store, id, t));
    out.extend(full);
    out
}

impl ComponentKind for DxScreen {
    fn name(&self) -> &'static str {
        "RDXSCREEN"
    }

    fn focusable(&self, _store: &dyn Store, _id: &str) -> bool {
        false
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        if w <= 0 || h <= 0 {
            return;
        }
        p.fill((0, 0, w, h), crate::text::bgr_to_rgb(0));
        let cache = &mut cx.ui.surface;
        let c = rapidr_value::objects::dxscreen_control(cx.id, &|i, p| cx.store.get(i, p));
        let Some(Some(s)) = rapidr_value::objects::with_dxscreen(cx.id, |d| {
            d.follow_control(w, h, c.follows());
            shown(cache, &mut d.front)
        }) else {
            return;
        };
        let rect = rapidr_value::objects::directx::picture_rect(s.size, (w, h), c.stretch, c.fullscreen);
        draw_shown(p, cx.id, &s, rect);
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        click_or_double(cx, m)
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        super::shared_describe(cx, self.name())
    }
}

#[cfg(test)]
mod tests {
    use rapidr_value::objects::bitmap::set_display_scale;
    use rapidr_value::{v_int, Value};

    use crate::{FormUi, Item, MemStore, Op, TextSystem};

    fn call(s: &MemStore, id: &str, method: &str, args: &[Value]) {
        use crate::Store;
        let reader = |i: &str, p: &str| s.get(i, p);
        rapidr_value::objects::call(id, method, args, &reader);
    }

    /// The `Op::Image`s of a display list: source, revision, rect.
    fn images(list: &crate::DisplayList) -> Vec<(String, u64, (i64, i64, i64, i64))> {
        list.items
            .iter()
            .filter_map(|i| match i {
                Item::Op { op: Op::Image { source, revision, rect }, .. } => Some((source.clone(), *revision, *rect)),
                _ => None,
            })
            .collect()
    }

    /// Nothing drawn shows until a Flip; a frame without one keeps the
    /// same picture (revision); AllowStretch fills the control, without it
    /// (RapidQ's default) the surface shows at its own size.
    #[test]
    fn shows_the_last_flip() {
        set_display_scale(1.0);
        let mut s = MemStore::new();
        s.add("dxform", "RFORM", None).set("dxform", "width", v_int(200)).set("dxform", "height", v_int(150));
        s.add("dxs", "RDXSCREEN", Some("dxform"));
        for (p, v) in [("left", 0), ("top", 0), ("width", 160), ("height", 120), ("allowstretch", 1)] {
            s.set("dxs", p, v_int(v));
        }
        call(&s, "dxs", "init", &[v_int(80), v_int(60)]);
        assert!(rapidr_value::objects::dxscreen_initialize("dxs", &|i, p| crate::Store::get(&s, i, p)));
        let mut text = TextSystem::new();
        let mut f = FormUi::build(&s, "dxform", false);
        let first = images(&f.paint(&s, &mut text, 1.0));
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].2, (0, 0, 160, 120), "stretched over the control");
        call(&s, "dxs", "fillrect", &[v_int(0), v_int(0), v_int(10), v_int(10), v_int(0xFF)]);
        assert_eq!(images(&f.paint(&s, &mut text, 1.0))[0].1, first[0].1, "not flipped: the same picture");
        call(&s, "dxs", "flip", &[]);
        let flipped = images(&f.paint(&s, &mut text, 1.0));
        assert_ne!(flipped[0].1, first[0].1, "a Flip shows the back buffer");
        let pixel = rapidr_value::objects::with_dxscreen("dxs", |d| d.front.pixel(5, 5)).flatten();
        assert_eq!(pixel, Some(0xFF));
        s.set("dxs", "allowstretch", v_int(0));
        assert_eq!(images(&f.paint(&s, &mut text, 1.0))[0].2, (0, 0, 80, 60), "its own size");
        // FullScreen: scaled to fit, its proportions kept (80 × 60 in
        // 160 × 120 — a 4:3 control: all of it).
        s.set("dxs", "fullscreen", v_int(-1)).set("dxs", "width", v_int(200));
        assert_eq!(images(&f.paint(&s, &mut text, 1.0))[0].2, (20, 0, 160, 120), "centred, black on both sides");
        rapidr_value::objects::remove("dxs");
    }
}
