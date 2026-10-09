//! QMEMO (RapidR's) and QRICHEDIT step 1 (`richedit.rs`): a multi-line
//! edit over the shared `TextEdit { multi }` — what the program reads as
//! Text (lines joined by CR LF), Line(i), LineCount, WhereX, WhereY,
//! SelStart / SelLength (a line break counts one) — drawn as Windows'
//! multi-line edit control: the sunken edge, the text from its top left,
//! one parley layout per paragraph and only the paragraphs in view drawn
//! (`text::editor`).
//!
//! - **WordWrap** (True by default) breaks lines at the view's width; off,
//!   a paragraph is one line and the view scrolls across.
//! - **ScrollBars** (ssNone 0, ssHorizontal 1, ssVertical 2, ssBoth 3) are
//!   the shared Windows-classic bars (`rapidr_value::scrollbars::Scroller`),
//!   shown when the text doesn't fit (HideScrollBars); a held arrow
//!   repeats (the kernel's tick). Without bars the view still follows the
//!   caret, and the wheel scrolls it.
//! - **WantTabs** (False by default): Tab types a tab instead of moving the
//!   focus on (Ctrl+Tab still does).
//! - **Alignment**, **ReadOnly**, **MaxLength**, **HideSelection**, the
//!   keys (Enter breaks the line; Up / Down / PageUp / PageDown by lines,
//!   Ctrl+Home / End to the text's ends), the clipboard, Ctrl+Z, the
//!   context menu, double click (a word) and triple click (the paragraph)
//!   are the edit's (`edit.rs`).
//!
//! RapidR's RCODEEDITOR is its own view (`codeeditor/`).

use rapidr_value::objects::ops::Rect;
use rapidr_value::objects::with_textedit;

use super::edit::{background, key_in, look_of, shows_selection, EditUi, MenuState, Source, Spec};
use super::{ComponentKind, Cx, Ime, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::a11y::AccessValue;
use crate::input::{Clipboard, Mods};
use crate::paint::Painter;
use crate::store::{self, Store};
use rapidr_value::objects::a11y::{AccessNode, Action};

pub struct Memo;

/// Whether form `f`'s focused component takes a plain Tab (a QMEMO /
/// QRICHEDIT with WantTabs, a code editor unless WantTabs is False; not
/// ReadOnly) instead of the focus moving on.
pub fn takes_tab(f: &crate::tree::FormUi, store: &dyn Store) -> bool {
    f.focus.is_some_and(|i| {
        let n = &f.nodes[i];
        match n.type_name.as_str() {
            "RMEMO" | "RRICHEDIT" => store::flag(store, &n.id, "wanttabs", false) && !with_textedit(&n.id, |t| t.read_only).unwrap_or(true),
            "RCODEEDITOR" => super::codeeditor::takes_tab(store, &n.id),
            _ => false,
        }
    })
}


/// Where a memo's parts are (its own pixels): the bars' area inside the
/// edge and the text's view.
#[derive(Clone, Copy, Debug)]
struct Geo {
    bars: Rect,
    text: Rect,
}

/// The text's view in a bars' area client of `cw` × `ch` (a pixel of
/// margin top and bottom, two left and right, as Windows' edit).
fn text_area(cw: i64, ch: i64) -> Rect {
    (4, 3, (cw - 4).max(1), (ch - 2).max(1))
}

impl Memo {
    /// The memo's editor laid out for its view (which bars show depends on
    /// the text, and a vertical bar narrows where it wraps), its bars'
    /// ranges, the scroll.
    fn setup<'c>(cx: &'c mut Cx) -> (&'c mut EditUi, Geo, Spec) {
        let (w, h) = (cx.width(), cx.height());
        let bars = (2, 2, (w - 4).max(0), (h - 4).max(0));
        let look = look_of(cx.store, cx.id, &cx.font, cx.state.enabled, true);
        let sb = store::int(cx.store, cx.id, "scrollbars", 0);
        let wrap = look.wrap;
        let mut spec = Spec { look, width: 0.0, multi: true, src: Source::Text };
        let id = cx.id;
        let scale = cx.scale;
        let mut text = text_area(bars.2, bars.3);
        spec.width = text.2 as f64;
        let mut e = spec.editor(&mut *cx.ui, &mut *cx.text, id, scale);
        e.bars.vert.visible = sb & 2 != 0;
        e.bars.horz.visible = sb & 1 != 0 && !wrap;
        for _ in 0..3 {
            let (cw, ch) = e.bars.client(bars.2, bars.3);
            text = text_area(cw, ch);
            spec.width = text.2 as f64;
            e = spec.editor(&mut *cx.ui, &mut *cx.text, id, scale);
            let s = f64::from(e.ed.scale()).max(0.01);
            let (tw, th) = e.ed.content_size();
            let lh = (e.ed.line_height() / s).round().max(1.0) as i64;
            let b = &mut e.bars;
            b.vert.range = (th / s).ceil() as i64 + (bars.3 - text.3).max(0) - 1;
            b.horz.range = (tw / s).ceil() as i64 + (bars.2 - text.2).max(0) + 1;
            b.vert.increment = lh;
            b.horz.increment = 8;
            let before = (b.vert.shown, b.horz.shown);
            b.update(bars.2, bars.3, &[]);
            if (b.vert.shown, b.horz.shown) == before {
                break;
            }
        }
        let geo = Geo { bars, text };
        let e = spec.editor(&mut *cx.ui, &mut *cx.text, id, scale);
        Self::fit_scroll(e, geo);
        (e, geo, spec)
    }


    /// What of the text shows (device pixels).
    fn view(e: &EditUi, geo: Geo) -> (f64, f64) {
        let s = f64::from(e.ed.scale());
        ((geo.text.2 as f64 * s).max(1.0), (geo.text.3 as f64 * s).max(1.0))
    }

    /// The scroll kept within the text, and the bars' thumbs where it is.
    fn fit_scroll(e: &mut EditUi, geo: Geo) {
        let view = Self::view(e, geo);
        let (tw, th) = e.ed.content_size();
        e.set_scroll(e.scroll(), view, (tw + 1.0, th));
        let s = f64::from(e.ed.scale()).max(0.01);
        let (sx, sy) = e.scroll();
        e.bars.horz.position = (sx / s).round() as i64;
        e.bars.vert.position = (sy / s).round() as i64;
    }

    /// The bars moved (the mouse on them): the text follows.
    fn from_bars(e: &mut EditUi, geo: Geo) {
        let s = f64::from(e.ed.scale());
        let to = (e.bars.horz.position as f64 * s, e.bars.vert.position as f64 * s);
        let view = Self::view(e, geo);
        let (tw, th) = e.ed.content_size();
        e.set_scroll(to, view, (tw + 1.0, th));
    }
}

