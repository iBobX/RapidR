//! The code's helpers in the code editor (docs/code-helpers-plan.md E0–E2):
//! the language service's sites fetched when typing pauses, and drawn —
//! a **mark** after a value (a colour's swatch, a picture's thumbnail or a
//! red mark when the file is missing, a font's sample, a sound's ▶, a
//! choice's ▾), a **chip** before an argument (its parameter's name), the
//! caret line's **note** at its end, a debugger's **values** at the ends of
//! the paused routine's lines —, opened (a click on a mark, Ctrl+. — Cmd+.
//! — on a value: the shared value editors, `crate::value_edit`) and written
//! back as one undo step in the code's own form.
//!
//! Marks and chips are blank columns in the row's text (`RowText`), so the
//! caret, selections, clicks and word wrap all count them and nothing
//! overlaps; the caret stops beside the text, never inside a decoration.
//! Screen readers: Ctrl+K H reads the caret's line with its helpers ("orange,
//! 255 130 20"); nothing is announced as helpers appear.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use rapidr_editor::service::{self, HelperKind, HelperSite};
use rapidr_editor::{Buffer, EditKind, Selections};
use rapidr_lang::kinds::FileKind;
use rapidr_value::objects::codehelp::{self, Helpers, Level};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::{Op, Place, Rect};
use rapidr_value::objects::text::text_size;

use super::paint::{blend, legible, Shown};
use super::view::x_of;
use super::{with_view, Ctx};
use crate::components::Cx;
use crate::display::Picture;
use crate::paint::Painter;
use crate::value_edit::list::Item;
use crate::value_edit::{self, ListKind, Owner, Picked, Popover};
use crate::value_view::{RowStyle, Swatch, ValueInfo, ValueRow};

/// Ctrl held this long alone shows every argument's name.
pub const HOLD: std::time::Duration = std::time::Duration::from_millis(450);
/// Decorations drawn at most per frame (a screen of a long file).
const MAX_PER_FRAME: usize = 400;
/// A thumbnail's longest side (pixels) as cached.
const THUMB: usize = 64;

/// The service's sites for the editor's text, when typing paused (with
/// the diagnostics).
pub fn fetch(x: &mut Ctx) {
    if x.c.helpers.level == Level::Off {
        if !x.c.helpers.sites.is_empty() {
            x.c.helpers.set_sites(Vec::new(), None);
        }
        return;
    }
    let file = x.file();
    let sites = service::with(|s| s.helpers(&file)).unwrap_or_default();
    // (the line being typed on keeps its decorations while the caret is there)
    let head = x.c.doc.selections().primary().head;
    let buf = x.c.doc.buffer();
    let typing = x.c.helpers.typing_at.filter(|&t| t <= buf.len_bytes() && buf.line_of(t) == buf.line_of(head)).map(|t| {
        let line = buf.line_of(t);
        (buf.line_start(line), buf.line_end(line))
    });
    x.c.helpers.set_sites(sites, typing);
}

/// The document bytes of line `line` (its break not counted).
fn line_span(x: &Ctx, line: usize) -> (usize, usize) {
    let buf = x.c.doc.buffer();
    (buf.line_start(line), buf.line_end(line))
}

// ---- files: inside the project only; thumbnails cached ----------------------

thread_local! {
    /// (folder, value) → the thumbnail (`None`: not a picture, or not there).
    static THUMBS: RefCell<HashMap<(String, String), Option<Arc<Picture>>>> = RefCell::new(HashMap::new());
    /// (folder, value) → whether the file is there.
    static THERE: RefCell<HashMap<(String, String), bool>> = RefCell::new(HashMap::new());
}

