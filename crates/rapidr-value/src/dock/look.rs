//! What a dock manager looks like, as the kernel's ops, in every theme:
//!
//! - **modern / dark** (Fluent; VS Code, Rider, Visual Studio 2022): the
//!   groups as panels on a slightly darker ground with 4-pixel gutters, a
//!   1-pixel border; a header with the pane's icon and title (one pane) or
//!   tabs (more), the shown tab underlined, the active group's header
//!   tinted with the accent's line on top; hover fills on tabs and
//!   buttons; documents as tabs (the shown one joined to the page, the
//!   accent along its top) or MDI windows on the workspace.
//! - **classic** (Delphi 7 / Visual Studio 6): title bars in the
//!   caption's colours (the active group's in the active caption's), tabs
//!   as raised 3D tabs, the workspace Windows' AppWorkspace grey.
//! - **highcontrast**: black, white borders everywhere, the accent's
//!   thick lines for what's active, yellow for what's under the mouse.
//!
//! Icons and glyphs are drawn as shapes (crisp at every scale).

use super::geometry::{self, Button, DocHit, Documents, Geometry, Group, GroupHit, Guide, Slot, Titles, GUIDE, HEADER};
use super::manager::{Manager, Part};
use super::{Anchor, Rect, Side, Target};
use crate::objects::font::Font;
use crate::objects::ops::{Op, Place};
use crate::theme::{Look, Theme};

/// The colours of a dock manager's chrome in a theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub classic: bool,
    pub contrast: bool,
    /// Behind the groups: the gutters, the strips.
    pub ground: u32,
    pub header: u32,
    pub header_active: u32,
    pub title: u32,
    pub title_active: u32,
    pub tab_text: u32,
    pub tab_hover: u32,
    pub accent: u32,
    /// The shown tab's underline in a group that isn't active.
    pub underline: u32,
    pub border: u32,
    pub body: u32,
    pub workspace: u32,
    pub doc_strip: u32,
    pub doc_tab: u32,
    pub doc_tab_hover: u32,
    pub glyph: u32,
    pub button_hover: u32,
    pub button_pressed: u32,
    pub splitter_hot: u32,
    pub strip_tab: u32,
    pub strip_tab_hover: u32,
    pub strip_text: u32,
    pub guide_face: u32,
    pub guide_border: u32,
    pub guide_hot: u32,
    pub preview: u32,
    pub preview_text: u32,
    pub hot_outline: u32,
    pub shadow: u32,
}

/// `a` toward `b` by `t` (0..=1).
pub fn mix(a: u32, b: u32, t: f64) -> u32 {
    let ch = |s: u32| {
        let (x, y) = (((a >> s) & 0xFF) as f64, ((b >> s) & 0xFF) as f64);
        ((x + (y - x) * t).round() as u32 & 0xFF) << s
    };
    ch(16) | ch(8) | ch(0)
}

