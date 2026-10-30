//! The eyes: what the program shows, for an AI (RapidR Studio's AI apps,
//! RAI) or the program itself (RWindowCapture) — a picture of each of its
//! windows and one of them all, at the screen's scale or at 1×, and a
//! layout report of its forms as the kernel has laid them out now (each
//! component's name, type, caption and text, place and size, visible and
//! enabled), with what a person would see wrong: text cut off, controls on
//! top of each other, a control past its window's edge.
//!
//! Only the program's own windows, ever: the pictures are the kernel's
//! drawing of the forms in [`Desktop`] — the program's — drawn again by the
//! CPU renderer, never a read of the screen. Nothing here can show another
//! program, the desktop or RapidR Studio (asking for a window the program
//! hasn't got is an error that lists the ones it has).
//!
//! Desktop and web alike: the same [`Desktop`], kernel and renderer.

use rapidr_ui_kernel::frame::{self, Look};
use rapidr_ui_kernel::{FormUi, Store};
use rapidr_value::objects::codec::Pixels;
use serde_json::{json, Value as Json};

use crate::desktop::Desktop;

/// How big a picture is: the window's own scale (the screen's: sharp, as
/// the person sees it) or this many device pixels per logical pixel (1: a
/// program's own coordinates, one pixel each).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Scale {
    Screen,
    Times(f64),
}

impl Scale {
    /// `"screen"` (or nothing) / `"1x"`, `"2"`, `"1.5x"`.
    pub fn parse(s: &str) -> Scale {
        let t = s.trim().trim_end_matches(['x', 'X']).trim();
        match t.parse::<f64>() {
            Ok(n) if n > 0.0 => Scale::Times(n.clamp(0.25, 4.0)),
            _ => Scale::Screen,
        }
    }

    fn of(self, f: &crate::desktop::Form) -> f64 {
        match self {
            Scale::Screen => f.scale,
            Scale::Times(n) => n,
        }
    }
}

/// The program's windows shown now, bottom to top (the front one last).
pub fn windows(desk: &Desktop) -> Vec<String> {
    desk.stacking()
}

/// The window `name` names: a form's name or its window's title (any
/// case); `""`, `"front"` or `"active"`: the front one. An error that lists
/// the windows there are when there's no such one.
pub fn find_window(desk: &Desktop, name: &str) -> Result<String, String> {
    let shown = desk.stacking();
    if shown.is_empty() {
        return Err("The program shows no window now.".into());
    }
    let want = name.trim();
    if want.is_empty() || want.eq_ignore_ascii_case("front") || want.eq_ignore_ascii_case("active") {
        return Ok(shown.last().cloned().unwrap_or_default());
    }
    if let Some(f) = shown.iter().find(|f| f.eq_ignore_ascii_case(want)) {
        return Ok(f.clone());
    }
    if let Some(f) = shown.iter().find(|f| desk.forms.get(*f).is_some_and(|w| w.spec.title.replace('&', "").trim().eq_ignore_ascii_case(want))) {
        return Ok(f.clone());
    }
    Err(format!("No window named \"{want}\" in this program. Its windows now: {}.", window_list(desk)))
}

/// The windows shown, as a person names them: `form1 ("Calculator")`.
pub fn window_list(desk: &Desktop) -> String {
    let shown = desk.stacking();
    if shown.is_empty() {
        return "none".into();
    }
    shown.iter().map(|f| format!("{f} (\"{}\")", desk.forms.get(f).map(|w| w.spec.title.as_str()).unwrap_or(""))).collect::<Vec<_>>().join(", ")
}

/// Runs `f` with the virtual cursor hidden unless `cursor`: a picture of
/// the program shows what the program shows.
fn without_cursor<R>(cursor: bool, f: impl FnOnce() -> R) -> R {
    if cursor {
        return f();
    }
    let kept = rapidr_ui_kernel::virtual_cursor::get();
    rapidr_ui_kernel::virtual_cursor::set(None);
    let r = f();
    rapidr_ui_kernel::virtual_cursor::set(kept);
    r
}

