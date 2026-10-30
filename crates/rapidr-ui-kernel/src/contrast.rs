//! Text against what it is drawn on, from a display list (WCAG 2.x): every
//! text run — a text op, an editor's glyph run — with the colour it is
//! drawn in and the colour of the fill under it (the topmost fill, rounded
//! rectangle or polygon under most of its box), their contrast ratio and
//! the ratio it needs. A test's hook (`RAPIDR_TEST_CONTRAST`, the web's
//! test results) writes the runs that read too faintly, so a theme can't
//! leave text unreadable unseen: the same list the window shows, on both
//! hosts.
//!
//! What it needs: 4.5:1 (WCAG AA) for text, 3:1 for large text (24 px, or
//! 19 px bold) and for disabled text (the theme's `gray_text`: WCAG asks
//! nothing of inactive controls, RapidR's themes keep them at 3:1 to be
//! seen). Text over a picture (an image op) has no known ground and isn't
//! judged; nor are the form designer's ghosts (a fade).
//!
//! The same pass says which text doesn't fit ([`Run::cut`]): a run wider or
//! taller than the box it is laid out in, or cut by a clip — what a longer
//! language (the pseudo-language `xx`, Spanish) shows first. The reports
//! list every run's text (`texts`), so a test can also tell text that came
//! from a catalog from text that didn't.

use rapidr_value::objects::ops::{Op, Place, Rect};
use rapidr_value::objects::text::text_size;
use rapidr_value::theme::{contrast, Theme};

use crate::display::{DisplayList, Item};
use crate::tree::FormUi;

/// A text run and what it reads against.
#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    pub text: String,
    /// Its box in the window (logical pixels).
    pub rect: (f64, f64, f64, f64),
    /// 0xRRGGBB.
    pub color: u32,
    pub background: u32,
    pub ratio: f64,
    /// The least it needs ([`need`]).
    pub need: f64,
    /// It doesn't fit: wider or taller than the box it is laid out in, or
    /// cut by a clip (a text op's; a code editor's runs are never).
    pub cut: bool,
    /// A code editor's glyph run (the program's own text), not a caption.
    pub editor: bool,
}

impl Run {
    pub fn fails(&self) -> bool {
        self.ratio + 1e-9 < self.need
    }
}

/// What a run of text in `color`, `px` logical pixels high, needs in
/// theme `t` ([module docs](self)).
pub fn need(t: &Theme, color: u32, px: f64, bold: bool) -> f64 {
    if color == t.gray_text || px >= 24.0 || (bold && px >= 18.66) {
        3.0
    } else {
        4.5
    }
}

/// A ground text can be on.
#[derive(Clone, Debug)]
enum Ground {
    Rect(F4, u32),
    Round(F4, f64, u32),
    Polygon(Vec<(f64, f64)>, F4, u32),
    /// A picture: nothing known under it.
    Unknown(F4),
}

type F4 = (f64, f64, f64, f64);

fn inside((x, y, w, h): F4, (px, py): (f64, f64)) -> bool {
    px >= x && py >= y && px < x + w && py < y + h
}

fn meet(a: F4, b: F4) -> F4 {
    let (x0, y0) = (a.0.max(b.0), a.1.max(b.1));
    let (x1, y1) = ((a.0 + a.2).min(b.0 + b.2), (a.1 + a.3).min(b.1 + b.3));
    (x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))
}

impl Ground {
    /// Its colour at `p`: `Some(None)` over a picture, `None` if not there.
    fn at(&self, p: (f64, f64)) -> Option<Option<u32>> {
        match self {
            Ground::Rect(r, c) => inside(*r, p).then_some(Some(*c)),
            Ground::Unknown(r) => inside(*r, p).then_some(None),
            Ground::Round(r, radius, c) => {
                if !inside(*r, p) {
                    return None;
                }
                // (outside a rounded corner: not on it)
                let (x, y, w, h) = *r;
                let rad = radius.min(w / 2.0).min(h / 2.0).max(0.0);
                let cx = p.0.clamp(x + rad, x + w - rad);
                let cy = p.1.clamp(y + rad, y + h - rad);
                ((p.0 - cx).powi(2) + (p.1 - cy).powi(2) <= rad * rad + 1e-9).then_some(Some(*c))
            }
            Ground::Polygon(points, clip, c) => {
                if !inside(*clip, p) {
                    return None;
                }
                let mut odd = false;
                let n = points.len();
                for i in 0..n {
                    let (a, b) = (points[i], points[(i + n - 1) % n]);
                    if (a.1 > p.1) != (b.1 > p.1) && p.0 < (b.0 - a.0) * (p.1 - a.1) / (b.1 - a.1) + a.0 {
                        odd = !odd;
                    }
                }
                odd.then_some(Some(*c))
            }
        }
    }
}