pub fn palette(t: &Theme) -> Palette {
    let contrast = t.name == "highcontrast";
    if t.look == Look::Classic {
        return Palette {
            classic: true,
            contrast: false,
            ground: t.face,
            header: t.inactive_caption,
            header_active: t.caption,
            title: t.inactive_caption_text,
            title_active: t.caption_text,
            tab_text: t.inactive_caption_text,
            tab_hover: mix(t.inactive_caption, t.light, 0.25),
            accent: t.caption,
            underline: t.shadow,
            border: t.shadow,
            body: t.face,
            workspace: 0x808080,
            doc_strip: t.face,
            doc_tab: t.face,
            doc_tab_hover: mix(t.face, t.light, 0.6),
            glyph: t.text,
            button_hover: t.face,
            button_pressed: t.face,
            splitter_hot: t.shadow,
            strip_tab: t.face,
            strip_tab_hover: t.light,
            strip_text: t.text,
            guide_face: t.face,
            guide_border: t.dark_shadow,
            guide_hot: t.highlight,
            preview: mix(t.highlight, t.window, 0.75),
            preview_text: t.text,
            hot_outline: t.highlight,
            shadow: t.shadow,
        };
    }
    if contrast {
        return Palette {
            classic: false,
            contrast: true,
            ground: 0x000000,
            header: 0x000000,
            header_active: 0x000000,
            title: t.text,
            title_active: t.text,
            tab_text: t.text,
            tab_hover: 0x000000,
            accent: t.accent,
            underline: t.text,
            border: t.border,
            body: t.window,
            workspace: 0x000000,
            doc_strip: 0x000000,
            doc_tab: 0x000000,
            doc_tab_hover: 0x000000,
            glyph: t.text,
            button_hover: 0x000000,
            button_pressed: t.accent,
            splitter_hot: t.hot_text,
            strip_tab: 0x000000,
            strip_tab_hover: 0x000000,
            strip_text: t.text,
            guide_face: 0x000000,
            guide_border: t.text,
            guide_hot: t.accent,
            preview: 0x000000,
            preview_text: t.text,
            hot_outline: t.hot_text,
            shadow: t.text,
        };
    }
    let dark = t.dark;
    Palette {
        classic: false,
        contrast: false,
        ground: if dark { mix(t.face, 0x000000, 0.32) } else { mix(t.face, t.shadow, 0.62) },
        header: t.face,
        header_active: if dark { mix(t.face, t.accent, 0.10) } else { mix(t.window, t.accent, 0.07) },
        title: mix(t.text, t.face, if dark { 0.22 } else { 0.18 }),
        title_active: t.text,
        tab_text: mix(t.text, t.face, if dark { 0.42 } else { 0.40 }),
        tab_hover: if dark { mix(t.face, 0xFFFFFF, 0.07) } else { mix(t.face, 0x000000, 0.045) },
        accent: if dark { t.accent } else { t.caption },
        underline: if dark { mix(t.face, 0xFFFFFF, 0.30) } else { mix(t.face, 0x000000, 0.28) },
        border: if dark { mix(t.border, t.face, 0.25) } else { t.border },
        body: t.window,
        workspace: if dark { mix(t.face, 0x000000, 0.45) } else { mix(t.face, t.shadow, 0.65) },
        doc_strip: t.face,
        doc_tab: t.window,
        doc_tab_hover: if dark { mix(t.face, 0xFFFFFF, 0.06) } else { mix(t.face, 0x000000, 0.04) },
        glyph: mix(t.text, t.face, 0.30),
        button_hover: if dark { mix(t.face, 0xFFFFFF, 0.12) } else { mix(t.face, 0x000000, 0.08) },
        button_pressed: if dark { mix(t.face, 0xFFFFFF, 0.18) } else { mix(t.face, 0x000000, 0.14) },
        splitter_hot: if dark { t.accent } else { t.caption },
        strip_tab: if dark { mix(t.face, 0xFFFFFF, 0.04) } else { t.window },
        strip_tab_hover: if dark { mix(t.face, 0xFFFFFF, 0.12) } else { mix(t.window, t.accent, 0.10) },
        strip_text: mix(t.text, t.face, 0.15),
        guide_face: if dark { mix(t.face, 0xFFFFFF, 0.06) } else { t.window },
        guide_border: if dark { mix(t.face, 0xFFFFFF, 0.35) } else { mix(t.border_strong, t.window, 0.35) },
        guide_hot: if dark { t.accent } else { t.caption },
        preview: if dark { mix(t.face, t.accent, 0.22) } else { mix(t.window, t.caption, 0.16) },
        preview_text: t.text,
        hot_outline: if dark { t.accent } else { t.caption },
        shadow: if dark { 0x000000 } else { mix(t.shadow, 0x000000, 0.25) },
    }
}

fn fill(ops: &mut Vec<Op>, rect: Rect, color: u32) {
    if rect.2 > 0 && rect.3 > 0 {
        ops.push(Op::Fill { rect, color });
    }
}

/// A 1-pixel frame inside `r`.
fn frame(ops: &mut Vec<Op>, r: Rect, color: u32, width: i64) {
    let (x, y, w, h) = r;
    fill(ops, (x, y, w, width), color);
    fill(ops, (x, y + h - width, w, width), color);
    fill(ops, (x, y + width, width, h - 2 * width), color);
    fill(ops, (x + w - width, y + width, width, h - 2 * width), color);
}

fn raised(ops: &mut Vec<Op>, r: Rect, t: &Theme) {
    ops.push(Op::Edge { rect: r, light: vec![t.light], dark: vec![t.dark_shadow, t.shadow] });
}

fn text(ops: &mut Vec<Op>, rect: Rect, s: &str, font: &Font, color: u32, place: Place, angle: i32) {
    if s.is_empty() || rect.2 <= 0 || rect.3 <= 0 {
        return;
    }
    ops.push(Op::ClipPush { rect });
    ops.push(Op::Text { rect, text: s.to_string(), font: Font { color: crate::theme::bgr(color) as i64, ..font.clone() }, color, angle, place });
    ops.push(Op::ClipPop);
}

/// Text fitted to `w` pixels (an ellipsis when it doesn't fit).
fn fitted(s: &str, font: &Font, w: i64) -> String {
    use crate::objects::text::text_size;
    if text_size(s, font).0 <= w {
        return s.to_string();
    }
    let mut out: String = s.to_string();
    while !out.is_empty() {
        out.pop();
        let t = format!("{}…", out.trim_end());
        if text_size(&t, font).0 <= w {
            return t;
        }
    }
    String::new()
}

/// The chrome's font: the dock manager's.
fn bold(font: &Font) -> Font {
    Font { styles: font.styles | 1, ..font.clone() }
}

// ---------------------------------------------------------------- glyphs --