impl ComponentKind for Memo {
    fn name(&self) -> &'static str {
        "RMEMO"
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        if p.fluent() && crate::store::int(cx.store, cx.id, "align", 0) == 5 {
            // (a memo filling its parent — a notepad's page — is the
            // window's own surface: square, a hairline, no focus ring
            // around the whole window)
            p.fill((0, 0, w, h), background(cx.store, cx.id));
            p.frame((0, 0, w, h), p.theme().border);
        } else if p.fluent() {
            p.fluent_field(w, h, background(cx.store, cx.id), Some(cx.state.focused));
        } else {
            p.fill((0, 0, w, h), background(cx.store, cx.id));
            p.sunken_edge((0, 0, w, h));
        }
        let (focused, caret_on) = (cx.state.focused, cx.state.caret_on);
        let show = shows_selection(cx.store, cx.id, focused);
        let id = cx.id.to_string();
        let (e, geo, _) = Self::setup(cx);
        e.paint_text(&id, p, geo.text, 0.0, show, focused && caret_on);
        let (bx, by, bw, bh) = geo.bars;
        if e.bars.vert.shown || e.bars.horz.shown {
            let ops = crate::paint::bar_ops(&e.bars, bw, bh);
            p.at((bx, by), |p| p.clipped((0, 0, bw, bh), |p| p.ops(ops)));
        }
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let id = cx.id.to_string();
        let mut wake = None;
        let (changed, on_bars) = {
            let (e, geo, _) = Self::setup(cx);
            let (bx, by, bw, bh) = geo.bars;
            let (lx, ly) = ((m.x - bx as f64).floor() as i64, (m.y - by as f64).floor() as i64);
            if m.kind == MouseKind::Down && e.bars.mouse_down(lx, ly, bw, bh).is_some() {
                e.hold_bars(Some((lx, ly)));
                Self::from_bars(e, geo);
                wake = Some(crate::tick::now() + crate::tick::REPEAT_DELAY);
                (false, true)
            } else if e.bars_held() {
                match m.kind {
                    MouseKind::Move => {
                        e.hold_bars(Some((lx, ly)));
                        e.bars.mouse_drag(lx, ly, bw, bh);
                    }
                    MouseKind::Up => {
                        e.bars.mouse_up(bw, bh);
                        e.hold_bars(None);
                    }
                    _ => {}
                }
                Self::from_bars(e, geo);
                (false, true)
            } else {
                if !e.mouse_text(m, geo.text, 0.0, true) {
                    return MouseOut::default();
                }
                let changed = e.sync_model(&id);
                let v = Self::view(e, geo);
                e.scroll_to_caret_in(v);
                Self::fit_scroll(e, geo);
                // (the input lane's: a drag out of the view scrolls on — `tick`)
                if e.drag_outside(geo.text) {
                    wake = Some(crate::tick::now() + crate::tick::REPEAT);
                }
                (changed, false)
            }
        };
        if wake.is_some() && !on_bars && cx.ui.wake.is_none() {
            cx.ui.wake = wake;
        }
        if on_bars {
            cx.ui.wake = if m.kind == MouseKind::Up { None } else { wake.or(cx.ui.wake) };
            // (a press on the bars doesn't take the focus, as Windows')
            return MouseOut { press: false, focus: Some(false) };
        }
        if changed {
            cx.change();
        }
        MouseOut::default()
    }

    fn tick(&self, cx: &mut Cx) {
        let id = cx.id.to_string();
        let (e, geo, _) = Self::setup(cx);
        let Some((x, y)) = e.held_at() else {
            // (the input lane's: a drag out of the view, a line on)
            let v = Self::view(e, geo);
            if e.auto_scroll(geo.text, 0.0, v) {
                e.sync_model(&id);
                Self::fit_scroll(e, geo);
                cx.ui.wake = Some(crate::tick::now() + crate::tick::REPEAT);
            }
            return;
        };
        let (_, _, bw, bh) = geo.bars;
        e.bars.repeat(x, y, bw, bh);
        Self::from_bars(e, geo);
        cx.ui.wake = Some(crate::tick::now() + crate::tick::REPEAT);
    }

    fn wheel(&self, cx: &mut Cx, dx: f64, dy: f64, mods: Mods) -> bool {
        let (e, geo, _) = Self::setup(cx);
        let view = Self::view(e, geo);
        let (tw, th) = e.ed.content_size();
        let (dx, dy) = if mods.shift && dx == 0.0 { (dy, 0.0) } else { (dx, dy) };
        let lh = e.ed.line_height();
        let (sx, sy) = e.scroll();
        let to = (sx + dx * 3.0 * lh, sy + dy * 3.0 * lh);
        e.set_scroll(to, view, (tw + 1.0, th));
        let moved = e.scroll() != (sx, sy);
        Self::fit_scroll(e, geo);
        // (a text that fits passes the wheel on)
        moved || th > view.1 || (tw > view.0 && !e.ed.look().wrap)
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, clip: &mut dyn Clipboard) -> bool {
        let (geo, spec) = {
            let (_, geo, spec) = Self::setup(cx);
            (geo, spec)
        };
        let handled = key_in(cx, &spec, k, clip, |e| Self::view(e, geo));
        let (e, geo, _) = Self::setup(cx);
        Self::fit_scroll(e, geo);
        handled
    }

    fn ime(&self, cx: &mut Cx, ime: &Ime) -> bool {
        if !self.wants_ime(cx.store, cx.id) {
            return false;
        }
        let (geo, spec) = {
            let (_, geo, spec) = Self::setup(cx);
            (geo, spec)
        };
        super::edit::ime_box(cx, &spec, ime, |e| Self::view(e, geo));
        let (e, geo, _) = Self::setup(cx);
        Self::fit_scroll(e, geo);
        true
    }

    fn ime_area(&self, cx: &mut Cx) -> Option<Rect> {
        let (x0, y0) = (cx.rect.0, cx.rect.1);
        let (e, geo, _) = Self::setup(cx);
        Some(e.caret_area((x0 + geo.text.0, y0 + geo.text.1), 0.0))
    }

    fn wants_ime(&self, _store: &dyn Store, id: &str) -> bool {
        !with_textedit(id, |t| t.read_only).unwrap_or(true)
    }

    fn context_menu(&self, cx: &mut Cx) -> Option<MenuState> {
        let id = cx.id.to_string();
        let (e, _, _) = Self::setup(cx);
        Some(e.menu_state_of(&id))
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        super::edit::describe_text(cx, true)
    }

    fn access(&self, cx: &mut Cx, action: Action, _part: Option<usize>, value: Option<&AccessValue>) -> bool {
        super::edit::access_text(cx, action, value)
    }
}