/// Where a list's ops land in the window: logical pixels, magnified
/// inside the designer's zoom.
#[derive(Clone, Copy)]
struct Place2 {
    scale: f64,
    at: (f64, f64),
}

impl Place2 {
    fn rect(&self, (ox, oy): (i64, i64), (x, y, w, h): (f64, f64, f64, f64)) -> F4 {
        ((ox as f64 + x) * self.scale + self.at.0, (oy as f64 + y) * self.scale + self.at.1, w * self.scale, h * self.scale)
    }
    fn point(&self, (ox, oy): (i64, i64), (x, y): (f64, f64)) -> (f64, f64) {
        ((ox as f64 + x) * self.scale + self.at.0, (oy as f64 + y) * self.scale + self.at.1)
    }
}

fn f4((x, y, w, h): Rect) -> F4 {
    (x as f64, y as f64, w as f64, h as f64)
}

/// A text op's box: where `text_size` puts it in `rect` by `place`.
fn text_box(rect: Rect, text: &str, font: &rapidr_value::objects::font::Font, angle: i32, place: Place) -> F4 {
    if angle != 0 {
        return f4(rect);
    }
    let (tw, th) = text_size(text, font);
    let (x, y, w, h) = rect;
    let left = match place {
        Place::TopLeft | Place::Left => x,
        Place::TopRight => x + w - tw,
        Place::Center | Place::TopCenter => x + (w - tw) / 2,
    };
    let top = match place {
        Place::TopLeft | Place::TopCenter | Place::TopRight => y,
        Place::Center | Place::Left => y + (h - th) / 2,
    };
    (left as f64, top as f64, tw as f64, th as f64)
}

/// The colour most of `area` is drawn on (five points: its middle and four
/// around it), `None` when that's unknown (a picture, nothing drawn).
fn ground_under(grounds: &[Ground], area: F4) -> Option<u32> {
    let (x, y, w, h) = area;
    let points = [(0.5, 0.5), (0.25, 0.3), (0.75, 0.3), (0.25, 0.7), (0.75, 0.7)].map(|(fx, fy)| (x + w * fx, y + h * fy));
    let mut seen: Vec<(Option<u32>, usize)> = Vec::new();
    for p in points {
        let c = grounds.iter().rev().find_map(|g| g.at(p)).flatten();
        match seen.iter_mut().find(|(k, _)| *k == c) {
            Some((_, n)) => *n += 1,
            None => seen.push((c, 1)),
        }
    }
    // (the most points; the middle's on a tie — it came first)
    seen.iter().fold(None::<(Option<u32>, usize)>, |best, &(c, n)| match best {
        Some((_, bn)) if bn >= n => best,
        _ => Some((c, n)),
    })?
    .0
}

