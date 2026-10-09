//! An inspector drawn: Xcode's and Xojo's inspectors in the fluent themes
//! (soft rows, a hairline between the columns, swatches and check boxes
//! drawn crisp), a tidy Delphi 7 object inspector in the classic one (the
//! selected name in the highlight, ± headings, grooves between the rows) —
//! every colour from the theme's look (`common::look`).

use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::{Place, Rect};
use rapidr_value::objects::text::text_size;
use rapidr_value::panels::inspector::values::{self, Kind};
use rapidr_value::panels::inspector::{type_label, Hover, Inspector, Row, RowKind};

use super::super::common::{self, mix, Look};
use super::geo::{self, Geo, Part};
use crate::components::tree::expander;
use crate::paint::Painter;

/// What drawing needs besides the model.
pub struct Ctx<'a> {
    pub id: &'a str,
    pub l: &'a Look,
    pub font: &'a Font,
    pub focused: bool,
    /// The row being edited (its key) and the search box being typed in.
    pub editing: Option<&'a str>,
    pub searching: bool,
    pub enabled: bool,
    /// EmptyText: what the footer says while nothing is inspected.
    pub empty: &'a str,
}

/// `font` in bold.
pub fn bold(font: &Font) -> Font {
    Font { styles: font.styles | 1, ..font.clone() }
}

/// A colour constant's or BGR value's screen colour (RGB): the system's
/// colours as the theme has them.
pub fn swatch_rgb(p: &Painter, v: i64) -> u32 {
    if values::is_system_color(v) {
        return p.theme().system_color(v as u32 & 0xFF);
    }
    let bgr = v as u32 & 0xFF_FFFF;
    ((bgr & 0xFF) << 16) | (bgr & 0xFF00) | (bgr >> 16)
}

/// The subject's header: its type's icon, its name, its type (a
/// multi-selection: how many; nothing: "No selection").
pub fn header(p: &mut Painter, g: &Geo, m: &Inspector, c: &Ctx) {
    let (x, y, w, h) = g.header;
    let l = c.l;
    p.fill(g.header, l.chrome);
    let objects = &m.snap.objects;
    let (title, kind) = match objects.len() {
        0 if m.snap.props.is_empty() => ("No selection".to_string(), String::new()),
        0 => ("Properties".to_string(), String::new()),
        1 => (objects[0].0.clone(), objects[0].1.clone()),
        n => (format!("{n} components"), if m.snap.type_name.is_empty() { "mixed types".into() } else { m.snap.type_name.clone() }),
    };
    // (RapidR's name, RapidQ's as a note: R-NAMES)
    let (long, short) = if objects.is_empty() || kind == "mixed types" { (kind.clone(), kind.clone()) } else { (type_label(&kind, false), type_label(&kind, true)) };
    if l.classic {
        // (Delphi's object selector: a sunken white box, "Button1: RButton")
        let r = (x + 4, y + 4, w - 8, h - 8);
        p.fill(r, l.field);
        p.sunken_edge(r);
        let text = if short.is_empty() { title } else { format!("{title}: {short}") };
        let b = bold(c.font);
        p.text((r.0 + 6, r.1, r.2 - 8, r.3), &common::elide(&text, &b, r.2 - 8), &b, l.text, Place::Left);
        return;
    }
    let icon = if objects.len() == 1 { kind.clone() } else { "select-all".to_string() };
    let mut tx = x + 10;
    if !objects.is_empty() && common::icon(p, &icon, x + 10, y + (h - 16) / 2, 16, None, !c.enabled) {
        tx = x + 32;
    }
    let b = bold(c.font);
    let (tw, _) = text_size(&title, &b);
    let room = w - (tx - x) - 8;
    p.text((tx, y, room, h), &common::elide(&title, &b, room), &b, if objects.is_empty() { l.dim } else { l.text }, Place::Left);
    if !kind.is_empty() && tw + 8 < room {
        let kx = tx + tw + 8;
        let space = x + w - kx - 6;
        let shown = if text_size(&long, c.font).0 <= space { long } else { short };
        p.text((kx, y, space, h), &common::elide(&shown, c.font, space), c.font, l.dim, Place::Left);
    }
    common::hline(p, x, y + h - 1, w, l.line);
}