/// A header button's glyph centred in `r`.
fn glyph(ops: &mut Vec<Op>, b: Button, r: Rect, color: u32, pinned: bool) {
    let (cx, cy) = (r.0 as f64 + r.2 as f64 / 2.0, r.1 as f64 + r.3 as f64 / 2.0);
    match b {
        Button::Close => {
            let d = 4.0;
            ops.push(Op::Stroke { points: vec![(cx - d, cy - d), (cx + d, cy + d)], color, width: 1.3 });
            ops.push(Op::Stroke { points: vec![(cx - d, cy + d), (cx + d, cy - d)], color, width: 1.3 });
        }
        Button::Pin => {
            let (x, y) = (cx.floor() as i64, cy.floor() as i64);
            if pinned {
                // (standing: a head, its collar, the needle)
                frame(ops, (x - 3, y - 6, 7, 7), color, 1);
                fill(ops, (x, y - 6, 1, 7), color);
                fill(ops, (x - 5, y + 1, 11, 1), color);
                fill(ops, (x, y + 2, 1, 4), color);
            } else {
                // (lying: pointing left, the flyout's "dock it")
                frame(ops, (x - 2, y - 3, 7, 7), color, 1);
                fill(ops, (x - 2, y, 7, 1), color);
                fill(ops, (x - 3, y - 5, 1, 11), color);
                fill(ops, (x - 7, y, 4, 1), color);
            }
        }
        Button::Dock => {
            let (x, y) = (cx.floor() as i64 - 6, cy.floor() as i64 - 5);
            frame(ops, (x, y, 12, 10), color, 1);
            fill(ops, (x, y, 12, 2), color);
            fill(ops, (x + 1, y + 2, 4, 7), color);
        }
    }
}