/// Every text run of `list` (form `form`'s, for its editors' layouts) that
/// shows, with its ground and contrast, in theme `t`.
pub fn runs(list: &DisplayList, form: &FormUi, t: &Theme) -> Vec<Run> {
    let base = Place2 { scale: 1.0, at: (0.0, 0.0) };
    let mut place = base;
    let mut clips: Vec<F4> = vec![(0.0, 0.0, list.size.0 as f64, list.size.1 as f64)];
    let mut grounds: Vec<Ground> = Vec::new();
    let mut fade = 0usize;
    let mut out = Vec::new();
    let clip = |clips: &Vec<F4>| *clips.last().expect("the window's clip");
    for item in &list.items {
        match item {
            Item::Zoom(z) => {
                place = z.map_or(base, |z| Place2 { scale: z.scale / list.scale, at: (z.at.0 / list.scale, z.at.1 / list.scale) });
            }
            Item::Op { origin, op } => match op {
                Op::ClipPush { rect } => {
                    let r = meet(clip(&clips), place.rect(*origin, f4(*rect)));
                    clips.push(r);
                }
                Op::ClipPolygon { points } => {
                    let pts: Vec<(f64, f64)> = points.iter().map(|&p| place.point(*origin, p)).collect();
                    let (x0, y0) = pts.iter().fold((f64::MAX, f64::MAX), |(a, b), p| (a.min(p.0), b.min(p.1)));
                    let (x1, y1) = pts.iter().fold((f64::MIN, f64::MIN), |(a, b), p| (a.max(p.0), b.max(p.1)));
                    let r = meet(clip(&clips), (x0, y0, x1 - x0, y1 - y0));
                    clips.push(r);
                }
                Op::ClipPop => {
                    if clips.len() > 1 {
                        clips.pop();
                    }
                }
                Op::Fade { .. } => fade += 1,
                Op::FadePop => fade = fade.saturating_sub(1),
                _ if fade > 0 => {}
                Op::Fill { rect, color } => grounds.push(Ground::Rect(meet(clip(&clips), place.rect(*origin, f4(*rect))), *color)),
                Op::Checker { rect, a, .. } => grounds.push(Ground::Rect(meet(clip(&clips), place.rect(*origin, f4(*rect))), *a)),
                Op::Round { rect, radius, fill: Some(c), .. } => {
                    let r = place.rect(*origin, f4(*rect));
                    let cl = clip(&clips);
                    // (clipped: a rectangle where the clip cuts it)
                    if meet(cl, r) == r {
                        grounds.push(Ground::Round(r, radius * place.scale, *c));
                    } else {
                        grounds.push(Ground::Rect(meet(cl, r), *c));
                    }
                }
                Op::Polygon { points, color } => {
                    grounds.push(Ground::Polygon(points.iter().map(|&p| place.point(*origin, p)).collect(), clip(&clips), *color));
                }
                Op::Image { rect, .. } => grounds.push(Ground::Unknown(meet(clip(&clips), place.rect(*origin, f4(*rect))))),
                Op::Text { rect, text, font, color, angle, place: at } => {
                    if text.trim().is_empty() {
                        continue;
                    }
                    let full = place.rect(*origin, text_box(*rect, text, font, *angle, *at));
                    let shown = meet(clip(&clips), full);
                    if shown.2 < 1.0 || shown.3 < 1.0 {
                        continue;
                    }
                    let Some(bg) = ground_under(&grounds, shown) else { continue };
                    let px = font.pixel_size() as f64 * place.scale;
                    // (cut off: a clip takes part of it, or it spills out of
                    // the box it was laid out in — two pixels of slack for
                    // the fonts' side bearings)
                    let boxed = place.rect(*origin, f4(*rect));
                    let spills = *angle == 0 && (full.0 < boxed.0 - 2.0 || full.0 + full.2 > boxed.0 + boxed.2 + 2.0);
                    let clipped = shown.2 + 2.0 < full.2 || shown.3 + 2.0 < full.3 * 0.75;
                    out.push(Run { text: text.clone(), rect: shown, color: *color, background: bg, ratio: contrast(*color, bg), need: need(t, *color, px, font.styles & 1 != 0), cut: spills || clipped, editor: false });
                }
                _ => {}
            },
            Item::Text(ti) => {
                let Some(layout) = form.editor_layout_at(&ti.node, ti.para) else { continue };
                let dev = |(x0, y0, x1, y1): (f64, f64, f64, f64)| ((ti.origin.0 + x0) / list.scale, (ti.origin.1 + y0) / list.scale, (x1 - x0) / list.scale, (y1 - y0) / list.scale);
                let selection: Vec<F4> = ti.selection.iter().map(|r| meet(clip(&clips), dev(*r))).collect();
                for r in &selection {
                    grounds.push(Ground::Rect(*r, ti.highlight));
                }
                for line in layout.lines() {
                    for it in line.items() {
                        let parley::PositionedLayoutItem::GlyphRun(g) = it else { continue };
                        let run = g.run();
                        let m = run.metrics();
                        let (x0, base) = (f64::from(g.offset()), f64::from(g.baseline()));
                        let r = dev((x0, base - f64::from(m.ascent), x0 + f64::from(g.advance()), base + f64::from(m.descent)));
                        let shown = meet(clip(&clips), r);
                        if shown.2 < 1.0 || shown.3 < 1.0 || g.positioned_glyphs().next().is_none() {
                            continue;
                        }
                        let Some(bg) = ground_under(&grounds, shown) else { continue };
                        let mid = (shown.0 + shown.2 / 2.0, shown.1 + shown.3 / 2.0);
                        let color = if selection.iter().any(|s| inside(*s, mid)) { ti.highlight_text } else { g.style().brush.0 };
                        let px = f64::from(run.font_size()) / list.scale;
                        out.push(Run { text: format!("{}#{} run", ti.node, ti.para), rect: shown, color, background: bg, ratio: contrast(color, bg), need: need(t, color, px, false), cut: false, editor: true });
                    }
                }
            }
        }
    }
    out
}