/// Form `form`'s window drawn: its inside (the client area and an
/// in-window menu bar) at `scale`, with its frame and title bar as the
/// kernel draws a window's when `framed` (the web's and the designer's
/// look), the virtual cursor only when `cursor`.
pub fn capture(desk: &mut Desktop, store: &dyn Store, form: &str, scale: Scale, framed: bool, cursor: bool) -> Result<Pixels, String> {
    let key = form.to_lowercase();
    let front = desk.stacking().last().is_some_and(|f| *f == key);
    let Desktop { forms, text, .. } = desk;
    let Some(f) = forms.get_mut(&key).filter(|f| f.shown) else {
        return Err(format!("{form} isn't shown"));
    };
    let s = scale.of(f);
    let was = f.ui.scale;
    let inside = without_cursor(cursor, || {
        // (a code editor's view finishes what its first frame started, as a
        // window's next frames would: the desktop's capture does the same)
        if f.ui.nodes.iter().any(|n| n.type_name == "RCODEEDITOR") {
            for _ in 0..3 {
                f.ui.paint(store, text, s);
                let now = rapidr_ui_kernel::tick::now();
                if f.ui.next_wake().is_none_or(|w| w > now) {
                    break;
                }
                f.ui.tick(store, text, now);
                f.ui.take_events();
            }
        }
        let list = f.ui.paint(store, text, s);
        rapidr_ui_render::cpu::capture(&list, text, &f.ui)
    });
    // (painted at another scale than its window's: the window's next frame
    // is its own again)
    if (was - s).abs() > f64::EPSILON {
        f.ui.scale = was;
        f.ui.dirty = true;
    }
    if !framed || !f.spec.border {
        return Ok(inside);
    }
    let caption = !f.spec.no_caption;
    let look = Look { title: f.spec.title.clone(), active: front, border: true, caption, frame: f.spec.frame, maximized: f.state == 2, icon: frame::icon_of(store, &key) };
    let (iw, ih) = (f.ui.client.0, f.ui.client.1 + f.ui.menu_offset);
    let size = frame::outer((iw, ih), true, caption);
    let list = frame::paint(&look, size, s);
    let mut whole = rapidr_ui_render::cpu::capture(&list, text, &f.ui);
    let (ix, iy) = frame::inset(true, caption);
    blit(&mut whole, &inside, ((ix as f64) * s).round() as usize, ((iy as f64) * s).round() as usize);
    Ok(whole)
}

/// `src` drawn into `dst` with its top left at (x, y).
fn blit(dst: &mut Pixels, src: &Pixels, x: usize, y: usize) {
    for row in 0..src.height {
        let ty = y + row;
        if ty >= dst.height {
            break;
        }
        let n = src.width.min(dst.width.saturating_sub(x));
        let d = ty * dst.width + x;
        let s = row * src.width;
        dst.pixels[d..d + n].copy_from_slice(&src.pixels[s..s + n]);
    }
}

/// Every window shown, side by side from the back one to the front one
/// (each framed when `framed`), on the theme's desktop-like background,
/// 16 logical pixels apart, their tops aligned: one picture of what the
/// program shows. `None` when it shows none.
pub fn capture_all(desk: &mut Desktop, store: &dyn Store, scale: Scale, framed: bool, cursor: bool) -> Result<Pixels, String> {
    let order = desk.stacking();
    if order.is_empty() {
        return Err("The program shows no window now.".into());
    }
    let mut shots = Vec::new();
    let mut gap = 16usize;
    for f in &order {
        let s = desk.forms.get(f).map_or(1.0, |w| scale.of(w));
        gap = (16.0 * s).round() as usize;
        shots.push(capture(desk, store, f, scale, framed, cursor)?);
    }
    if shots.len() == 1 {
        return Ok(shots.pop().unwrap_or(Pixels { width: 1, height: 1, pixels: vec![0] }));
    }
    let width = shots.iter().map(|p| p.width).sum::<usize>() + gap * (shots.len() + 1);
    let height = shots.iter().map(|p| p.height).max().unwrap_or(1) + 2 * gap;
    // (the theme's shadow tone: what a desktop behind windows looks like,
    // as &HBBGGRR)
    let t = rapidr_value::theme::current();
    let bg = rgb_to_bgr(t.shadow);
    let mut all = Pixels { width, height, pixels: vec![bg; width * height] };
    let mut x = gap;
    for p in &shots {
        blit(&mut all, p, x, gap);
        x += p.width + gap;
    }
    Ok(all)
}

