//! The demo's frames: the program's windows as the kernel draws them —
//! each window framed as RapidR's look frames one, placed where it is on
//! the (headless) screen, the program RapidR Studio runs over them, the
//! demo overlay on top — one picture a frame, at the screen's scale, so
//! text is as sharp as on a Retina screen. Never a read of the real
//! screen: nothing outside the program can show.
//!
//! A window is painted every frame (its display list), but drawn again
//! only when the list changed; a frame that is the same as one already
//! written (a caret blinking back, a still screen while a sentence is
//! said) is written once and listed again. PNG files are encoded by a few
//! threads; `frames.json` lists each frame's time and file.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use rapidr_ui_kernel::demo_overlay::{self, Overlay};
use rapidr_ui_kernel::frame::{self, Look};
use rapidr_ui_kernel::{DisplayList, Store};
use rapidr_ui_render::cpu::CpuRenderer;
use rapidr_value::objects::codec::Pixels;

use crate::desktop::Desktop;

/// A window drawn last time: its lists and pixels (inside and frame).
struct Drawn {
    inside: DisplayList,
    frame: DisplayList,
    renderer: CpuRenderer,
    frame_renderer: CpuRenderer,
    pixels: Pixels,
}

/// A layer from elsewhere (the program Studio runs): where on the screen
/// (logical pixels) and its pixels (already framed, at the scale).
#[derive(Clone, Debug)]
pub struct Layer {
    pub at: (i64, i64),
    pub pixels: Arc<Pixels>,
}

pub struct Recorder {
    dir: PathBuf,
    /// The frame in device pixels, and the scale.
    size: (usize, usize),
    scale: f64,
    drawn: HashMap<String, Drawn>,
    overlay: CpuRenderer,
    /// Each floating window's shadow, by its place: the pixels it darkens.
    shadows: HashMap<(i64, i64, i64, i64), Shadow>,
    /// Frames written, by their pixels' hash.
    seen: HashMap<u64, String>,
    /// Each frame: its time (ms of the video) and file.
    pub index: Vec<(u64, String)>,
    tx: Option<SyncSender<(PathBuf, Pixels)>>,
    workers: Vec<JoinHandle<()>>,
    errors: Arc<Mutex<Vec<String>>>,
    /// The last frame (the poster's, a test's look).
    pub last: Option<Pixels>,
}

/// A shadow's pixels: each one's index and the colour it darkens to.
type Shadow = Vec<(usize, [u8; 4])>;
/// A window's frame top left, its inside's offset in it, the frame's size.
pub type Placement = ((i64, i64), (i64, i64), (i64, i64));

/// Where window `form`'s frame and inside are on the screen (logical
/// pixels): its frame's top left, the inside's offset in it, the frame's
/// size.
pub fn placement(desk: &Desktop, store: &dyn Store, form: &str) -> Option<Placement> {
    let f = desk.forms.get(form)?;
    let caption = !f.spec.no_caption;
    let (iw, ih) = (f.ui.client.0, f.ui.client.1 + f.ui.menu_offset);
    let (inset, outer) = if f.spec.border { (frame::inset(true, caption), frame::outer((iw, ih), true, caption)) } else { ((0, 0), (iw, ih)) };
    let left = rapidr_ui_kernel::store::int(store, form, "left", 0);
    let top = rapidr_ui_kernel::store::int(store, form, "top", 0);
    // (a window the program didn't place — at 0, 0, not maximized — goes
    // where a desktop's window manager puts a new one: centred on the
    // screen, never under its top left corner)
    if f.state != 2 && (left, top) == (0, 0) {
        if let Some((sw, sh)) = super::screen() {
            return Some(((((sw - outer.0) / 2).max(0), ((sh - outer.1) / 2).max(0)), inset, outer));
        }
    }
    Some(((left, top), inset, outer))
}

/// A screen point (logical) of window `form`'s inside point `at`.
pub fn to_screen(desk: &Desktop, store: &dyn Store, form: &str, at: (f64, f64)) -> Option<(f64, f64)> {
    let ((x, y), (ix, iy), _) = placement(desk, store, form)?;
    Some(((x + ix) as f64 + at.0, (y + iy) as f64 + at.1))
}