/// The search strip: the box (with the editor over it while typing) and
/// the view's button (categories / A–Z).
pub fn strip(p: &mut Painter, g: &Geo, m: &Inspector, c: &Ctx) -> Rect {
    let l = c.l;
    p.fill(g.strip, l.chrome);
    let shown = if c.searching { "" } else { m.filter.as_str() };
    let placeholder = if c.searching { "" } else if m.on_events() { "Search events" } else { "Search properties" };
    let area = common::search_box(p, g.search, l, shown, placeholder, c.searching && c.focused, c.font);
    // (the view's button: A–Z lit when on)
    let b = g.view_button;
    let hot = m.ui.hover == Some(Hover::ViewButton);
    let on = m.alphabetic;
    if l.classic {
        p.fill(b, l.chrome);
        if on {
            p.thin_sunken(b);
        } else if hot {
            p.thin_raised(b);
        }
    } else if on || hot {
        let fill = if on { mix(l.accent, l.chrome, 0.78) } else { l.hover };
        p.round(b, 4.0, Some(fill), if l.contrast { Some(l.border) } else { None }, 1.0);
    }
    let icon = if on { "sort-ascending" } else { "layout" };
    common::icon(p, icon, b.0 + (b.2 - 16) / 2, b.1 + (b.3 - 16) / 2, 16, Some(if on { l.accent } else { l.dim }), m.on_events());
    common::hline(p, g.strip.0, g.strip.1 + g.strip.3, g.strip.2, l.line);
    area
}

/// The description under the rows: the selected row's name and its doc
/// (or why a value was refused).
pub fn footer(p: &mut Painter, r: Rect, m: &Inspector, rows: &[Row], c: &Ctx) {
    let (x, y, w, h) = r;
    let l = c.l;
    p.fill(r, l.chrome);
    common::hline(p, x, y, w, l.line);
    let sel = m.selected_in(rows).map(|i| &rows[i]);
    if let Some((key, why)) = &m.ui.error {
        common::icon(p, "error", x + 8, y + 8, 14, Some(l.error), false);
        let b = bold(c.font);
        p.text((x + 28, y + 4, w - 34, 18), "Not changed", &b, l.error, Place::Left);
        let _ = key;
        wrapped(p, (x + 28, y + 22, w - 34, h - 24), why, c.font, l.text, 2);
        return;
    }
    let Some(row) = sel else {
        let hint = if m.snap.objects.is_empty() && m.snap.props.is_empty() && !c.empty.trim().is_empty() {
            c.empty
        } else if !m.designer.is_empty() && m.snap.objects.is_empty() {
            "Select a component on the form."
        } else if m.snap.objects.is_empty() && m.snap.props.is_empty() {
            "Set Target (a component's name) or Designer to inspect."
        } else {
            "Select a row to see what it does."
        };
        wrapped(p, (x + 8, y + 6, w - 16, h - 8), hint, c.font, l.dim, 2);
        return;
    };
    let name = row.prop_name(&m.snap.props, &m.snap.events);
    let doc = rapidr_value::panels::inspector::row_doc(m, row);
    let b = bold(c.font);
    p.text((x + 8, y + 4, w - 16, 18), &common::elide(&name, &b, w - 16), &b, l.text, Place::Left);
    wrapped(p, (x + 8, y + 22, w - 16, h - 24), &doc, c.font, l.dim, 2);
}

/// `text` in up to `lines` lines across `r` (the last cut with "…").
fn wrapped(p: &mut Painter, r: Rect, text: &str, font: &Font, color: u32, lines: usize) {
    let (x, y, w, _) = r;
    let lh = text_size("Ag", font).1.max(1);
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut k = 0;
    while k < words.len() {
        let next = if cur.is_empty() { words[k].to_string() } else { format!("{cur} {}", words[k]) };
        if text_size(&next, font).0 <= w || cur.is_empty() {
            cur = next;
            k += 1;
            continue;
        }
        out.push(std::mem::take(&mut cur));
        if out.len() == lines {
            break;
        }
    }
    if out.len() < lines && !cur.is_empty() {
        out.push(cur);
    } else if k < words.len() {
        if let Some(last) = out.last_mut() {
            *last = common::elide(&format!("{last} {}", words[k..].join(" ")), font, w);
        }
    }
    for (i, line) in out.iter().enumerate() {
        p.text((x, y + i as i64 * lh, w, lh), line, font, color, Place::Left);
    }
}