/// The runs that fail, as JSON (`{"form", "checked", "failures": [{text,
/// rect, color, background, ratio, need}]}`), what the test hooks write.
pub fn report_json(form_id: &str, runs: &[Run]) -> String {
    let q = |s: &str| {
        let mut o = String::from("\"");
        for c in s.chars() {
            match c {
                '"' => o.push_str("\\\""),
                '\\' => o.push_str("\\\\"),
                '\n' => o.push_str("\\n"),
                c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
                c => o.push(c),
            }
        }
        o.push('"');
        o
    };
    let failures: Vec<String> = runs
        .iter()
        .filter(|r| r.fails())
        .map(|r| {
            format!(
                "{{\"text\":{},\"rect\":[{:.0},{:.0},{:.0},{:.0}],\"color\":\"#{:06x}\",\"background\":\"#{:06x}\",\"ratio\":{:.2},\"need\":{:.1}}}",
                q(&r.text.chars().take(60).collect::<String>()),
                r.rect.0,
                r.rect.1,
                r.rect.2,
                r.rect.3,
                r.color,
                r.background,
                r.ratio,
                r.need
            )
        })
        .collect();
    // (every caption's text and box, and whether it fits: the language tests')
    let texts: Vec<String> = runs
        .iter()
        .filter(|r| !r.editor)
        .map(|r| format!("{{\"text\":{},\"rect\":[{:.0},{:.0},{:.0},{:.0}],\"cut\":{}}}", q(&r.text.chars().take(200).collect::<String>()), r.rect.0, r.rect.1, r.rect.2, r.rect.3, r.cut))
        .collect();
    format!("{{\"form\":{},\"checked\":{},\"failures\":[{}],\"texts\":[{}]}}", q(form_id), runs.len(), failures.join(","), texts.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rapidr_value::theme::{self, ALL};
    use rapidr_value::{v_int, v_str};

    use crate::{FormUi, MemStore, Mods, TextSystem};

    /// A form with the components whose rows have states: a list, a tree,
    /// a grid, a combo box, beside a label, an edit, a check box, a button.
    fn store() -> MemStore {
        let mut s = MemStore::new();
        let put = |s: &mut MemStore, id: &str, t: &str, (l, top, w, h): (i64, i64, i64, i64)| {
            s.add(id, t, Some("cf")).set(id, "left", v_int(l)).set(id, "top", v_int(top)).set(id, "width", v_int(w)).set(id, "height", v_int(h));
        };
        s.add("cf", "RFORM", None).set("cf", "clientwidth", v_int(520)).set("cf", "clientheight", v_int(260));
        put(&mut s, "clst", "RLISTBOX", (8, 8, 120, 110));
        s.call("clst", "additems", &[v_str("Locals"), v_str("Globals"), v_str("count = 3"), v_str("total = 0")]);
        s.set("clst", "itemindex", v_int(1));
        put(&mut s, "ctre", "RTREEVIEW", (136, 8, 140, 110));
        s.call("ctre", "additems", &[v_str("(the program)  shop.rr:147"), v_str("AddUp  shop.rr:76"), v_str("PriceOf  shop.rr:40")]);
        s.set("ctre", "itemindex", v_int(1)).set("ctre", "hideselection", v_int(0));
        put(&mut s, "cgrd", "RSTRINGGRID", (284, 8, 228, 110));
        s.set("cgrd", "colcount", v_int(3)).set("cgrd", "rowcount", v_int(4));
        for r in 0..4 {
            for c in 0..3 {
                s.call("cgrd", "cell", &[v_int(c), v_int(r), v_str(&format!("r{r}c{c}"))]);
            }
        }
        s.set("cgrd", "row", v_int(2));
        put(&mut s, "ccmb", "RCOMBOBOX", (8, 126, 140, 24));
        s.call("ccmb", "additems", &[v_str("When"), v_str("Hit")]);
        s.set("ccmb", "itemindex", v_int(0));
        put(&mut s, "clbl", "RLABEL", (160, 130, 60, 17));
        s.set("clbl", "caption", v_str("When"));
        put(&mut s, "cedt", "REDIT", (230, 126, 120, 24));
        s.set("cedt", "text", v_str("count > 1"));
        put(&mut s, "cchk", "RCHECKBOX", (360, 128, 60, 20));
        s.set("cchk", "caption", v_str("On"));
        put(&mut s, "cbtn", "RBUTTON", (430, 126, 80, 26));
        s.set("cbtn", "caption", v_str("Continue"));
        s
    }

    /// What fails in `t` once `prepare` put the form in a state.
    fn failures(s: &MemStore, t: &'static Theme, prepare: impl Fn(&mut FormUi, &MemStore, &mut TextSystem)) -> Vec<String> {
        let was = theme::current();
        theme::set(t);
        let mut ts = TextSystem::new();
        let mut f = FormUi::build_unfocused(s, "cf", false);
        drop(f.paint(s, &mut ts, 1.0));
        prepare(&mut f, s, &mut ts);
        let list = f.paint(s, &mut ts, 1.0);
        let out = runs(&list, &f, t).into_iter().filter(Run::fails).map(|r| format!("{} {:?} #{:06x} on #{:06x} {:.2}:1", t.name, r.text, r.color, r.background, r.ratio)).collect();
        theme::set(was);
        out
    }

    #[test]
    fn every_row_state_reads_in_rapidrs_looks() {
        // (normal rows, the selection with the focus and without, the row
        // under the mouse, everything disabled — in light, dark and high
        // contrast: WCAG AA, disabled text 3:1)
        let mut bad = Vec::new();
        for t in ALL.into_iter().filter(|t| t.fluent()) {
            let s = store();
            bad.extend(failures(&s, t, |_, _, _| {}));
            for id in ["clst", "ctre", "cgrd", "ccmb", "cedt"] {
                bad.extend(failures(&s, t, |f, s, _| {
                    f.focus_id(s, id);
                }));
            }
            // (the mouse over a row that isn't selected)
            bad.extend(failures(&s, t, |f, s, ts| f.mouse_move(s, ts, 30.0, 60.0, Mods::NONE)));
            bad.extend(failures(&s, t, |f, s, ts| f.mouse_move(s, ts, 160.0, 14.0, Mods::NONE)));
            let mut off = store();
            for id in ["clst", "ctre", "cgrd", "ccmb", "clbl", "cedt", "cchk", "cbtn"] {
                off.set(id, "enabled", v_int(0));
            }
            bad.extend(failures(&off, t, |_, _, _| {}));
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }

    #[test]
    fn text_the_program_left_to_the_theme_follows_it() {
        // (a QFONT assigned to a parent gives clWindowText — the theme's
        // text, not black; a stored black that no one chose is no choice)
        for t in ALL.into_iter().filter(|t| t.fluent() && t.dark) {
            let mut s = store();
            s.set("ctre", "font.color", v_int(rapidr_value::component_defaults::CL_WINDOW_TEXT)).set("clst", "fontcolor", v_int(0));
            let bad = failures(&s, t, |_, _, _| {});
            assert!(bad.is_empty(), "{}", bad.join("\n"));
            // (black the program did choose is drawn black, as RapidQ draws it)
            s.set("clbl", "font.color", v_int(0)).set("clbl", "fontcolor", v_int(0));
            let bad = failures(&s, t, |_, _, _| {});
            assert!(bad.iter().any(|b| b.contains("\"When\"") && b.contains("#000000")), "{bad:?}");
        }
    }

    #[test]
    fn grounds_and_needs() {
        let t = &theme::RAPIDR;
        assert_eq!(need(t, t.gray_text, 11.0, false), 3.0);
        assert_eq!(need(t, t.text, 11.0, false), 4.5);
        assert_eq!(need(t, t.text, 24.0, false), 3.0);
        assert_eq!(need(t, t.text, 19.0, true), 3.0);
        let g = [Ground::Rect((0.0, 0.0, 100.0, 100.0), 0xFFFFFF), Ground::Round((10.0, 10.0, 40.0, 20.0), 6.0, 0x000000)];
        assert_eq!(ground_under(&g, (12.0, 12.0, 30.0, 14.0)), Some(0x000000));
        assert_eq!(ground_under(&g, (60.0, 60.0, 30.0, 14.0)), Some(0xFFFFFF));
        // (a rounded corner's outside is what's under it)
        assert_eq!(g[1].at((10.5, 10.5)), None);
        let pic = [Ground::Rect((0.0, 0.0, 100.0, 100.0), 0xFFFFFF), Ground::Unknown((0.0, 0.0, 50.0, 50.0))];
        assert_eq!(ground_under(&pic, (5.0, 5.0, 20.0, 10.0)), None);
    }
}