/// The window under screen point `p` (the front one) and the point in its
/// inside.
pub fn from_screen(desk: &Desktop, store: &dyn Store, p: (f64, f64)) -> Option<(String, (f64, f64))> {
    for f in desk.stacking().into_iter().rev() {
        let Some(((x, y), (ix, iy), (w, h))) = placement(desk, store, &f) else { continue };
        if p.0 >= x as f64 && p.1 >= y as f64 && p.0 < (x + w) as f64 && p.1 < (y + h) as f64 {
            return Some((f, (p.0 - (x + ix) as f64, p.1 - (y + iy) as f64)));
        }
    }
    None
}

/// A fast hash of the pixels (identical frames found).
fn hash(px: &Pixels) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ (px.width as u64) << 32 ^ px.height as u64;
    for c in px.pixels.chunks(2) {
        let v = u64::from(c[0]) | u64::from(*c.get(1).unwrap_or(&0)) << 32;
        h = (h ^ v).wrapping_mul(0x100_0000_01b3).rotate_left(5);
    }
    h
}

/// `src` drawn into `dst` with its top left at (x, y) (device pixels; parts
/// outside cut off).
pub fn blit(dst: &mut Pixels, src: &Pixels, x: i64, y: i64) {
    for row in 0..src.height as i64 {
        let ty = y + row;
        if ty < 0 || ty >= dst.height as i64 {
            continue;
        }
        let x0 = x.max(0);
        let x1 = (x + src.width as i64).min(dst.width as i64);
        if x1 <= x0 {
            continue;
        }
        let s = (row as usize) * src.width + (x0 - x) as usize;
        let d = ty as usize * dst.width + x0 as usize;
        let n = (x1 - x0) as usize;
        dst.pixels[d..d + n].copy_from_slice(&src.pixels[s..s + n]);
    }
}