/// A check box for a Boolean value.
pub fn check(p: &mut Painter, r: Rect, on: Option<bool>, l: &Look, hot: bool) {
    let (x, y, w, _) = r;
    if l.classic {
        p.fill(r, l.field);
        p.sunken_edge(r);
        match on {
            Some(true) => p.check_glyph(x as f64, y as f64, w as f64, l.text),
            None => p.fill((x + 4, y + 6, w - 8, 1), l.dim),
            _ => {}
        }
        return;
    }
    match on {
        Some(true) => {
            p.round(r, 3.0, Some(l.accent), if l.contrast { Some(l.border) } else { None }, 1.0);
            p.check_glyph(x as f64, y as f64, w as f64, l.accent_text);
        }
        None => {
            p.round(r, 3.0, Some(l.field), Some(l.field_border), 1.0);
            p.fill((x + 3, y + w / 2, w - 6, 1), l.dim);
        }
        _ => p.round(r, 3.0, Some(l.field), Some(if hot { l.field_focus } else { l.field_border }), 1.0),
    }
}

/// A colour's swatch.
pub fn swatch(p: &mut Painter, r: Rect, rgb: u32, l: &Look) {
    if l.classic {
        p.fill(r, rgb);
        p.frame(r, l.text);
    } else {
        p.round(r, 3.0, Some(rgb), Some(mix(l.border, l.text, 0.25)), 1.0);
    }
}

/// A row's button: a drop arrow or "…".
fn button(p: &mut Painter, r: Rect, ellipsis: bool, open: bool, hot: bool, c: &Ctx) {
    let l = c.l;
    let (x, y, w, h) = r;
    if l.classic {
        p.fill(r, l.chrome);
        if open {
            p.thin_sunken(r);
        } else {
            p.button_edge(r);
        }
    } else if hot || open {
        p.round(r, 4.0, Some(l.hover), if l.contrast { Some(l.border) } else { None }, 1.0);
    }
    let ink = if l.classic || hot { l.text } else { l.dim };
    if ellipsis {
        let b = bold(c.font);
        p.text((x, y - 2, w, h), "…", &b, ink, Place::Center);
    } else if l.classic {
        p.classic_arrow((x + 2, y + 2, w - 4, h - 4), crate::paint::Dir::Down, ink, open);
    } else {
        p.chevron((x + w / 2) as f64 + 0.5, (y + h / 2) as f64 + 0.5, 7.0, true, ink);
    }
}