/// A pane's icon (by name) in a 16 × 16 box at (x, y); unknown names draw
/// nothing. Every icon is RapidR's own, drawn as shapes.
pub fn icon(ops: &mut Vec<Op>, name: &str, x: i64, y: i64, color: u32, accent: u32) {
    let (fx, fy) = (x as f64, y as f64);
    let line = |ops: &mut Vec<Op>, pts: &[(f64, f64)], w: f64| ops.push(Op::Stroke { points: pts.iter().map(|(a, b)| (fx + a, fy + b)).collect(), color, width: w });
    match name.trim().to_ascii_lowercase().as_str() {
        "explorer" | "project" | "files" | "folder" => {
            line(ops, &[(1.5, 4.5), (6.0, 4.5), (7.5, 6.0), (14.5, 6.0), (14.5, 13.5), (1.5, 13.5), (1.5, 4.5)], 1.2);
            line(ops, &[(1.5, 8.0), (14.5, 8.0)], 1.0);
        }
        "search" | "find" => {
            ops.push(Op::Round { rect: (x + 2, y + 2, 9, 9), radius: 4.5, fill: None, stroke: Some(color), width: 1.3 });
            line(ops, &[(10.0, 10.0), (14.0, 14.0)], 1.6);
        }
        "output" | "terminal" | "console" => {
            ops.push(Op::Round { rect: (x + 1, y + 2, 14, 12), radius: 2.0, fill: None, stroke: Some(color), width: 1.2 });
            line(ops, &[(4.0, 6.0), (6.5, 8.0), (4.0, 10.0)], 1.2);
            line(ops, &[(8.0, 10.5), (12.0, 10.5)], 1.2);
        }
        "problems" | "errors" | "warning" => {
            line(ops, &[(8.0, 1.8), (14.8, 13.8), (1.2, 13.8), (8.0, 1.8)], 1.2);
            fill(ops, (x + 7, y + 6, 2, 4), color);
            fill(ops, (x + 7, y + 11, 2, 2), color);
        }
        "properties" | "inspector" | "settings" => {
            for (k, knob) in [(0, 10.0), (1, 5.0), (2, 9.0)] {
                let yy = 3.5 + k as f64 * 4.5;
                line(ops, &[(1.5, yy), (14.5, yy)], 1.1);
                ops.push(Op::Round { rect: (x + knob as i64 - 2, y + yy as i64 - 2, 5, 5), radius: 2.5, fill: Some(accent), stroke: None, width: 0.0 });
            }
        }
        "toolbox" | "components" | "palette" => {
            for (dx, dy) in [(1, 1), (9, 1), (1, 9), (9, 9)] {
                ops.push(Op::Round { rect: (x + dx, y + dy, 6, 6), radius: 1.5, fill: None, stroke: Some(color), width: 1.2 });
            }
            ops.push(Op::Round { rect: (x + 10, y + 10, 4, 4), radius: 1.0, fill: Some(accent), stroke: None, width: 0.0 });
        }
        "outline" | "tree" | "structure" => {
            fill(ops, (x + 1, y + 2, 5, 3), color);
            line(ops, &[(3.5, 5.0), (3.5, 12.5), (7.0, 12.5)], 1.1);
            line(ops, &[(3.5, 8.0), (7.0, 8.0)], 1.1);
            fill(ops, (x + 8, y + 7, 7, 3), color);
            fill(ops, (x + 8, y + 11, 7, 3), color);
        }
        "form" | "designer" | "window" => {
            ops.push(Op::Round { rect: (x + 1, y + 2, 14, 12), radius: 1.5, fill: None, stroke: Some(color), width: 1.2 });
            fill(ops, (x + 2, y + 3, 12, 3), accent);
            fill(ops, (x + 4, y + 8, 4, 2), color);
            fill(ops, (x + 4, y + 11, 8, 1), color);
        }
        "code" | "module" | "file" | "document" | "source" => {
            line(ops, &[(3.0, 1.5), (9.5, 1.5), (13.0, 5.0), (13.0, 14.5), (3.0, 14.5), (3.0, 1.5)], 1.1);
            line(ops, &[(9.5, 1.5), (9.5, 5.0), (13.0, 5.0)], 1.1);
            for (k, len) in [(0, 5), (1, 6), (2, 4)] {
                fill(ops, (x + 5, y + 7 + k * 2, len, 1), if k == 1 { accent } else { color });
            }
        }
        "debug" | "bug" | "breakpoints" => {
            ops.push(Op::Round { rect: (x + 4, y + 4, 8, 10), radius: 4.0, fill: None, stroke: Some(color), width: 1.2 });
            line(ops, &[(8.0, 6.0), (8.0, 13.0)], 1.0);
            for yy in [6.5, 9.5, 12.5] {
                line(ops, &[(1.5, yy), (4.0, yy)], 1.0);
                line(ops, &[(12.0, yy), (14.5, yy)], 1.0);
            }
            ops.push(Op::Round { rect: (x + 6, y + 1, 4, 4), radius: 2.0, fill: Some(accent), stroke: None, width: 0.0 });
        }
        "database" | "data" => {
            ops.push(Op::Round { rect: (x + 2, y + 1, 12, 5), radius: 2.5, fill: None, stroke: Some(color), width: 1.1 });
            line(ops, &[(2.5, 3.5), (2.5, 12.5)], 1.1);
            line(ops, &[(13.5, 3.5), (13.5, 12.5)], 1.1);
            ops.push(Op::Round { rect: (x + 2, y + 10, 12, 5), radius: 2.5, fill: None, stroke: Some(color), width: 1.1 });
        }
        "chart" | "plot" => {
            fill(ops, (x + 1, y + 14, 14, 1), color);
            fill(ops, (x + 2, y + 8, 3, 6), color);
            fill(ops, (x + 7, y + 3, 3, 11), accent);
            fill(ops, (x + 12, y + 6, 3, 8), color);
        }
        "list" | "tasks" => {
            for k in 0..3 {
                fill(ops, (x + 1, y + 3 + k * 4, 2, 2), accent);
                fill(ops, (x + 5, y + 3 + k * 4, 10, 2), color);
            }
        }
        _ => {}
    }
}

// ----------------------------------------------------------- the ground --