fn rgb_to_bgr(c: u32) -> u32 {
    (c & 0xFF) << 16 | (c & 0xFF00) | (c >> 16 & 0xFF)
}

/// A picture as a PNG file's bytes.
pub fn png(px: &Pixels) -> Result<Vec<u8>, String> {
    rapidr_value::objects::codec::encode_png(px)
}

// ------------------------------------------------------------- the layout --

/// The components kinds whose text is what they show (a button's
/// caption, an edit's text) — and the ones a person types into.
fn types_into(ty: &str) -> bool {
    matches!(ty, "REDIT" | "RMEMO" | "RRICHEDIT" | "RCOMBOBOX" | "RSPINEDIT" | "RMASKEDIT" | "RCODEEDITOR")
}

/// Kinds that can't be judged as overlapping others (they hold others, or
/// are drawn under them on purpose).
fn see_through(ty: &str) -> bool {
    matches!(ty, "RLABEL" | "RPANEL" | "RGROUPBOX" | "RSCROLLBOX" | "RTABCONTROL" | "RPAGECONTROL" | "RIMAGE" | "RSHAPE" | "RBEVEL" | "RCANVAS" | "RSPLITTER" | "RSTATUSBAR" | "RTOOLBAR" | "RCOOLBAR" | "RDOCKMANAGER" | "RDOCKGROUP" | "RDOCKDOCS")
}

/// What a component of type `ty` shows as its text (its caption, or an
/// edit's text — a password's as dots, never what was typed).
fn texts(store: &dyn Store, id: &str, ty: &str) -> (String, String) {
    let s = |p: &str| match store.get(id, p) {
        rapidr_value::Value::Null => String::new(),
        v => v.to_string_val(),
    };
    let caption = s("caption").replace('&', "");
    let mut text = if types_into(ty) || ty == "RLISTBOX" { s("text") } else { String::new() };
    let pw = s("passwordchar");
    if !pw.is_empty() && pw != "0" && !text.is_empty() {
        text = "•".repeat(text.chars().count().min(12));
    }
    if text.chars().count() > 200 {
        text = text.chars().take(200).collect::<String>() + "…";
    }
    (caption, text)
}

/// RapidR's name for a kernel type (`RBUTTON` → `RButton`).
pub fn type_name(ty: &str) -> String {
    rapidr_lang::component(ty).map_or_else(|| ty.to_string(), |c| format!("R{}", c.display))
}

/// Whether component `i` (or a parent) is inside something that scrolls
/// its children (a scroll box: past its edge is normal there).
fn in_scroller(ui: &FormUi, i: usize) -> bool {
    let mut cur = ui.nodes[i].parent;
    while let Some(p) = cur {
        if matches!(ui.nodes[p].type_name.as_str(), "RSCROLLBOX" | "RTABCONTROL" | "RPAGECONTROL") {
            return true;
        }
        cur = ui.nodes[p].parent;
    }
    false
}