/// Draws row `r` (index `i`) at `rr`; the name column `nw` wide.
#[allow(clippy::too_many_arguments)]
pub fn row(p: &mut Painter, m: &Inspector, r: &Row, i: usize, rr: Rect, nw: i64, selected: bool, c: &Ctx) {
    let l = c.l;
    let (x, y, w, h) = rr;
    let hover = matches!(m.ui.hover, Some(Hover::Row(k) | Hover::Reset(k) | Hover::Button(k) | Hover::Check(k)) if k == i);
    if r.kind == RowKind::Category {
        common::section(p, rr, l, &r.name, r.expanded, Some(r.count), c.font);
        if r.ext {
            let (tw, _) = text_size(&r.name, &bold(c.font));
            let cw = text_size(&r.count.to_string(), c.font).0;
            common::badge(p, x + 22 + tw + 6 + cw + 8, y + h / 2, "R", l, c.font);
        }
        if selected && c.focused && c.editing.is_none() && !c.searching {
            common::row_focus(p, rr, l);
        }
        return;
    }
    match r.kind {
        RowKind::Pins => return pins(p, m, r, rr, selected, c),
        RowKind::Picker => return picker(p, m, r, rr, selected, c),
        _ => {}
    }
    // (the row's ground: fluent's soft selection across it; classic
    // highlights the name only, as Delphi does)
    let focused = c.focused && !c.searching;
    let mut ink = l.text;
    if l.classic {
        if selected {
            let (fill, text) = if focused { (l.selected, l.selected_text) } else { (l.inactive, l.inactive_text) };
            p.fill((x, y, nw, h - 1), fill);
            ink = text;
        }
    } else {
        ink = common::row(p, rr, l, selected, focused, hover && !selected);
    }
    let parts = geo::parts(m, r, rr, nw);
    // (the name: its expander, the search's marks, an extension's badge)
    if let Some((cx, mid)) = parts.expander {
        if l.classic {
            expander(p, cx, mid, 9, r.expanded);
        } else {
            p.chevron(cx as f64 + 0.5, mid as f64 + 0.5, 7.0, r.expanded, if selected { ink } else { l.dim });
        }
    }
    let name_room = nw - (parts.name_x - x) - 4;
    let name_ink = if matches!(r.kind, RowKind::Line(_) | RowKind::AddLine) && !selected { l.dim } else { ink };
    let name_font = c.font.clone();
    if r.kind == RowKind::AddLine {
        common::icon(p, "add", parts.name_x - 16, y + (h - 12) / 2, 12, Some(l.dim), false);
    }
    let badge_room = if r.ext && m.alphabetic { 22 } else { 0 };
    common::marked_text(p, (parts.name_x, y, (name_room - badge_room).max(0), h), &r.name, &r.marks, &name_font, name_ink, if selected && l.classic { ink } else { l.mark });
    if badge_room > 0 {
        let (tw, _) = text_size(&common::elide(&r.name, &name_font, name_room - badge_room), &name_font);
        common::badge(p, parts.name_x + tw + 5, y + h / 2, "R", l, c.font);
    }
    // (the reset glyph: on a changed row under the mouse or selected)
    let shows_reset = (hover || selected) && !r.is_default && !r.mixed && geo::resettable(r) && !m.read_only;
    if shows_reset {
        let hot = m.ui.hover == Some(Hover::Reset(i));
        if !l.classic {
            // (the glyph's ground: the row's, so a long name ends under it)
            let ground = match (selected, focused) {
                (true, true) => l.selected,
                (true, false) => l.inactive,
                _ => l.hover,
            };
            if !(l.contrast && !selected) {
                p.fill((parts.reset.0 - 4, y + 2, parts.reset.2 + 6, h - 4), ground);
            }
            if hot {
                p.round((parts.reset.0 - 2, parts.reset.1 - 1, parts.reset.2 + 4, parts.reset.3 + 2), 4.0, Some(mix(l.accent, l.body, 0.8)), None, 1.0);
            }
        } else if selected {
            p.fill((parts.reset.0 - 2, y, parts.reset.2 + 4, h - 1), if focused { l.selected } else { l.inactive });
        }
        let glyph_ink = if hot { l.accent } else if selected && l.classic { ink } else { l.dim };
        common::icon(p, "undo", parts.reset.0 + 2, parts.reset.1 + 2, 12, Some(glyph_ink), false);
    }
    // (the line between the rows; the columns')
    if l.classic {
        p.fill((x, y + h - 1, w, 1), l.line);
        p.fill((x + nw, y, 1, h), l.border);
    } else {
        p.fill((x + nw, y + h - 1, w - nw, 1), mix(l.line, l.body, 0.45));
        p.fill((x + nw, y, 1, h), mix(l.line, l.body, 0.2));
    }
    value(p, m, r, i, rr, nw, &parts, selected, hover, ink, c);
    if selected && focused && c.editing.is_none() && l.classic {
        p.focus((x + 1, y + 1, nw - 2, h - 3));
    }
}