/// The dock manager itself (under its groups): the ground, a splitter
/// under the mouse or held, the auto-hide strips and their tabs.
pub fn manager_ops(m: &Manager, g: &Geometry, t: &Theme, font: &Font) -> Vec<Op> {
    let p = palette(t);
    let mut ops = Vec::new();
    fill(&mut ops, (0, 0, g.size.0, g.size.1), p.ground);
    let hot_split = match (&m.ui.split, &m.ui.hover) {
        (Some(s), _) => Some(s.splitter),
        (None, Some(Part::Manager(geometry::Hit::Splitter(i)))) => Some(*i),
        _ => None,
    };
    if let Some(s) = hot_split.and_then(|i| g.splitters.get(i)) {
        let (x, y, w, h) = s.rect;
        // (a line along the middle of the gutter)
        let r = if s.axis == super::Axis::Row { (x + w / 2 - 1, y, 2, h) } else { (x, y + h / 2 - 1, w, 2) };
        fill(&mut ops, r, p.splitter_hot);
    }
    for (side, r) in &g.strips {
        fill(&mut ops, *r, p.ground);
        // (a hairline toward the layout)
        let edge = match side {
            Side::Left => (r.0 + r.2 - 1, r.1, 1, r.3),
            Side::Right => (r.0, r.1, 1, r.3),
            Side::Top => (r.0, r.1 + r.3 - 1, r.2, 1),
            Side::Bottom => (r.0, r.1, r.2, 1),
        };
        fill(&mut ops, edge, p.border);
    }
    let titles = m.titles();
    for st in &g.strip_tabs {
        let hot = matches!(&m.ui.hover, Some(Part::Manager(geometry::Hit::Strip(p))) if *p == st.pane);
        let open = m.flyout.as_deref() == Some(st.pane.as_str());
        let (x, y, w, h) = st.rect;
        if p.classic {
            fill(&mut ops, st.rect, if hot || open { p.strip_tab_hover } else { p.strip_tab });
            raised(&mut ops, st.rect, t);
        } else {
            ops.push(Op::Round { rect: st.rect, radius: if p.contrast { 0.0 } else { 3.0 }, fill: Some(if hot || open { p.strip_tab_hover } else { p.strip_tab }), stroke: Some(if p.contrast && (hot || open) { p.hot_outline } else { p.border }), width: 1.0 });
            // (the open one's accent bar on its outer edge)
            if open {
                let bar = match st.side {
                    Side::Left => (x, y + 2, 2, h - 4),
                    Side::Right => (x + w - 2, y + 2, 2, h - 4),
                    Side::Top => (x + 2, y, w - 4, 2),
                    Side::Bottom => (x + 2, y + h - 2, w - 4, 2),
                };
                fill(&mut ops, bar, p.accent);
            }
        }
        let title = titles.title(&st.pane);
        let ic = titles.icon(&st.pane);
        let vertical = matches!(st.side, Side::Left | Side::Right);
        let ink = if hot || open { p.title_active } else { p.strip_text };
        if vertical {
            let mut ty = y + 8;
            if !ic.is_empty() {
                icon(&mut ops, &ic, x + (w - 16) / 2, y + 6, ink, p.accent);
                ty += 22;
            }
            let rect = (x, ty, w, (y + h - 8 - ty).max(0));
            text(&mut ops, rect, &title, font, ink, Place::Center, 270);
        } else {
            let mut tx = x + 8;
            if !ic.is_empty() {
                icon(&mut ops, &ic, x + 6, y + (h - 16) / 2, ink, p.accent);
                tx += 22;
            }
            text(&mut ops, (tx, y, (x + w - tx - 4).max(0), h), &title, font, ink, Place::Left, 0);
        }
    }
    ops
}

/// Over the groups: the open flyout's shadow, a drag's compass and the
/// outline of where the pane would go (or the keyboard's move's).
pub fn overlay_ops(m: &Manager, g: &Geometry, t: &Theme, font: &Font) -> Vec<Op> {
    let p = palette(t);
    let mut ops = Vec::new();
    if let Some(f) = &g.flyout {
        let (x, y, w, h) = f.rect;
        // (a soft edge on its open sides)
        for k in 1..=3i64 {
            let c = mix(p.shadow, p.ground, 0.55 + 0.15 * k as f64);
            fill(&mut ops, (x + w + k - 1, y + k, 1, h - k), c);
            fill(&mut ops, (x + k, y + h + k - 1, w, 1), c);
        }
    }
    let (pane, target, at, area) = match (&m.ui.drag, &m.ui.moving) {
        (Some(d), _) if d.started => (d.pane.clone(), d.target.clone(), Some(d.at), None),
        (_, Some(mv)) => (mv.pane.clone(), m.move_target(), None, Some(mv.area)),
        _ => return ops,
    };
    let extent = m.pane(&pane).map_or(super::DEFAULT_SIDE, |i| i.extent);
    if let Some(r) = target.as_ref().and_then(|tg| g.preview(tg, extent)) {
        let (x, y, w, h) = r;
        fill(&mut ops, (x + 2, y + 2, w - 4, h - 4), p.preview);
        frame(&mut ops, r, p.guide_hot, 2);
        let label = describe_target(target.as_ref().unwrap(), m);
        let lw = crate::objects::text::text_size(&label, font).0 + 20;
        if w > lw && h > 30 {
            // (near its top, clear of the compass in its middle)
            let lr = (x + (w - lw) / 2, y + 12.min(h / 4), lw, 24);
            ops.push(Op::Round { rect: lr, radius: if p.contrast { 0.0 } else { 4.0 }, fill: Some(p.guide_hot), stroke: None, width: 0.0 });
            text(&mut ops, lr, &label, font, if p.contrast { 0x000000 } else { t.accent_text }, Place::Center, 0);
        }
    }
    // (the compass: the cross's backing plate first)
    let guides = g.compass(&pane, at, area);
    let cross: Vec<&(Guide, Rect, Target)> = guides.iter().filter(|(gd, _, _)| !matches!(gd, Guide::Outer(_))).collect();
    if let (Some(first), true) = (cross.first(), cross.len() > 1) {
        let (mut x0, mut y0, mut x1, mut y1) = (first.1 .0, first.1 .1, first.1 .0 + GUIDE, first.1 .1 + GUIDE);
        for (_, r, _) in &cross {
            x0 = x0.min(r.0);
            y0 = y0.min(r.1);
            x1 = x1.max(r.0 + r.2);
            y1 = y1.max(r.1 + r.3);
        }
        let mid = ((x0 + x1) / 2, (y0 + y1) / 2);
        let s = GUIDE + 4;
        let plate = |ops: &mut Vec<Op>, r: Rect| ops.push(Op::Round { rect: r, radius: if p.contrast { 0.0 } else { 8.0 }, fill: Some(p.ground), stroke: Some(p.guide_border), width: 1.0 });
        plate(&mut ops, (mid.0 - GUIDE / 2 - 4, y0 - 4, GUIDE + 8, y1 - y0 + 8));
        plate(&mut ops, (x0 - 4, mid.1 - GUIDE / 2 - 4, x1 - x0 + 8, GUIDE + 8));
        // (the plates' seam)
        fill(&mut ops, (mid.0 - GUIDE / 2 - 3, mid.1 - GUIDE / 2 - 3, GUIDE + 6, GUIDE + 6), p.ground);
        let _ = s;
    }
    for (gd, r, tg) in &guides {
        let hot = target.as_ref() == Some(tg);
        guide(&mut ops, *gd, *r, hot, &p, t);
    }
    ops
}