/// What a person would see wrong in form `form` as laid out now: text cut
/// off, controls on top of each other, a control past the window's edge
/// or past its panel's.
/// Plain sentences, each with the numbers and what to do.
pub fn problems(desk: &mut Desktop, store: &dyn Store, form: &str) -> Vec<String> {
    let Some(f) = desk.forms.get_mut(&form.to_lowercase()) else { return Vec::new() };
    f.ui.sync(store);
    let ui = &f.ui;
    let (cw, ch) = (ui.client.0, ui.client.1 + ui.menu_offset);
    let mut out = Vec::new();
    for (i, n) in ui.nodes.iter().enumerate() {
        if !n.shown || n.abs.2 <= 0 || n.abs.3 <= 0 {
            continue;
        }
        let (x, y, w, h) = n.abs;
        // (past the window's edge)
        if !in_scroller(ui, i) {
            if x >= cw || y >= ch || x + w <= 0 || y + h <= ui.menu_offset {
                out.push(format!("{} ({}) is outside the window (at {x},{y}; the window's inside is {cw} × {ch}): nobody can see it — move it inside, or make the window bigger.", n.id, type_name(&n.type_name)));
                continue;
            } else if x + w > cw + 1 || y + h > ch + 1 || x < -1 {
                out.push(format!("{} ({}) is cut off by the window's edge (it reaches {},{}; the inside is {cw} × {ch}) — make the window bigger (ClientWidth / ClientHeight) or move it.", n.id, type_name(&n.type_name), x + w, y + h));
            }
        }
        // (past its panel's edge: the panel cuts it off — a scroll box
        // scrolls, a page shows its own)
        if let Some(pi) = n.parent {
            let pn = &ui.nodes[pi];
            if matches!(pn.type_name.as_str(), "RPANEL" | "RGROUPBOX") && !in_scroller(ui, i) {
                let (px, py, pw, ph) = pn.abs;
                if x < px - 1 || y < py - 1 || x + w > px + pw + 1 || y + h > py + ph + 1 {
                    out.push(format!("{} ({}) is cut off by {}'s edge (it reaches {},{}; {} ends at {},{}) — make {} bigger or move {} inside it.", n.id, type_name(&n.type_name), pn.id, x + w, y + h, pn.id, px + pw, py + ph, pn.id, n.id));
                }
            }
        }
        // (text that doesn't fit what shows it)
        let (caption, _) = texts(store, &n.id, &n.type_name);
        let pad = match n.type_name.as_str() {
            "RBUTTON" | "RBITBTN" | "RSPEEDBUTTON" | "ROVALBTN" | "RCOOLBTN" => Some(4),
            "RCHECKBOX" | "RRADIOBUTTON" => Some(18),
            "RLABEL" if !rapidr_value::autosize::autosize_on(&store.get(&n.id, "autosize")) && !rapidr_ui_kernel::store::flag(store, &n.id, "wordwrap", false) => Some(0),
            _ => None,
        };
        if let (Some(pad), false) = (pad, caption.is_empty()) {
            let font = store.font(&n.id);
            let need = caption.lines().map(|l| rapidr_value::objects::text::text_size(l, &font).0).max().unwrap_or(0) + pad;
            if need > w + 1 {
                out.push(format!("{}'s caption \"{caption}\" is cut off: it needs about {need} pixels, {} is {w} wide — make it wider or the caption shorter.", n.id, n.id));
            }
        }
        // (a word-wrapped label that isn't sized by its text: its lines must
        // fit its height)
        if n.type_name == "RLABEL" && !caption.is_empty() && rapidr_ui_kernel::store::flag(store, &n.id, "wordwrap", false) && !rapidr_value::autosize::autosize_on(&store.get(&n.id, "autosize")) {
            let font = store.font(&n.id);
            let lines = rapidr_value::autosize::lines(&caption, &font, Some(w)).len() as i64;
            let need = rapidr_value::objects::text::text_size(" ", &font).1 * lines;
            if need > h + 1 {
                out.push(format!("{}'s text is cut off: wrapped it takes {lines} lines, about {need} pixels, {} is {h} high — make it taller or the text shorter.", n.id, n.id));
            }
        }
    }
    // (on top of each other: two solid controls of the same parent)
    let solid: Vec<usize> = ui.nodes.iter().enumerate().filter(|(_, n)| n.shown && n.abs.2 > 0 && n.abs.3 > 0 && !see_through(&n.type_name)).map(|(i, _)| i).collect();
    for (k, &a) in solid.iter().enumerate() {
        for &b in &solid[k + 1..] {
            let (na, nb) = (&ui.nodes[a], &ui.nodes[b]);
            if na.parent != nb.parent {
                continue;
            }
            let (ax, ay, aw, ah) = na.abs;
            let (bx, by, bw, bh) = nb.abs;
            let ox = (ax + aw).min(bx + bw) - ax.max(bx);
            let oy = (ay + ah).min(by + bh) - ay.max(by);
            if ox > 2 && oy > 2 {
                out.push(format!("{} and {} are on top of each other ({ox} × {oy} pixels overlap; {} is drawn over {}) — move one.", na.id, nb.id, nb.id, na.id));
            }
        }
    }
    out
}