/// Row `r`'s value cell.
#[allow(clippy::too_many_arguments)]
fn value(p: &mut Painter, m: &Inspector, r: &Row, i: usize, rr: Rect, nw: i64, parts: &geo::RowParts, selected: bool, hover: bool, row_ink: u32, c: &Ctx) {
    let l = c.l;
    let (_, y, _, h) = rr;
    let editing = c.editing == Some(r.key.as_str());
    let kind = geo::row_kind(m, r);
    let error = m.ui.error.as_ref().is_some_and(|(k, _)| *k == r.key);
    let ink = if l.classic { l.text } else if selected { row_ink } else { l.text };
    let changed_font = bold(c.font);
    let (font, color) = match () {
        _ if error => (c.font.clone(), l.error),
        _ if r.mixed => (c.font.clone(), l.dim),
        _ if r.is_default || matches!(r.kind, RowKind::Event) => (c.font.clone(), if r.kind == RowKind::Event || l.contrast { ink } else { l.dim }),
        _ => (if matches!(r.kind, RowKind::Line(_) | RowKind::AddLine) { c.font.clone() } else { changed_font }, ink),
    };
    let right = parts.button.map_or(rr.0 + rr.2 - 4, |b| b.0 - 4);
    if !editing {
        if let Some(cb) = parts.check {
            let on = (!r.mixed).then(|| r.text == "True");
            check(p, cb, on, l, m.ui.hover == Some(Hover::Check(i)));
        }
        if let Some(s) = parts.swatch {
            let v = m.snap.props.get(r.prop).and_then(|pr| match r.kind {
                RowKind::FontPart(k) => pr.parts.get(k).and_then(|q| q.value.clone()),
                _ => pr.value.clone(),
            });
            match v.filter(|_| !r.mixed) {
                Some(v) => swatch(p, s, swatch_rgb(p, v.to_i64()), l),
                None => {
                    p.round(s, 3.0, None, Some(l.border), 1.0);
                }
            }
        }
        let text = if r.mixed { "—".to_string() } else { r.text.clone() };
        let room = (right - parts.text_x).max(0);
        p.text((parts.text_x, y, room, h), &common::elide(&text, &font, room), &font, color, Place::Left);
    }
    // (its button: on the selected row, or under the mouse)
    if let (Some(b), Some(k)) = (parts.button, kind.as_ref()) {
        if (selected || hover) && !m.read_only {
            let ellipsis = k.ellipsis() && !matches!(k, Kind::Color);
            let open = match k {
                Kind::Color => r.expanded,
                _ => m.ui.dropped.as_deref() == Some(r.key.as_str()) && crate::components::combo::is_dropped(c.id),
            };
            button(p, b, ellipsis, open, m.ui.hover == Some(Hover::Button(i)), c);
        }
    }
    let _ = nw;
}