/// The folder files are read from: ProjectFolder, else the code file's.
pub fn folder_of(x: &Ctx) -> String {
    if !x.c.helpers.project_folder.is_empty() {
        return x.c.helpers.project_folder.clone();
    }
    std::path::Path::new(&x.c.file_name).parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Whether the file a site names is there (`None`: it can't be told — a
/// URL, outside the project).
pub fn file_there(h: &Helpers, folder: &str, value: &str) -> Option<bool> {
    if let Some(listed) = h.listed(value) {
        if listed {
            return Some(true);
        }
    }
    let path = codehelp::confined(folder, value)?;
    let key = (folder.to_string(), value.to_string());
    if let Some(v) = THERE.with(|t| t.borrow().get(&key).copied()) {
        return Some(v);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let there = std::path::Path::new(&path).is_file();
    #[cfg(target_arch = "wasm32")]
    let there = rapidr_value::objects::read_file(&path).is_ok();
    THERE.with(|t| {
        let mut t = t.borrow_mut();
        if t.len() > 512 {
            t.clear();
        }
        t.insert(key, there);
    });
    Some(there)
}

/// A picture file's thumbnail (its longest side [`THUMB`] pixels), read
/// only from inside `folder`; cached.
pub fn thumbnail(folder: &str, value: &str) -> Option<Arc<Picture>> {
    let key = (folder.to_string(), value.to_string());
    if let Some(v) = THUMBS.with(|t| t.borrow().get(&key).cloned()) {
        return v;
    }
    let pic = codehelp::confined(folder, value).filter(|p| FileKind::Image.matches(p)).and_then(|path| {
        let bytes = rapidr_value::objects::read_file(&path).ok()?;
        use rapidr_value::objects::codec;
        let (px, alpha) = if codec::is_svg(&bytes) {
            let (px, a) = codec::decode_svg(&bytes, 1.0).ok()?;
            (px, Some(a))
        } else {
            codec::decode_raster(&bytes).ok()?
        };
        Some(Arc::new(shrink(px.width, px.height, &px.pixels, alpha.as_deref())))
    });
    THUMBS.with(|t| {
        let mut t = t.borrow_mut();
        if t.len() > 128 {
            t.clear();
        }
        t.insert(key, pic.clone());
    });
    pic
}

/// A picture made at most [`THUMB`] pixels a side (each pixel the average
/// of the ones it covers); RapidQ's BGR in, RGBA out.
fn shrink(w: usize, h: usize, px: &[u32], alpha: Option<&[u8]>) -> Picture {
    let k = (w.max(h) as f64 / THUMB as f64).max(1.0);
    let (tw, th) = (((w as f64 / k).round() as usize).max(1), ((h as f64 / k).round() as usize).max(1));
    let mut rgba = vec![0u8; tw * th * 4];
    for ty in 0..th {
        for tx in 0..tw {
            let (x0, x1) = ((tx as f64 * k) as usize, (((tx + 1) as f64 * k) as usize).min(w).max((tx as f64 * k) as usize + 1));
            let (y0, y1) = ((ty as f64 * k) as usize, (((ty + 1) as f64 * k) as usize).min(h).max((ty as f64 * k) as usize + 1));
            let mut acc = [0u64; 4];
            let mut n = 0u64;
            for y in y0..y1.min(h) {
                for x in x0..x1.min(w) {
                    let i = y * w + x;
                    let c = px.get(i).copied().unwrap_or(0);
                    let a = alpha.and_then(|a| a.get(i).copied()).unwrap_or(255) as u64;
                    acc[0] += (c & 0xFF) as u64 * a;
                    acc[1] += ((c >> 8) & 0xFF) as u64 * a;
                    acc[2] += ((c >> 16) & 0xFF) as u64 * a;
                    acc[3] += a;
                    n += 1;
                }
            }
            let o = (ty * tw + tx) * 4;
            if acc[3] > 0 {
                rgba[o] = (acc[0] / acc[3]) as u8;
                rgba[o + 1] = (acc[1] / acc[3]) as u8;
                rgba[o + 2] = (acc[2] / acc[3]) as u8;
            }
            rgba[o + 3] = (acc[3] / n.max(1)) as u8;
        }
    }
    Picture { width: tw, height: th, rgba }
}

// ---- drawing ----------------------------------------------------------------

/// A decoration's rectangle (the component's logical pixels) in row `sh`.
fn deco_rect(x: &Ctx, sh: &Shown, disp: usize, cols: usize, origin: (i64, i64)) -> Option<Rect> {
    let layout = x.ui.cache.layout(sh.slot)?;
    let s = x.scale;
    let x0 = (sh.origin.0 + x_of(layout, disp)) / s - origin.0 as f64;
    let x1 = (sh.origin.0 + x_of(layout, disp + cols)) / s - origin.0 as f64;
    Some((x0.round() as i64, sh.top.round() as i64, (x1 - x0).round() as i64, x.ui.lh().round() as i64))
}

/// The marks and chips of the rows in view.
pub fn paint_decos(x: &mut Ctx, p: &mut Painter, shown: &[Shown]) {
    if x.c.helpers.sites.is_empty() || x.c.helpers.level == Level::Off {
        return;
    }
    let origin = p.origin();
    let folder = folder_of(x);
    let sc = x.scheme;
    let t = p.theme();
    let code = Font { name: x.ui.font.name.clone(), size: x.ui.font.size, ..Font::default() };
    let chip_font = Font { size: -((x.ui.metrics.px * 0.82).round() as i64).max(8), ..code.clone() };
    let chip_bg = blend(sc.text, sc.background, if sc.dark { 0.14 } else { 0.07 });
    let chip_ink = legible(blend(sc.text, sc.background, 0.6), chip_bg, sc.text);
    let mut drawn = 0;
    for sh in shown {
        for &(disp, cols, chip, k) in &sh.text.decos {
            if drawn >= MAX_PER_FRAME {
                return;
            }
            drawn += 1;
            let Some(d) = sh.decos.get(k).copied() else { continue };
            let Some(site) = x.c.helpers.sites.get(d.site).cloned() else { continue };
            // (relative to the painter's origin: the component's)
            let Some(r) = deco_rect(x, sh, disp, cols, origin) else { continue };
            if chip {
                let tw = text_size(&format!("{}:", site.label), &chip_font).0;
                let pill = (r.0, r.1 + 3, (tw + 8).min(r.2 - 2).max(6), r.3 - 6);
                p.round(pill, 4.0, Some(chip_bg), None, 1.0);
                p.text((pill.0 + 4, pill.1, pill.2, pill.3), &format!("{}:", site.label), &chip_font, chip_ink, Place::Left);
                continue;
            }
            paint_mark(p, &site, r, &folder, &x.c.helpers, sc, t, &code);
        }
    }
}

/// One mark in `r` (its columns: a square in the middle of them).
#[allow(clippy::too_many_arguments)]
fn paint_mark(p: &mut Painter, site: &HelperSite, r: Rect, folder: &str, helpers: &Helpers, sc: &rapidr_value::code_scheme::Scheme, t: &rapidr_value::theme::Theme, code: &Font) {
    let side = (r.3 - 6).min(r.2 - 2).max(6);
    let sq = (r.0 + (r.2 - side) / 2, r.1 + (r.3 - side) / 2, side, side);
    match site.kind {
        HelperKind::Color => {
            let Ok(v) = site.value.parse::<i64>() else { return };
            let Some(s) = Swatch::of(v) else { return };
            crate::value_view::swatch(p, sq, s.rgb(t), true);
        }
        HelperKind::Font => {
            let f = Font { name: site.value.clone(), ..code.clone() };
            p.clipped(r, |p| p.text(r, "Aa", &f, sc.text, Place::Center));
        }
        HelperKind::File => {
            let there = file_there(helpers, folder, &site.value);
            if there == Some(false) {
                // (a red mark: the file isn't in the project)
                p.icon("actions/error", sq, Some(legible(sc.error, sc.background, sc.text)), false);
                return;
            }
            match site.label.as_str() {
                "image" => match thumbnail(folder, &site.value) {
                    Some(pic) => {
                        let fit = value_edit::fit_into(sq, pic.width as i64, pic.height as i64);
                        p.picture(&format!("code-thumb/{folder}/{}", site.value), 1, pic, fit);
                        p.frame((fit.0 - 1, fit.1 - 1, fit.2 + 2, fit.3 + 2), blend(sc.text, sc.background, 0.35));
                    }
                    None => {
                        p.icon("files/image", sq, Some(blend(sc.text, sc.background, 0.7)), false);
                    }
                },
                "sound" => {
                    p.icon("files/audio", sq, Some(blend(sc.text, sc.background, 0.75)), false);
                }
                _ => {
                    p.icon("files/file", sq, Some(blend(sc.text, sc.background, 0.7)), false);
                }
            }
        }
        HelperKind::Choice | HelperKind::Bool | HelperKind::Component | HelperKind::Handler => {
            p.icon("glyphs/chevron-down", sq, Some(blend(sc.text, sc.background, 0.6)), false);
        }
        _ => {}
    }
}

/// At the ends of the rows in view: the caret line's note, and a paused
/// program's values on the lines that use them.
pub fn paint_line_ends(x: &mut Ctx, p: &mut Painter, shown: &[Shown], caret_line: usize) {
    if x.c.helpers.level < Level::Helpful {
        return;
    }
    let origin = p.origin();
    let s = x.scale;
    let sc = x.scheme;
    let t = p.theme();
    let font = Font { name: x.ui.font.name.clone(), size: x.ui.font.size, ..Font::default() };
    let note_font = Font { styles: 2, ..font.clone() };
    let lh = x.ui.lh().round() as i64;
    for sh in shown {
        // (a line's last row only)
        let last = shown.iter().rfind(|o| o.row.line == sh.row.line).map_or(sh.index, |o| o.index);
        if sh.index != last {
            continue;
        }
        // (a run-time error's or a pause's own note is there: it has the end)
        if x.c.markers_on(sh.row.line).any(|m| !m.note.is_empty()) {
            continue;
        }
        let Some(layout) = x.ui.cache.layout(sh.slot) else { continue };
        let end_dev = sh.origin.0 + x_of(layout, sh.text.text.len());
        let mut lx = ((end_dev / s) - origin.0 as f64).round() as i64 + (x.ui.metrics.ch * 3.0).round() as i64;
        let top = sh.top.round() as i64;
        let line_text = x.c.doc.line(sh.row.line).into_owned();
        let values: Vec<(String, String, String)> = x.c.helpers.values_on(sh.row.line, &line_text).into_iter().map(|v| (v.name.clone(), v.value.clone(), v.kind.clone())).collect();
        if !values.is_empty() {
            let bg = blend(sc.info, sc.background, if sc.dark { 0.18 } else { 0.10 });
            let style = RowStyle::new(&font);
            let rows: Vec<ValueRow> = values.iter().map(|(n, v, k)| ValueRow::new(&ValueInfo { name: n, kind: k, text: v, ..ValueInfo::default() })).collect();
            let icon = (lh - 6).clamp(8, 16);
            let total: i64 = rows.iter().map(|r| r.width(&font, icon) + 14).sum();
            p.round((lx - 6, top + 2, total + 4, lh - 4), 4.0, Some(bg), None, 1.0);
            let _ = t;
            for r in &rows {
                let w = r.paint(p, (lx, top, r.width(&font, icon) + 4, lh), &style);
                lx += w + 14;
            }
            continue;
        }
        if sh.row.line == caret_line {
            let (a, b) = line_span(x, sh.row.line);
            if let Some(note) = x.c.helpers.note(a, b) {
                let ink = legible(blend(sc.text, sc.background, 0.55), sc.background, sc.text);
                let (tw, th) = text_size(note, &note_font);
                p.text((lx, top + (lh - th) / 2, tw + 4, th), note, &note_font, ink, Place::TopLeft);
            }
        }
    }
    let _ = Op::ClipPop;
}

// ---- the mouse, the keys ----------------------------------------------------

/// The site of the mark under component point (`mx`, `my`), if any.
pub fn mark_at(x: &Ctx, mx: f64, my: f64) -> Option<usize> {
    let s = x.scale;
    for sh in &x.ui.shown {
        let lh = x.ui.lh();
        if my < sh.top || my >= sh.top + lh {
            continue;
        }
        let layout = x.ui.cache.layout(sh.slot)?;
        for &(disp, cols, chip, k) in &sh.text.decos {
            if chip {
                continue;
            }
            // (the component's logical x of the mark's columns: its origin
            // is where the rows' layouts start, absolute — see layout_rows)
            let x0 = (sh.origin.0 + x_of(layout, disp)) / s - x.abs.0 as f64;
            let x1 = (sh.origin.0 + x_of(layout, disp + cols)) / s - x.abs.0 as f64;
            if mx >= x0 - 1.0 && mx < x1 + 1.0 {
                return sh.decos.get(k).map(|d| d.site);
            }
        }
    }
    None
}

/// Ctrl+. (Cmd+.) on a value: its editor (`false`: no helper at the caret).
pub fn open_at_caret(x: &mut Ctx) -> bool {
    let head = x.c.doc.selections().primary().head;
    let line = x.c.doc.buffer().line_of(head);
    let (a, b) = line_span(x, line);
    let Some(i) = x.c.helpers.editable_at(head, a, b) else { return false };
    open(x, i);
    true
}

/// Opens site `i`'s value editor, hanging from its mark (or its value).
pub fn open(x: &mut Ctx, i: usize) {
    let Some(site) = x.c.helpers.sites.get(i).cloned() else { return };
    let anchor = anchor_of(x, &site);
    let owner = Owner::Code(site.clone());
    let pop = match site.kind {
        HelperKind::Color => {
            let v = site.value.parse::<i64>().unwrap_or(0);
            let project = project_colors(&x.c.helpers);
            Popover::color(owner, v, &project, anchor)
        }
        HelperKind::Choice => {
            let picked: Vec<&str> = site.value.split('+').map(str::trim).collect();
            let items: Vec<Item> = site.choices.iter().map(|c| {
                let mut it = Item::new(c.clone());
                if let Some((v, _)) = rapidr_lang::constant(c) {
                    it.detail = v.to_string();
                }
                it
            }).collect();
            let sel = site.choices.iter().position(|c| picked.iter().any(|p| p.eq_ignore_ascii_case(c)));
            let kind = if site.flag { ListKind::Flags } else { ListKind::Choice };
            let mut pop = Popover::list(owner, kind, items, sel, anchor);
            if let value_edit::Editor::List { list, .. } = &mut pop.editor {
                if site.flag {
                    list.ticks = Some(site.choices.iter().map(|c| picked.iter().any(|p| p.eq_ignore_ascii_case(c))).collect());
                }
            }
            pop
        }
        HelperKind::Bool => {
            let sel = if site.value.eq_ignore_ascii_case("true") { 0 } else { 1 };
            Popover::list(owner, ListKind::Bool, vec![Item::new("True"), Item::new("False")], Some(sel), anchor)
        }
        HelperKind::Component | HelperKind::Handler => {
            let mut items: Vec<Item> = Vec::new();
            let mut create = None;
            if site.kind == HelperKind::Handler {
                let name = unique_name(&site.hint, &site.choices);
                let mut it = Item::new(format!("Create handler “{name}”"));
                it.action = true;
                it.said = format!("Create handler {name}");
                items.push(it);
                create = Some(name);
            }
            items.extend(site.choices.iter().map(|c| Item::new(c.clone())));
            let sel = items.iter().position(|it| !it.action && it.label.eq_ignore_ascii_case(&site.value)).or(Some(0));
            let mut pop = Popover::list(owner, ListKind::Link, items, sel, anchor);
            pop.title = if site.kind == HelperKind::Handler { "Handler".into() } else { "Link".into() };
            if let value_edit::Editor::List { create: c, .. } = &mut pop.editor {
                *c = create;
            }
            pop
        }
        HelperKind::File => {
            let kind = match site.label.as_str() {
                "image" => FileKind::Image,
                "sound" => FileKind::Sound,
                _ => FileKind::Any,
            };
            let folder = folder_of(x);
            let mut files = project_files(&x.c.helpers, &folder, kind);
            let mut items: Vec<Item> = Vec::new();
            if !site.value.is_empty() && !files.iter().any(|f| f.eq_ignore_ascii_case(&site.value.replace('\\', "/"))) {
                let mut it = Item::new(site.value.clone());
                it.missing = file_there(&x.c.helpers, &folder, &site.value) != Some(true);
                items.push(it);
            }
            files.sort_by_key(|f| f.to_lowercase());
            items.extend(files.into_iter().map(|f| {
                let mut it = Item::new(f.clone());
                if let Some((dir, _)) = f.rsplit_once('/') {
                    it.detail = dir.to_string();
                }
                it
            }));
            let sel = items.iter().position(|it| it.label.replace('\\', "/").eq_ignore_ascii_case(&site.value.replace('\\', "/")));
            let mut pop = Popover::list(owner, ListKind::File(kind), items, sel, anchor);
            if let value_edit::Editor::List { folder: f, .. } = &mut pop.editor {
                *f = folder;
            }
            pop
        }
        HelperKind::Font => {
            let names = x.ts.family_names();
            let mut items: Vec<Item> = names.iter().map(|n| {
                let mut it = Item::new(n.clone());
                it.face = Some(n.clone());
                it
            }).collect();
            if !names.iter().any(|n| n.eq_ignore_ascii_case(&site.value)) && !site.value.is_empty() {
                let mut it = Item::new(site.value.clone());
                it.detail = "not on this computer".into();
                items.insert(0, it);
            }
            let sel = items.iter().position(|it| it.label.eq_ignore_ascii_case(&site.value));
            Popover::list(owner, ListKind::Font, items, sel, anchor)
        }
        HelperKind::ArgName | HelperKind::Note => return,
    };
    let said = pop.said();
    value_edit::open(x.id, pop, (i64::MAX / 4, i64::MAX / 4));
    x.c.helpers.editor_text = said;
    x.c.completion = None;
    x.c.hover = None;
}

/// A new SUB's name not taken (`Button1Click`, else `Button1Click2` …).
fn unique_name(hint: &str, taken: &[String]) -> String {
    let base = if hint.is_empty() { "Handler" } else { hint };
    let mut name = base.to_string();
    let mut n = 2;
    while taken.iter().any(|t| t.eq_ignore_ascii_case(&name)) {
        name = format!("{base}{n}");
        n += 1;
    }
    name
}

/// Where a site's editor hangs from: its mark, else its value (absolute).
fn anchor_of(x: &Ctx, site: &HelperSite) -> Rect {
    let s = x.scale;
    let lh = x.ui.lh().round() as i64;
    let at = site.end;
    if let Some(sh) = super::paint::shown_at(x.ui, x.c, at) {
        let cx = super::paint::caret_x(x.ui, sh, at) / s;
        let top = x.abs.1 + sh.top.round() as i64;
        let start_x = super::paint::shown_at(x.ui, x.c, site.start).map_or(cx, |s0| super::paint::caret_x(x.ui, s0, site.start) / s);
        return (start_x.round() as i64, top, ((cx - start_x).round() as i64).max(8) + (x.ui.metrics.ch * 2.0) as i64, lh);
    }
    (x.abs.0 + x.ui.geo.text.0, x.abs.1 + x.ui.geo.text.1, 40, lh)
}

/// The project's colours (this file's, the most used first): the colour
/// editor's palette.
fn project_colors(h: &Helpers) -> Vec<i64> {
    let mut counts: Vec<(i64, usize)> = Vec::new();
    for s in h.sites.iter().filter(|s| s.kind == HelperKind::Color) {
        if let Ok(v) = s.value.parse::<i64>() {
            match counts.iter_mut().find(|(c, _)| *c == v) {
                Some(e) => e.1 += 1,
                None => counts.push((v, 1)),
            }
        }
    }
    counts.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    counts.into_iter().map(|(c, _)| c).collect()
}

/// The project's files of a kind: ProjectFiles, else (on the desktop) the
/// folder's own, a few levels down.
fn project_files(h: &Helpers, folder: &str, kind: FileKind) -> Vec<String> {
    if !h.project_files.is_empty() {
        return h.project_files.iter().filter(|f| kind.matches(f)).cloned().collect();
    }
    let mut out = Vec::new();
    #[cfg(not(target_arch = "wasm32"))]
    {
        fn walk(root: &std::path::Path, dir: &std::path::Path, depth: usize, kind: FileKind, out: &mut Vec<String>) {
            let Ok(rd) = std::fs::read_dir(dir) else { return };
            let mut entries: Vec<_> = rd.flatten().collect();
            entries.sort_by_key(|e| e.file_name());
            for e in entries {
                if out.len() >= 400 {
                    return;
                }
                let name = e.file_name().to_string_lossy().into_owned();
                if name.starts_with('.') || matches!(name.as_str(), "target" | "node_modules" | "build" | "dist") {
                    continue;
                }
                let p = e.path();
                if p.is_dir() {
                    if depth < 3 {
                        walk(root, &p, depth + 1, kind, out);
                    }
                } else if kind.matches(&name) {
                    if let Ok(rel) = p.strip_prefix(root) {
                        out.push(rel.to_string_lossy().replace('\\', "/"));
                    }
                }
            }
        }
        if !folder.is_empty() {
            let root = std::path::Path::new(folder);
            walk(root, root, 0, kind, &mut out);
        }
    }
    let _ = (folder, kind);
    out
}

/// What a popover picked, written into the code: one undo step, the code's
/// own form (`RGB(…)` stays `RGB(…)`); a new handler's SUB with it.
pub fn apply(cx: &mut Cx, site: &HelperSite, picked: &Picked) {
    with_view(cx, |x| {
        x.c.helpers.editor_text.clear();
        if matches!(picked, Picked::Cancel | Picked::Nothing) || x.c.doc.read_only {
            return;
        }
        let len = x.c.doc.len_bytes();
        if site.end > len || site.start > site.end {
            return;
        }
        // (a handler not written yet, `OnClick =`: its place takes the
        // blanks after the `=` too, so the name follows one space)
        let mut site = site.clone();
        if site.start == site.end {
            let buf = x.c.doc.buffer();
            while let Some(c) = buf.char_after(site.end).filter(|c| *c == ' ' || *c == '\t') {
                site.end += c.len_utf8();
            }
        }
        let site = &site;
        let old = x.c.doc.slice(site.start..site.end).into_owned();
        let mut changes: Vec<rapidr_editor::transaction::Change> = Vec::new();
        let mut caret: Option<usize> = None;
        let new = match (site.kind, picked) {
            (HelperKind::Color, Picked::Color(v)) => Some(codehelp::color_text(&site.form, *v, &old)),
            (HelperKind::Choice, Picked::Choice(v)) => Some(match site.form.as_str() {
                "const" if v.len() == 1 => v[0].clone(),
                f => codehelp::choice_text(f, v),
            }),
            (HelperKind::Bool, Picked::Bool(b)) => Some(codehelp::bool_text(&site.form, *b, &old)),
            (HelperKind::File | HelperKind::Font, Picked::Text(s)) => match codehelp::string_text(s) {
                Some(t) => Some(t),
                None => {
                    x.ui.announce = "That name can't be written in the code (it has a quote)".into();
                    None
                }
            },
            (HelperKind::Component | HelperKind::Handler, Picked::Text(s)) => Some(if x.c.doc.buffer().char_before(site.start) == Some('=') { format!(" {s}") } else { s.clone() }),
            (HelperKind::Handler, Picked::CreateHandler(name)) => {
                let params = if site.label.is_empty() { String::new() } else { format!("({})", site.label) };
                let unit = x.c.doc.indent_unit();
                let head = format!("SUB {name}{params}\n");
                let stub = format!("{head}{unit}\nEND SUB\n\n");
                let at = site.at.min(site.start);
                changes.push(rapidr_editor::transaction::Change::insert(at, stub.clone()));
                caret = Some(at + head.len() + unit.len());
                let sep = if x.c.doc.buffer().char_before(site.start) == Some('=') { " " } else { "" };
                Some(format!("{sep}{name}"))
            }
            _ => None,
        };
        let Some(new) = new else { return };
        if new == old && changes.is_empty() {
            return;
        }
        changes.push(rapidr_editor::transaction::Change::new(site.start..site.end, new));
        changes.sort_by_key(|c| c.range.start);
        let Ok(set) = rapidr_editor::ChangeSet::new(changes, len) else { return };
        let after = match caret {
            Some(c) => Selections::caret(c),
            None => x.c.doc.selections().map(&set),
        };
        if x.c.doc.apply(set, after, EditKind::Command, super::input::now_ms()).is_ok() {
            super::input::edited(x, None);
            // (no typing on this line to wait for: the new sites come whole)
            x.c.helpers.typing_at = None;
            super::input::report_moves(x);
            x.ui.announce = "Changed".into();
        }
    });
}

/// The open editor's words, for HelperEditorText.
pub fn editor_said(cx: &mut Cx, said: &str) {
    with_view(cx, |x| x.c.helpers.editor_text = said.to_string());
}

/// Ctrl+K H: the caret's line read with its helpers (announced once).
pub fn read_line(x: &mut Ctx) {
    let head = x.c.doc.selections().primary().head;
    let line = x.c.doc.buffer().line_of(head);
    let (a, b) = line_span(x, line);
    let text = x.c.doc.line(line).into_owned();
    let folder = folder_of(x);
    let h = &x.c.helpers;
    let said = h.describe_line(a, b, &text, &|s| file_there(h, &folder, &s.value));
    x.ui.announce = said;
}

/// Ctrl pressed (`down`) or let go in the form: held alone a moment, the
/// focused code editor shows every argument's name; let go, they go.
pub fn ctrl(f: &mut crate::tree::FormUi, down: bool) {
    let Some(i) = f.focus else { return };
    if f.nodes[i].type_name != "RCODEEDITOR" {
        return;
    }
    let id = f.nodes[i].id.clone();
    let Some(ui) = f.nodes[i].ui.code.as_mut() else { return };
    if down {
        if ui.ctrl_at.is_none() {
            let at = crate::tick::now() + HOLD;
            ui.ctrl_at = Some(at);
            f.nodes[i].ui.wake = Some(f.nodes[i].ui.wake.map_or(at, |w| w.min(at)));
        }
    } else {
        ui.ctrl_at = None;
        let was = rapidr_value::objects::with_code_mut(&id, |c| {
            let held = std::mem::take(&mut c.helpers.held);
            if held {
                c.helpers.version += 1;
            }
            held
        })
        .unwrap_or(false);
        if was {
            f.dirty = true;
        }
    }
}

/// Another key while Ctrl is down: it was a shortcut, not a hold.
pub fn ctrl_used(x: &mut Ctx) {
    x.ui.ctrl_at = None;
    if std::mem::take(&mut x.c.helpers.held) {
        x.c.helpers.version += 1;
    }
}

/// The view's tick: Ctrl held long enough.
pub fn tick(x: &mut Ctx, at: crate::tick::Instant) {
    if x.ui.ctrl_at.is_some_and(|t| t <= at) {
        x.ui.ctrl_at = None;
        if x.c.helpers.on("names") {
            x.c.helpers.held = true;
            x.c.helpers.version += 1;
        }
    }
}

/// Whether the caret's line has helpers a screen reader can ask for.
pub fn line_has_helpers(x: &Ctx) -> bool {
    let head = x.c.doc.selections().primary().head;
    let line = x.c.doc.buffer().line_of(head);
    let (a, b) = line_span(x, line);
    x.c.helpers.in_range(a, b).any(|i| !matches!(x.c.helpers.sites[i].kind, HelperKind::ArgName | HelperKind::Note))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumbnails_are_small() {
        let px: Vec<u32> = (0..200 * 100).map(|i| if i % 2 == 0 { 0x0000FF } else { 0xFF0000 }).collect();
        let p = shrink(200, 100, &px, None);
        assert_eq!((p.width, p.height), (64, 32));
        // (red and blue averaged: purple, opaque)
        assert!(p.rgba[0] > 50 && p.rgba[2] > 50 && p.rgba[3] == 255, "{:?}", &p.rgba[..4]);
    }

    #[test]
    fn handler_names_are_new() {
        assert_eq!(unique_name("OkClick", &["Other".into()]), "OkClick");
        assert_eq!(unique_name("OkClick", &["okclick".into(), "OkClick2".into()]), "OkClick3");
    }
}