/// What a target does, in words (the preview's label).
pub fn describe_target(target: &Target, m: &Manager) -> String {
    let titles = m.titles();
    let what = |a: &Anchor| match a {
        Anchor::Documents => "Documents".to_string(),
        // (a group by the pane it shows)
        Anchor::Pane(p) => match m.layout.find(p) {
            Some(super::Where::Docked(path, _)) => match m.layout.root.at(&path) {
                Some(super::Node::Tabs { panes, active }) => titles.title(&panes[*active]),
                _ => titles.title(p),
            },
            _ => titles.title(p),
        },
    };
    match target {
        Target::Edge(s) => format!("Dock {}", s.name()),
        Target::Beside(a, s) => format!("{} of {}", match s {
            Side::Left => "Left",
            Side::Right => "Right",
            Side::Top => "Above",
            Side::Bottom => "Below",
        }, what(a)),
        Target::Into(Anchor::Documents) => "As a document".to_string(),
        Target::Into(a) => format!("Tab with {}", what(a)),
        Target::Float(_) => "Float".to_string(),
        Target::AutoHide(s) => format!("Auto-hide {}", s.name()),
    }
}

/// A compass button: a plate with a little window showing where the pane
/// goes (its side filled), the centre a window with a tab.
fn guide(ops: &mut Vec<Op>, gd: Guide, r: Rect, hot: bool, p: &Palette, t: &Theme) {
    let (x, y, w, h) = r;
    let face = if hot { p.guide_hot } else { p.guide_face };
    let ink = if hot { if p.contrast { 0x000000 } else { t.accent_text } } else { p.guide_border };
    let fillc = if hot { ink } else { p.guide_hot };
    if p.classic {
        fill(ops, r, face);
        raised(ops, r, t);
    } else {
        ops.push(Op::Round { rect: r, radius: if p.contrast { 0.0 } else { 6.0 }, fill: Some(face), stroke: Some(if hot { p.guide_hot } else { p.guide_border }), width: 1.0 });
    }
    // (the little window)
    let win = (x + 7, y + 8, w - 14, h - 16);
    frame(ops, win, ink, 1);
    fill(ops, (win.0, win.1, win.2, 2), ink);
    let (wx, wy, ww, wh) = (win.0 + 1, win.1 + 2, win.2 - 2, win.3 - 3);
    let side_fill = |ops: &mut Vec<Op>, s: Side| {
        let part = match s {
            Side::Left => (wx, wy, ww / 2 - 1, wh),
            Side::Right => (wx + ww - ww / 2 + 1, wy, ww / 2 - 1, wh),
            Side::Top => (wx, wy, ww, wh / 2 - 1),
            Side::Bottom => (wx, wy + wh - wh / 2 + 1, ww, wh / 2 - 1),
        };
        fill(ops, part, fillc);
    };
    match gd {
        Guide::Center => {
            fill(ops, (wx + 1, wy + 1, ww - 2, wh - 2), fillc);
        }
        Guide::Arm(s) => side_fill(ops, s),
        Guide::Outer(s) => {
            side_fill(ops, s);
            // (an arrow toward the edge)
            let (cx, cy) = (x as f64 + w as f64 / 2.0, y as f64 + h as f64 / 2.0);
            let pts = match s {
                Side::Left => [(x as f64 + 2.0, cy), (x as f64 + 6.0, cy - 4.0), (x as f64 + 6.0, cy + 4.0)],
                Side::Right => [(x as f64 + w as f64 - 2.0, cy), (x as f64 + w as f64 - 6.0, cy - 4.0), (x as f64 + w as f64 - 6.0, cy + 4.0)],
                Side::Top => [(cx, y as f64 + 2.0), (cx - 4.0, y as f64 + 6.0), (cx + 4.0, y as f64 + 6.0)],
                Side::Bottom => [(cx, y as f64 + h as f64 - 2.0), (cx - 4.0, y as f64 + h as f64 - 6.0), (cx + 4.0, y as f64 + h as f64 - 6.0)],
            };
            ops.push(Op::Arrow { points: pts, color: ink });
        }
    }
}