/// The anchors' pin editor: the parent's frame, the control in it, a pin
/// on each side — a solid strut where it's anchored, a faint dashed one
/// where it isn't — and what that does, in words.
fn pins(p: &mut Painter, m: &Inspector, r: &Row, rr: Rect, selected: bool, c: &Ctx) {
    let l = c.l;
    let (x, y, w, h) = rr;
    p.fill(rr, if l.classic { l.body } else { mix(l.chrome, l.body, 0.5) });
    let (parent, control, struts) = geo::pins(rr);
    let mask = m.snap.props.get(r.prop).and_then(|pr| pr.value.as_ref()).map(|v| v.to_i64());
    let on = |k: usize| mask.is_some_and(|mk| mk & (1 << k) != 0);
    // (the parent: a window's frame)
    if l.classic {
        p.fill(parent, l.field);
        p.sunken_edge(parent);
    } else {
        p.round(parent, 6.0, Some(l.field), Some(l.field_border), 1.0);
    }
    // (the control: a button-like block; arrows inside when it stretches)
    let ctl_fill = if l.classic { l.chrome } else { mix(l.accent, l.field, 0.82) };
    if l.classic {
        p.fill(control, ctl_fill);
        p.raised_edge(control);
    } else {
        p.round(control, 4.0, Some(ctl_fill), Some(mix(l.accent, l.field, 0.35)), 1.0);
    }
    let (cx, cy) = (control.0 + control.2 / 2, control.1 + control.3 / 2);
    let arrow_ink = if l.classic { l.dim } else { mix(l.accent, l.field, 0.25) };
    if on(0) && on(2) {
        let (x1, x2) = ((control.0 + 8) as f64, (control.0 + control.2 - 8) as f64);
        let yy = cy as f64 + 0.5;
        p.stroke(&[(x1, yy), (x2, yy)], arrow_ink, 1.0);
        p.stroke(&[(x1 + 3.0, yy - 3.0), (x1, yy), (x1 + 3.0, yy + 3.0)], arrow_ink, 1.0);
        p.stroke(&[(x2 - 3.0, yy - 3.0), (x2, yy), (x2 - 3.0, yy + 3.0)], arrow_ink, 1.0);
    }
    if on(1) && on(3) {
        let (y1, y2) = ((control.1 + 4) as f64, (control.1 + control.3 - 4) as f64);
        let xx = cx as f64 + 0.5;
        p.stroke(&[(xx, y1), (xx, y2)], arrow_ink, 1.0);
        p.stroke(&[(xx - 3.0, y1 + 3.0), (xx, y1), (xx + 3.0, y1 + 3.0)], arrow_ink, 1.0);
        p.stroke(&[(xx - 3.0, y2 - 3.0), (xx, y2), (xx + 3.0, y2 - 3.0)], arrow_ink, 1.0);
    }
    // (the struts: an I-beam where anchored, dashes where it floats)
    for (k, s) in struts.iter().enumerate() {
        let (sx, sy, sw, sh) = *s;
        let hot = m.ui.hover == Some(Hover::Pin(k));
        let anchored = on(k);
        let ink = if anchored { if l.classic { l.selected } else { l.accent } } else if hot { l.text } else { mix(l.dim, l.field, 0.35) };
        if hot && !l.classic {
            p.round((sx, sy, sw, sh), 3.0, Some(l.hover), None, 1.0);
        }
        let horizontal = k % 2 == 0;
        if horizontal {
            let my = sy + sh / 2;
            if anchored {
                p.fill((sx + 2, my - 1, sw - 4, 2), ink);
                p.fill((sx + 2, my - 4, 2, 8), ink);
                p.fill((sx + sw - 4, my - 4, 2, 8), ink);
            } else {
                let mut at = sx + 3;
                while at + 3 <= sx + sw - 3 {
                    p.fill((at, my, 3, 1), ink);
                    at += 6;
                }
            }
        } else {
            let mx = sx + sw / 2;
            if anchored {
                p.fill((mx - 1, sy + 2, 2, sh - 4), ink);
                p.fill((mx - 4, sy + 2, 8, 2), ink);
                p.fill((mx - 4, sy + sh - 4, 8, 2), ink);
            } else {
                let mut at = sy + 3;
                while at + 3 <= sy + sh - 3 {
                    p.fill((mx, at, 1, 3), ink);
                    at += 6;
                }
            }
        }
        // (the keyboard's pin)
        if selected && c.focused && m.ui.pin == k && c.editing.is_none() && !c.searching {
            if l.classic {
                p.focus((sx, sy, sw, sh));
            } else {
                p.ring((sx - 1, sy - 1, sw + 2, sh + 2), 4.0, l.focus, if l.contrast { 2.0 } else { 1.5 });
            }
        }
    }
    // (what the anchors do, in words)
    let words = anchors_words(mask.unwrap_or(3));
    let ty = parent.1 + parent.3 + 2;
    p.text((x + 6, ty, w - 12, (y + h - ty).max(0)), &words, c.font, l.dim, Place::Center);
}

/// What anchors `mask` (akLeft 1, akTop 2, akRight 4, akBottom 8) make a
/// control do as its parent resizes.
pub fn anchors_words(mask: i64) -> String {
    let h = match (mask & 1 != 0, mask & 4 != 0) {
        (true, true) => "stretches across",
        (true, false) => "stays left",
        (false, true) => "follows the right",
        (false, false) => "keeps the middle",
    };
    let v = match (mask & 2 != 0, mask & 8 != 0) {
        (true, true) => "stretches down",
        (true, false) => "stays at the top",
        (false, true) => "follows the bottom",
        (false, false) => "keeps the middle",
    };
    let mut s = format!("{h}, {v}");
    if let Some(f) = s.get_mut(0..1) {
        f.make_ascii_uppercase();
    }
    s
}