impl Recorder {
    /// Frames into `dir` (made), `screen` logical pixels at `scale`.
    pub fn new(dir: &Path, screen: (i64, i64), scale: f64) -> Result<Recorder, String> {
        std::fs::create_dir_all(dir).map_err(|e| format!("can't make {}: {e}", dir.display()))?;
        // (a take before this one's frames go: only demo mode's own files)
        if let Ok(old) = std::fs::read_dir(dir) {
            for e in old.flatten() {
                let n = e.file_name().to_string_lossy().into_owned();
                if (n.starts_with('f') && n.ends_with(".png")) || n == "frames.json" {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
        let size = (((screen.0 as f64) * scale).round() as usize, ((screen.1 as f64) * scale).round() as usize);
        let (tx, rx) = sync_channel::<(PathBuf, Pixels)>(16);
        let rx = Arc::new(Mutex::new(rx));
        let errors = Arc::new(Mutex::new(Vec::new()));
        let n = std::thread::available_parallelism().map_or(4, |n| n.get().clamp(2, 6));
        let workers = (0..n)
            .map(|_| {
                let rx = rx.clone();
                let errors = errors.clone();
                std::thread::spawn(move || loop {
                    let job = { rx.lock().ok().and_then(|r| r.recv().ok()) };
                    let Some((file, px)) = job else { break };
                    let r = rapidr_value::objects::codec::encode_png(&px).and_then(|b| std::fs::write(&file, b).map_err(|e| format!("{}: {e}", file.display())));
                    if let Err(e) = r {
                        if let Ok(mut v) = errors.lock() {
                            v.push(e);
                        }
                    }
                })
            })
            .collect();
        Ok(Recorder { dir: dir.to_path_buf(), size, scale, drawn: HashMap::new(), overlay: CpuRenderer::new(size.0 as u32, size.1 as u32), shadows: HashMap::new(), seen: HashMap::new(), index: Vec::new(), tx: Some(tx), workers, errors, last: None })
    }

    /// The frame now: every window shown (back to front), `layers` over
    /// them, the overlay on top.
    pub fn compose(&mut self, desk: &mut Desktop, store: &dyn Store, layers: &[Layer], overlay: &Overlay) -> Pixels {
        let t = rapidr_value::theme::current();
        // (where no window is: the theme's shadow tone, as a desktop)
        let bg = bgr(t.shadow);
        let mut out = Pixels { width: self.size.0, height: self.size.1, pixels: vec![bg; self.size.0 * self.size.1] };
        let order = desk.stacking();
        let front = order.last().cloned();
        for form in &order {
            let Some(((x, y), _, (w, h))) = placement(desk, store, form) else { continue };
            // (a window that isn't maximized floats: the theme's soft shadow
            // under it, as the web's windows have)
            if desk.forms.get(form).is_some_and(|f| f.state != 2 && f.spec.border) {
                self.shadow(desk, &mut out, (x, y, w, h));
            }
            if let Some(px) = self.window(desk, store, form, front.as_deref() == Some(form.as_str())) {
                blit(&mut out, &px, (x as f64 * self.scale).round() as i64, (y as f64 * self.scale).round() as i64);
            }
        }
        for l in layers {
            blit(&mut out, &l.pixels, (l.at.0 as f64 * self.scale).round() as i64, (l.at.1 as f64 * self.scale).round() as i64);
        }
        if !overlay.is_empty() {
            self.draw_overlay(desk, &mut out, overlay);
        }
        out
    }

    /// Window `form` drawn (framed), again only if its lists changed.
    fn window(&mut self, desk: &mut Desktop, store: &dyn Store, form: &str, active: bool) -> Option<Pixels> {
        let scale = self.scale;
        let icon = frame::icon_of(store, form);
        let Desktop { forms, text, .. } = desk;
        let f = forms.get_mut(form).filter(|f| f.shown)?;
        let caption = !f.spec.no_caption;
        let (iw, ih) = (f.ui.client.0, f.ui.client.1 + f.ui.menu_offset);
        let inside = f.ui.paint(store, text, scale);
        let look = Look { title: f.spec.title.clone(), active, border: f.spec.border, caption, frame: f.spec.frame, maximized: f.state == 2, icon };
        let (outer, inset) = if f.spec.border { (frame::outer((iw, ih), true, caption), frame::inset(true, caption)) } else { ((iw, ih), (0, 0)) };
        let framed = if f.spec.border { frame::paint(&look, outer, scale) } else { DisplayList::default() };
        let size_of = |l: &DisplayList| rapidr_ui_render::canvas::device_size(l);
        let d = self.drawn.entry(form.to_string()).or_insert_with(|| Drawn { inside: DisplayList::default(), frame: DisplayList::default(), renderer: CpuRenderer::new(1, 1), frame_renderer: CpuRenderer::new(1, 1), pixels: Pixels { width: 0, height: 0, pixels: Vec::new() } });
        if d.inside == inside && d.frame == framed && d.pixels.width > 0 {
            return Some(d.pixels.clone());
        }
        let (w, h) = size_of(&inside);
        d.renderer.render(w, h, &inside, text, &f.ui);
        let inner = d.renderer.pixels();
        let px = if f.spec.border {
            let (fw, fh) = size_of(&framed);
            d.frame_renderer.render(fw, fh, &framed, text, &f.ui);
            let mut whole = d.frame_renderer.pixels();
            blit(&mut whole, &inner, (inset.0 as f64 * scale).round() as i64, (inset.1 as f64 * scale).round() as i64);
            whole
        } else {
            inner
        };
        d.inside = inside;
        d.frame = framed;
        d.pixels = px.clone();
        Some(px)
    }

    /// The theme's shadow under a window at `rect` (logical): drawn once
    /// for each place, kept as the pixels it darkens.
    fn shadow(&mut self, desk: &mut Desktop, out: &mut Pixels, rect: (i64, i64, i64, i64)) {
        if !self.shadows.contains_key(&rect) {
            let t = rapidr_value::theme::current();
            let mut list = DisplayList { size: (((self.size.0 as f64) / self.scale).round() as i64, ((self.size.1 as f64) / self.scale).round() as i64), scale: self.scale, ..Default::default() };
            {
                let mut p = rapidr_ui_kernel::paint::Painter::new(&mut list);
                p.elevate(rect, t.radius.max(6.0), 28.0);
            }
            let mut sparse = Vec::new();
            let Desktop { forms, text, .. } = desk;
            if let Some(any) = forms.values().next() {
                let (w, h) = (self.size.0 as u32, self.size.1 as u32);
                self.overlay.render_transparent(w, h, &list, text, &any.ui);
                for (i, p) in self.overlay.pixmap.data().iter().enumerate() {
                    if p.a > 0 {
                        sparse.push((i, [p.r, p.g, p.b, p.a]));
                    }
                }
            }
            self.shadows.insert(rect, sparse);
        }
        for (i, [r, g, b, a]) in &self.shadows[&rect] {
            blend(&mut out.pixels[*i], *r, *g, *b, *a);
        }
    }

    fn draw_overlay(&mut self, desk: &mut Desktop, out: &mut Pixels, overlay: &Overlay) {
        let mut list = DisplayList { size: overlay.size, scale: self.scale, ..Default::default() };
        {
            let mut p = rapidr_ui_kernel::paint::Painter::new(&mut list);
            demo_overlay::paint(&mut p, overlay);
        }
        let Desktop { forms, text, .. } = desk;
        // (any form serves as the renderer's: the overlay has no editors)
        let Some(any) = forms.values().next() else { return };
        let (w, h) = (self.size.0 as u32, self.size.1 as u32);
        self.overlay.render_transparent(w, h, &list, text, &any.ui);
        for (o, p) in out.pixels.iter_mut().zip(self.overlay.pixmap.data()) {
            if p.a > 0 {
                blend(o, p.r, p.g, p.b, p.a);
            }
        }
    }

    /// Frame `px` at `ms` of the video: written (or listed again when the
    /// same frame was written before).
    pub fn push(&mut self, ms: u64, px: Pixels) {
        let h = hash(&px);
        let file = match self.seen.get(&h) {
            Some(f) => f.clone(),
            None => {
                let name = format!("f{:05}.png", self.seen.len() + 1);
                if let Some(tx) = &self.tx {
                    let _ = tx.send((self.dir.join(&name), px.clone()));
                }
                self.seen.insert(h, name.clone());
                name
            }
        };
        self.index.push((ms, file));
        self.last = Some(px);
    }

    /// The writers done; `frames.json` written. Errors from the writers.
    pub fn finish(&mut self) -> Result<(), String> {
        self.tx = None;
        for w in self.workers.drain(..) {
            let _ = w.join();
        }
        let list: Vec<serde_json::Value> = self.index.iter().map(|(t, f)| serde_json::json!({"t": t, "file": f})).collect();
        let doc = serde_json::json!({"size": [self.size.0, self.size.1], "scale": self.scale, "written": self.seen.len(), "frames": list});
        std::fs::write(self.dir.join("frames.json"), doc.to_string()).map_err(|e| format!("can't write frames.json: {e}"))?;
        let errors = self.errors.lock().map(|e| e.clone()).unwrap_or_default();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(format!("{} frame(s) not written: {}", errors.len(), errors[0]))
        }
    }

    /// Distinct frames written so far.
    pub fn written(&self) -> usize {
        self.seen.len()
    }
}

/// A premultiplied pixel over `o` (&HBBGGRR): out = src + dst × (1 − a).
fn blend(o: &mut u32, r: u8, g: u8, b: u8, a: u8) {
    if a == 255 {
        *o = u32::from(r) | u32::from(g) << 8 | u32::from(b) << 16;
        return;
    }
    let k = 255 - u32::from(a);
    let mix = |src: u8, dst: u32| (u32::from(src) + (dst * k + 127) / 255).min(255);
    let (dr, dg, db) = (*o & 0xFF, *o >> 8 & 0xFF, *o >> 16 & 0xFF);
    *o = mix(r, dr) | mix(g, dg) << 8 | mix(b, db) << 16;
}

/// 0xRRGGBB → &HBBGGRR.
pub fn bgr(c: u32) -> u32 {
    (c & 0xFF) << 16 | (c & 0xFF00) | (c >> 16 & 0xFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blit_clips_at_every_edge() {
        let mut dst = Pixels { width: 4, height: 3, pixels: vec![0; 12] };
        let src = Pixels { width: 2, height: 2, pixels: vec![1, 2, 3, 4] };
        blit(&mut dst, &src, -1, -1);
        assert_eq!(dst.pixels[0], 4);
        blit(&mut dst, &src, 3, 2);
        assert_eq!(dst.pixels[11], 1);
        blit(&mut dst, &src, 9, 9);
        assert_eq!(dst.pixels.iter().filter(|&&p| p != 0).count(), 2);
    }

    #[test]
    fn identical_frames_hash_alike() {
        let a = Pixels { width: 2, height: 2, pixels: vec![1, 2, 3, 4] };
        let b = Pixels { width: 2, height: 2, pixels: vec![1, 2, 3, 5] };
        assert_eq!(hash(&a), hash(&a.clone()));
        assert_ne!(hash(&a), hash(&b));
    }
}