// --------------------------------------------------------------- groups --

/// A group (its own coordinates): the header with its title or tabs and
/// buttons, the border, the body behind its pane. `active`: it holds the
/// active pane.
pub fn group_ops(m: &Manager, gr: &Group, t: &Theme, font: &Font, active: bool) -> Vec<Op> {
    let p = palette(t);
    let mut ops = Vec::new();
    let (w, h) = (gr.rect.2, gr.rect.3);
    let hover = match &m.ui.hover {
        Some(Part::Group(s, hit)) if *s == gr.slot => Some(*hit),
        _ => None,
    };
    let pressed = match &m.ui.pressed {
        Some(Part::Group(s, hit)) if *s == gr.slot => Some(*hit),
        _ => None,
    };
    fill(&mut ops, (0, 0, w, h), p.body);
    let titles = m.titles();
    let head = if active { p.header_active } else { p.header };
    let ink_title = if active { p.title_active } else { p.title };
    // ---- the header
    if p.classic {
        fill(&mut ops, (0, 0, w, HEADER), p.ground);
        fill(&mut ops, (1, 1, w - 2, HEADER - 2), head);
    } else {
        fill(&mut ops, (0, 0, w, HEADER), head);
        fill(&mut ops, (0, HEADER - 1, w, 1), p.border);
        if active {
            fill(&mut ops, (0, 0, w, if p.contrast { 3 } else { 2 }), p.accent);
        }
    }
    if gr.single() {
        let pane = &gr.panes[0];
        let mut x = 8;
        let ic = titles.icon(pane);
        if !ic.is_empty() {
            icon(&mut ops, &ic, x, (HEADER - 16) / 2, ink_title, p.accent);
            x += 22;
        }
        let room = gr.tabs[0].rect.2 - x - 4;
        let title = fitted(&titles.title(pane), &bold(font), room);
        text(&mut ops, (x, 0, room.max(0), HEADER), &title, &bold(font), ink_title, Place::Left, 0);
    } else {
        for (i, tab) in gr.tabs.iter().enumerate() {
            let shown = i == gr.active;
            let (x, _, tw, _) = tab.rect;
            let hot = hover == Some(GroupHit::Tab(i));
            if p.classic {
                if shown {
                    // (a raised tab rising out of the body)
                    let r = (x, 3, tw, HEADER - 3);
                    fill(&mut ops, r, t.face);
                    ops.push(Op::Edge { rect: (x, 3, tw, HEADER), light: vec![t.light], dark: vec![t.dark_shadow, t.shadow] });
                } else if hot {
                    fill(&mut ops, (x + 1, 4, tw - 2, HEADER - 7), p.tab_hover);
                }
            } else if hot && !shown {
                ops.push(Op::Round { rect: (x + 2, 3, tw - 4, HEADER - 7), radius: if p.contrast { 0.0 } else { 4.0 }, fill: Some(p.tab_hover), stroke: if p.contrast { Some(p.hot_outline) } else { None }, width: 1.0 });
            }
            let ink = if p.classic {
                if shown {
                    t.text
                } else {
                    p.tab_text
                }
            } else if shown {
                ink_title
            } else if hot {
                p.title_active
            } else {
                p.tab_text
            };
            let mut tx = x + 10;
            let ic = titles.icon(&tab.pane);
            if !ic.is_empty() {
                icon(&mut ops, &ic, tx, (HEADER - 16) / 2, ink, p.accent);
                tx += 22;
            }
            let room = x + tw - tx - 8;
            let f = if shown { bold(font) } else { font.clone() };
            let title = fitted(&titles.title(&tab.pane), &f, room);
            text(&mut ops, (tx, 0, room.max(0), HEADER), &title, &f, ink, Place::Left, 0);
            if shown && !p.classic {
                let thick = if p.contrast { 3 } else { 2 };
                fill(&mut ops, (x + 4, HEADER - thick - 1, tw - 8, thick), if active { p.accent } else { p.underline });
            }
        }
    }
    // ---- its buttons
    for (b, r) in &gr.buttons {
        let hot = hover == Some(GroupHit::Button(*b));
        let down = pressed == Some(GroupHit::Button(*b)) && hot;
        let ink = if p.classic {
            t.text
        } else if hot || active {
            ink_title
        } else {
            p.glyph
        };
        if p.classic {
            fill(&mut ops, *r, t.face);
            if down {
                ops.push(Op::Edge { rect: *r, light: vec![t.dark_shadow, t.shadow], dark: vec![t.light] });
            } else {
                ops.push(Op::Edge { rect: *r, light: vec![t.light], dark: vec![t.dark_shadow, t.shadow] });
            }
        } else if hot {
            ops.push(Op::Round { rect: *r, radius: if p.contrast { 0.0 } else { 4.0 }, fill: Some(if down { p.button_pressed } else { p.button_hover }), stroke: if p.contrast { Some(p.hot_outline) } else { None }, width: 1.0 });
        }
        glyph(&mut ops, *b, *r, ink, gr.slot != Slot::Flyout);
    }
    // ---- the border
    if p.classic {
        frame(&mut ops, (0, HEADER, w, h - HEADER), t.shadow, 1);
    } else {
        frame(&mut ops, (0, 0, w, h), if p.contrast && active { p.accent } else { p.border }, 1);
        if active && !p.contrast {
            fill(&mut ops, (0, 0, w, 2), p.accent);
        }
    }
    ops
}