/// A colour's picker: the 16 standard colours, the system's, the current
/// one with its spelling (a click there types a value).
fn picker(p: &mut Painter, m: &Inspector, r: &Row, rr: Rect, selected: bool, c: &Ctx) {
    let l = c.l;
    p.fill(rr, if l.classic { l.body } else { mix(l.chrome, l.body, 0.5) });
    let pk = geo::picker(rr);
    let small = Font { size: (c.font.size - 1).max(6), ..c.font.clone() };
    p.text(pk.standard_label, "Standard", &small, l.dim, Place::Left);
    p.text(pk.system_label, "System (follow the theme)", &small, l.dim, Place::Left);
    let current = m.snap.props.get(r.prop).and_then(|pr| match r.key.split('#').next().and_then(|k| k.split_once('.')) {
        Some((_, part)) => pr.parts.iter().find(|q| q.name.eq_ignore_ascii_case(part)).and_then(|q| q.value.clone()),
        None => pr.value.clone(),
    });
    let cur = current.as_ref().map(|v| v.to_i64());
    for (k, s) in pk.swatches.iter().enumerate() {
        let name = geo::swatch_name(k);
        let v = values::color_value(name).unwrap_or(0);
        swatch(p, *s, swatch_rgb(p, v), l);
        let hot = m.ui.hover == Some(Hover::Swatch(k));
        if cur == Some(v) {
            let ring = if l.classic { l.text } else { l.accent };
            p.ring((s.0 - 2, s.1 - 2, s.2 + 4, s.3 + 4), 4.0, ring, 1.5);
        } else if hot {
            p.ring((s.0 - 2, s.1 - 2, s.2 + 4, s.3 + 4), 4.0, l.dim, 1.0);
        }
        if selected && c.focused && m.ui.swatch == k && c.editing.is_none() && !c.searching {
            if l.classic {
                p.focus((s.0 - 3, s.1 - 3, s.2 + 6, s.3 + 6));
            } else {
                p.ring((s.0 - 3, s.1 - 3, s.2 + 6, s.3 + 6), 5.0, l.focus, 1.5);
            }
        }
    }
    // (the current colour, as written; the hot swatch's name while hovered)
    match cur {
        Some(v) => swatch(p, pk.preview, swatch_rgb(p, v), l),
        None => p.round(pk.preview, 3.0, None, Some(l.border), 1.0),
    }
    let f = pk.field;
    if c.editing != Some(r.key.as_str()) {
        if l.classic {
            p.fill(f, l.field);
            p.sunken_edge(f);
        } else {
            p.round(f, 4.0, Some(l.field), Some(l.field_border), 1.0);
        }
        let shown = match m.ui.hover {
            Some(Hover::Swatch(k)) => {
                let name = geo::swatch_name(k);
                format!("{name}  {}", values::color_hex(values::color_value(name).unwrap_or(0)))
            }
            _ => cur.map(|v| match values::color_name(v) {
                Some(n) => format!("{n}  {}", values::color_hex(v)),
                None => values::color_hex(v),
            })
            .unwrap_or_default(),
        };
        p.text((f.0 + 6, f.1, f.2 - 8, f.3), &common::elide(&shown, c.font, f.2 - 8), c.font, l.text, Place::Left);
    }
}

/// The empty list's words.
pub fn empty(p: &mut Painter, r: Rect, m: &Inspector, c: &Ctx) {
    let text = if !m.filter.trim().is_empty() {
        "Nothing matches the search"
    } else if m.on_events() {
        "No events"
    } else {
        "Nothing to inspect"
    };
    p.text((r.0, r.1 + 16, r.2, 20), text, c.font, c.l.dim, Place::Center);
}

/// Whether a part is one the mouse lights.
pub fn hover_of(part: Part, i: usize) -> Hover {
    match part {
        Part::Reset => Hover::Reset(i),
        Part::Button => Hover::Button(i),
        Part::Check => Hover::Check(i),
        Part::Pin(k) => Hover::Pin(k),
        Part::Swatch(k) => Hover::Swatch(k),
        _ => Hover::Row(i),
    }
}