/// The layout report of the windows shown (or of `only`): JSON, for
/// tools — each window's name, title, place and size, whether it's in
/// front and modal; each component's name, type, caption, text, place and
/// size in the window's inside (what a click at x, y takes), visible,
/// enabled and focused; and the problems ([`problems`]).
pub fn layout(desk: &mut Desktop, store: &dyn Store, only: Option<&str>) -> Json {
    let order = desk.stacking();
    let front = order.last().cloned();
    let modal = desk.modal.clone();
    let mut windows = Vec::new();
    for key in order.iter().rev() {
        if only.is_some_and(|o| !o.eq_ignore_ascii_case(key)) {
            continue;
        }
        let warnings = problems(desk, store, key);
        let Some(f) = desk.forms.get_mut(key) else { continue };
        f.ui.sync(store);
        let ui = &f.ui;
        let focused = ui.focus.map(|i| ui.nodes[i].id.clone());
        let mut comps = Vec::new();
        for n in &ui.nodes {
            let (caption, text) = texts(store, &n.id, &n.type_name);
            let mut c = json!({
                "name": n.id,
                "type": type_name(&n.type_name),
                "x": n.abs.0, "y": n.abs.1, "width": n.abs.2, "height": n.abs.3,
            });
            if !caption.is_empty() {
                c["caption"] = json!(caption);
            }
            if !text.is_empty() {
                c["text"] = json!(text);
            }
            if !n.shown {
                c["visible"] = json!(false);
            }
            if !n.enabled {
                c["enabled"] = json!(false);
            }
            if focused.as_deref() == Some(n.id.as_str()) {
                c["focused"] = json!(true);
            }
            if let Some(p) = n.parent {
                c["parent"] = json!(ui.nodes[p].id);
            }
            comps.push(c);
        }
        let (x, y) = f.spec.position.unwrap_or((0, 0));
        windows.push(json!({
            "name": key,
            "type": type_name(&store.type_of(key)),
            "title": f.spec.title,
            "x": x, "y": y,
            "width": ui.client.0, "height": ui.client.1 + ui.menu_offset,
            "menuBar": ui.menu_offset,
            "front": front.as_deref() == Some(key.as_str()),
            "modal": modal.iter().any(|m| m.eq_ignore_ascii_case(key)),
            "components": comps,
            "problems": warnings,
        }));
    }
    json!({ "windows": windows })
}

/// [`layout`] in words, a line a component, for a small model or a person:
///
/// ```text
/// Window form1 "Calculator" (RForm), inside 240 × 300, in front
///   display (REdit) text "0" at 8,8 size 224×28
///   b7 (RButton) "7" at 8,60 size 56×36
/// No problems seen (nothing cut off, nothing on top of another, all inside the window).
/// ```
pub fn layout_text(report: &Json) -> String {
    let mut out = String::new();
    let windows = report["windows"].as_array().cloned().unwrap_or_default();
    if windows.is_empty() {
        return "The program shows no window now.".into();
    }
    for w in &windows {
        let mut head = format!("Window {} \"{}\" ({}), inside {} × {}", w["name"].as_str().unwrap_or(""), w["title"].as_str().unwrap_or(""), w["type"].as_str().unwrap_or(""), w["width"], w["height"]);
        if w["front"] == json!(true) {
            head.push_str(", in front");
        }
        if w["modal"] == json!(true) {
            head.push_str(", modal (waits for an answer)");
        }
        out.push_str(&head);
        out.push('\n');
        for c in w["components"].as_array().into_iter().flatten() {
            let mut line = format!("  {} ({})", c["name"].as_str().unwrap_or(""), c["type"].as_str().unwrap_or(""));
            if let Some(cap) = c["caption"].as_str() {
                line.push_str(&format!(" \"{cap}\""));
            }
            if let Some(t) = c["text"].as_str() {
                line.push_str(&format!(" text \"{}\"", t.replace('\n', "⏎")));
            }
            line.push_str(&format!(" at {},{} size {}×{}", c["x"], c["y"], c["width"], c["height"]));
            if c["visible"] == json!(false) {
                line.push_str(", hidden");
            }
            if c["enabled"] == json!(false) {
                line.push_str(", disabled");
            }
            if c["focused"] == json!(true) {
                line.push_str(", has the keyboard");
            }
            out.push_str(&line);
            out.push('\n');
        }
        let problems = w["problems"].as_array().cloned().unwrap_or_default();
        if problems.is_empty() {
            out.push_str("No problems seen (nothing cut off, nothing on top of another, all inside the window).\n");
        } else {
            for p in problems {
                out.push_str("Problem: ");
                out.push_str(p.as_str().unwrap_or(""));
                out.push('\n');
            }
        }
    }
    out
}