// ------------------------------------------------------------ documents --

/// The document area (its own coordinates): the workspace behind MDI
/// windows, or the tabs (tabbed mode).
pub fn documents_ops(m: &Manager, d: &Documents, t: &Theme, font: &Font) -> Vec<Op> {
    let p = palette(t);
    let mut ops = Vec::new();
    let (w, h) = (d.rect.2, d.rect.3);
    if d.tabs.is_empty() {
        fill(&mut ops, (0, 0, w, h), p.workspace);
        if p.contrast {
            frame(&mut ops, (0, 0, w, h), p.border, 1);
        }
        return ops;
    }
    let hover = match &m.ui.hover {
        Some(Part::Doc(hit)) => Some(*hit),
        _ => None,
    };
    let titles = m.titles();
    let strip = geometry::DOC_TABS;
    fill(&mut ops, (0, 0, w, strip), p.doc_strip);
    fill(&mut ops, (0, strip, w, h - strip), p.body);
    // (the strip's bottom line, under the tabs but the shown one)
    fill(&mut ops, (0, strip - 1, w, 1), p.border);
    let active = m.layout.active_document;
    for (i, tab) in d.tabs.iter().enumerate() {
        let shown = active == Some(i);
        let (x, y, tw, th) = tab.rect;
        let hot = matches!(hover, Some(DocHit::Tab(k)) | Some(DocHit::Close(k)) if k == i);
        if p.classic {
            if shown {
                fill(&mut ops, (x, y + 2, tw, th - 2), t.face);
                ops.push(Op::Edge { rect: (x, y + 2, tw, th + 1), light: vec![t.light], dark: vec![t.dark_shadow, t.shadow] });
            } else {
                fill(&mut ops, (x + tw, y + 7, 1, th - 13), t.shadow);
                if hot {
                    fill(&mut ops, (x, y + 4, tw, th - 5), p.doc_tab_hover);
                }
            }
        } else if shown {
            fill(&mut ops, (x, y, tw, th), p.doc_tab);
            frame(&mut ops, (x, y, tw, th + 1), p.border, 1);
            fill(&mut ops, (x + 1, y + th - 1, tw - 2, 1), p.doc_tab);
            fill(&mut ops, (x, y, tw, if p.contrast { 3 } else { 2 }), p.accent);
        } else {
            if hot {
                fill(&mut ops, (x, y, tw, th - 1), p.doc_tab_hover);
            }
            // (a short divider between unshown tabs)
            fill(&mut ops, (x + tw, y + 8, 1, th - 16), p.border);
        }
        // (classic: black on the face, the unshown ones grey)
        let ink = if p.classic {
            if shown || hot {
                t.text
            } else {
                t.gray_text
            }
        } else if shown || hot {
            p.title_active
        } else {
            p.tab_text
        };
        let mut tx = x + 12;
        let ic = titles.icon(&tab.pane);
        if !ic.is_empty() {
            icon(&mut ops, &ic, tx, y + (th - 16) / 2, ink, p.accent);
            tx += 22;
        }
        let close = tab.close.unwrap_or((x + tw, y, 0, 0));
        let room = close.0 - tx - 4;
        let title = fitted(&titles.title(&tab.pane), font, room);
        text(&mut ops, (tx, y, room.max(0), th), &title, font, ink, Place::Left, 0);
        // (the close button: shown on the shown tab and under the mouse)
        if shown || hot {
            let over = hover == Some(DocHit::Close(i));
            if over && !p.classic {
                ops.push(Op::Round { rect: close, radius: if p.contrast { 0.0 } else { 4.0 }, fill: Some(p.button_hover), stroke: if p.contrast { Some(p.hot_outline) } else { None }, width: 1.0 });
            }
            glyph(&mut ops, Button::Close, close, if over { p.title_active } else { p.glyph }, true);
        }
    }
    ops
}
