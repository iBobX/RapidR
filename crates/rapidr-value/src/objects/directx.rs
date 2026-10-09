//! RapidQ's DirectX 2D objects (manual, Appendix B), implemented once for
//! every runtime (docs/directx-plan.md). RapidQ built them on DelphiX's
//! TDXDraw, TDXImageList and TDXTimer (its D3D notes say so); RapidR has no
//! DirectX: a screen is a CPU surface every runtime shows as a picture.
//!
//! - **QDXSCREEN** ([`DxScreen`]): every drawing call goes to an off-screen
//!   surface, the back buffer; nothing shows until `Flip` copies it to what
//!   the screen shows, the front ("always flip to see it"). The back buffer
//!   keeps its pixels after a Flip, as a windowed DirectDraw blit does. The
//!   screen is set up — cleared to black, OnInitialize then
//!   OnInitializeSurface — when its form is first shown ([`initialize`]):
//!   what a program draws before that is lost, as DirectX had no surface
//!   yet (RapidQ's manual says to wait for the surface's initialization
//!   before drawing).
//! - The surface's size: `Init(Width, Height)`'s, or the control's without
//!   one. With AutoSize (the default) a control resized after that — the
//!   `Align = alClient` RapidQ programs set after Init — gives the surface
//!   its new size, as DelphiX's AutoSize did on WM_SIZE
//!   ([`DxScreen::follow_control`]; the corpus's `mgplist.bas` swaps Init's
//!   width and height and still shows right this way). Where the surface
//!   and the control differ, AllowStretch (the default) stretches the
//!   picture over the control, else it shows at its own size at the top
//!   left.
//! - Text is drawn in the screen's Font, MS Sans Serif 8 until the program
//!   sets one (`Font = QFont`, `Font.Size = …`): DelphiX's surface canvas
//!   was a TCanvas with Delphi's default font, as the manual's chapter 13
//!   screenshots show ("FPS: 20").
//! - FullScreen (manual: set before ShowModal, with the form bsNone): the
//!   form's window covers the screen and the surface keeps its size — as
//!   DirectDraw's exclusive mode switched the display to it — shown scaled
//!   to the screen with its proportions kept, black around ([`picture_rect`];
//!   no display mode is changed).
//! - Rotate(xOrigin, yOrigin, Angle) turns what the back buffer holds about
//!   the point, clockwise, Angle in DelphiX's units (256 a whole turn —
//!   its `Cos256` tables); View.SetFront / SetBack / SetPlane keep the 3D
//!   viewport's clipping planes (front 1, back 5000 as the manual says),
//!   View.Clear clears the back buffer to the scene's background (black).
//! - Colours are RapidQ's &HBBGGRR everywhere but `Fill` and `FastPset`,
//!   which write the surface's own pixel value as DirectDraw does: &HRRGGBB
//!   on today's 32-bit screens (the manual: "Fill … &HFF = Blue, &HFF00 =
//!   Green, &HFF0000 = Red"). `Line(0, 0, 200, 200, &HFF)` is red, as the
//!   manual's example says.
//! - **QDXIMAGELIST** ([`DxImageList`]): the pictures of a DelphiX image
//!   library (`.DXG`, a TPictureCollection streamed by Delphi), drawn onto
//!   its Parent screen with `Draw(Item, X, Y, Pattern)` — the fourth
//!   argument (the manual's "Mask") is DelphiX's pattern index into a
//!   picture cut into PatternWidth × PatternHeight cells; a Transparent
//!   picture leaves out its TransparentColor.
//! - **QDXSOUND** ([`DxSound`]): a WAV file played by the runtime's sound
//!   device ([`SoundDevice`]: rodio on the desktop, Web Audio in the
//!   browser) at Frequency, Volume and Pan (DirectSound's decibels), Looped
//!   or once; Playing and Position follow the clock.
//! - **QDXTIMER** ([`DxTimer`]): the runtimes' timers fire it (Interval 0
//!   is once a screen refresh, [`DX_FRAME_MS`]); it counts its OnTimers
//!   into FrameRate, the last whole second's.

use std::cell::Cell;
use std::rc::Rc;

use super::bitmap::Bitmap;
use crate::{v_int, Value};

/// A QDXSCREEN.
#[derive(Debug, Clone)]
pub struct DxScreen {
    /// The off-screen surface the program draws on.
    pub back: Bitmap,
    /// What the last Flip showed.
    pub front: Bitmap,
    /// `Init(Width, Height)`: the surface's size (`None`: the control's,
    /// DelphiX's AutoSize, until Init).
    pub init: Option<(i64, i64)>,
    /// Its form was shown and OnInitialize fired.
    pub initialized: bool,
    /// The control's size when the surface last took one (AutoSize
    /// follows the control when it changes).
    seen: Option<(i64, i64)>,
    /// Flips so far.
    pub flips: u64,
    /// The 3D viewport's front and back clipping planes (View.SetFront /
    /// SetBack) and its front plane's sides (View.SetPlane: left, right,
    /// bottom, top; `None`: the camera's default field).
    pub view_front: f64,
    pub view_back: f64,
    pub view_plane: Option<[f64; 4]>,
    /// The scene's background (View.Clear's colour; SetBackgroundRGB).
    pub background: u32,
}

/// DelphiX's surface canvas font: Delphi's default TFont.
fn screen_font() -> super::font::Font {
    super::font::Font { name: "MS Sans Serif".into(), size: 8, color: 0, styles: 0 }
}

impl Default for DxScreen {
    fn default() -> Self {
        let black = || Bitmap { background: 0, transparent_color: 0, font: screen_font(), ..Bitmap::default() };
        Self { back: black(), front: black(), init: None, initialized: false, seen: None, flips: 0, view_front: 1.0, view_back: 5000.0, view_plane: None, background: 0 }
    }
}

/// A DirectDraw pixel value (&HRRGGBB on a 32-bit surface) as a RapidQ
/// colour (&HBBGGRR).
fn device_color(c: i64) -> u32 {
    let c = c as u32 & 0xFF_FFFF;
    (c & 0xFF) << 16 | (c & 0xFF00) | c >> 16
}

impl DxScreen {
    /// The surface takes size `w` × `h` (what's drawn stays where it fits;
    /// new area is black).
    fn size_to(&mut self, w: i64, h: i64) {
        self.back.resize(w, h);
        self.front.resize(w, h);
    }

    /// The control is `w` × `h` now: with AutoSize, a control resized
    /// since the surface took its size gives the surface its size (what's
    /// drawn stays where it fits).
    pub fn follow_control(&mut self, w: i64, h: i64, autosize: bool) {
        let resized = self.seen.is_some_and(|s| s != (w, h));
        self.seen = Some((w, h));
        if autosize && resized && w > 0 && h > 0 {
            self.size_to(w, h);
        }
    }

    /// Set up as DirectX did when the window came (the control `w` × `h`):
    /// the surface sized — Init's, the control's without one or after a
    /// resize with AutoSize — and cleared to black, nothing shown yet.
    fn initialize(&mut self, w: i64, h: i64, autosize: bool) {
        self.follow_control(w, h, autosize);
        if self.back.img.width == 0 || self.back.img.height == 0 {
            let (w, h) = self.init.unwrap_or((w, h));
            self.size_to(w, h);
        }
        let (w, h) = (self.back.img.width as i64, self.back.img.height as i64);
        self.back.fill_rect(0, 0, w, h, 0);
        self.front = self.back.clone();
        self.initialized = true;
    }

    /// `Flip`: what the back buffer holds shows (it keeps its pixels).
    pub fn flip(&mut self) {
        self.front = self.back.clone();
        self.flips += 1;
    }

    pub fn get(&self, prop: &str) -> Option<Value> {
        if let Some(p) = prop.strip_prefix("font.") {
            return self.back.font.get(p);
        }
        // (`DX.View.GetFront` read without parentheses)
        match prop {
            "view.getfront" => Some(Value::Double(self.view_front)),
            "view.getback" => Some(Value::Double(self.view_back)),
            _ => None,
        }
    }

    /// `Rotate(xOrigin, yOrigin, Angle)`: what the back buffer holds turned
    /// about (x0, y0) by `angle` 256ths of a turn, clockwise on the screen;
    /// where nothing turns onto, the pixels stay.
    pub fn rotate(&mut self, x0: i64, y0: i64, angle: i64) {
        let (w, h) = (self.back.img.width as i64, self.back.img.height as i64);
        if w == 0 || h == 0 || angle.rem_euclid(256) == 0 {
            return;
        }
        let a = angle.rem_euclid(256) as f64 * std::f64::consts::TAU / 256.0;
        let (sin, cos) = a.sin_cos();
        let src = self.back.img.pixels.clone();
        for y in 0..h {
            for x in 0..w {
                // The pixel that turns onto (x, y): turned back.
                let (dx, dy) = ((x - x0) as f64, (y - y0) as f64);
                let sx = (x0 as f64 + dx * cos + dy * sin).round() as i64;
                let sy = (y0 as f64 - dx * sin + dy * cos).round() as i64;
                if (0..w).contains(&sx) && (0..h).contains(&sy) {
                    self.back.img.pixels[(y * w + x) as usize] = src[(sy * w + sx) as usize];
                }
            }
        }
        self.back.invalidate_display();
    }

    pub fn set(&mut self, prop: &str, val: &Value) -> Option<Result<(), String>> {
        if let Some(p) = prop.strip_prefix("font.") {
            return self.back.font.set(p, val).then_some(Ok(()));
        }
        None
    }

    /// The drawing methods that need nothing but the surface (`Draw`,
    /// `StretchDraw`, `CopyRect`, `TextRect`: objects::call).
    pub fn call(&mut self, method: &str, args: &[Value]) -> Option<Value> {
        let n = |i: usize| args.get(i).map_or(0, Value::to_i64);
        match method {
            "init" => {
                let (w, h) = (n(0).clamp(0, 32767), n(1).clamp(0, 32767));
                self.init = Some((w, h));
                self.size_to(w, h);
            }
            "flip" => self.flip(),
            // DirectDraw's own pixel values (module docs).
            "fill" => {
                let (w, h) = (self.back.img.width as i64, self.back.img.height as i64);
                self.back.fill_rect(0, 0, w, h, device_color(n(0)));
            }
            "fastpset" => self.back.pset(n(0), n(1), device_color(n(2))),
            "rotate" => self.rotate(n(0), n(1), n(2)),
            // The 3D viewport (`DX.View.SetFront(10)`: method `view.setfront`).
            "view.clear" => {
                let (w, h) = (self.back.img.width as i64, self.back.img.height as i64);
                self.back.fill_rect(0, 0, w, h, self.background);
            }
            "view.setfront" => self.view_front = args.first().map_or(1.0, Value::to_f64),
            "view.setback" => self.view_back = args.first().map_or(5000.0, Value::to_f64),
            "view.setplane" if args.len() >= 4 => {
                let f = |i: usize| args[i].to_f64();
                self.view_plane = Some([f(0), f(1), f(2), f(3)]);
            }
            "view.getfront" | "view.getback" => return self.get(method),
            // (nothing to give back: the surface is the runtime's memory)
            "release" => {}
            // `Pixel(x, y)` reads; `Pixel(x, y) = c` writes (the value last).
            "pixel" if args.len() < 3 => return Some(v_int(self.back.pixel(n(0), n(1)).map_or(-1, i64::from))),
            "pset" | "pixel" | "line" | "rectangle" | "fillrect" | "circle" | "roundrect" | "paint" | "textout" | "textwidth" | "textheight" => {
                return self.back.call(method, args);
            }
            _ => return None,
        }
        Some(Value::Null)
    }
}

/// Where a screen's picture goes in its control (logical pixels): the
/// surface `surface` (w, h) in a control `control` (w, h) — over all of it
/// with AllowStretch, at its own size at the top left without, and with
/// FullScreen scaled to fit with its proportions kept, centred.
pub fn picture_rect(surface: (i64, i64), control: (i64, i64), stretch: bool, fullscreen: bool) -> (i64, i64, i64, i64) {
    let ((sw, sh), (cw, ch)) = (surface, control);
    if fullscreen && sw > 0 && sh > 0 {
        let k = (cw as f64 / sw as f64).min(ch as f64 / sh as f64);
        let (w, h) = ((sw as f64 * k).round() as i64, (sh as f64 * k).round() as i64);
        return ((cw - w) / 2, (ch - h) / 2, w, h);
    }
    if stretch { (0, 0, cw, ch) } else { (0, 0, sw, sh) }
}

/// Sets QDXSCREEN `screen` up (its form shown; `w` × `h` the control's
/// size, `autosize` its AutoSize): `true` the first time — the runtime then
/// fires OnInitialize and OnInitializeSurface.
pub fn initialize(screen: &mut DxScreen, w: i64, h: i64, autosize: bool) -> bool {
    if screen.initialized {
        return false;
    }
    screen.initialize(w, h, autosize);
    true
}

// ------------------------------------------------------------ QDXTIMER --

/// How often a QDXTIMER with Interval 0 ("let DirectX handle FPS") fires:
/// once a 60 Hz screen refresh, as a Flip waiting for the vertical blank
/// paced DelphiX's idle-time timer.
pub const DX_FRAME_MS: u64 = 16;

/// A QDXTIMER's frame counter.
#[derive(Debug, Clone, Default)]
pub struct DxTimer {
    /// OnTimers since `since`.
    frames: i64,
    /// When the current second began (the runtime's milliseconds).
    since: Option<f64>,
    /// FrameRate: OnTimers in the last whole second.
    pub rate: i64,
}

impl DxTimer {
    /// Whether the timer's OnTimer fires now: with ActiveOnly (the
    /// default) only while the program is the active application (one of
    /// its windows has the keyboard; on the web, the page shows).
    pub fn fires(active_only: bool, app_active: bool) -> bool {
        !active_only || app_active
    }

    /// The timer fires at `now_ms` (any clock the runtime keeps).
    pub fn tick(&mut self, now_ms: f64) {
        let since = *self.since.get_or_insert(now_ms);
        self.frames += 1;
        if now_ms - since >= 1000.0 {
            self.rate = self.frames;
            self.frames = 0;
            self.since = Some(now_ms);
        }
    }

    pub fn get(&self, prop: &str) -> Option<Value> {
        (prop == "framerate").then(|| v_int(self.rate))
    }
}

/// A QDXTIMER's milliseconds between OnTimers for its Interval `ms`.
pub fn timer_interval_ms(ms: i64) -> u64 {
    if ms > 0 { ms as u64 } else { DX_FRAME_MS }
}

// -------------------------------------------------------- QDXIMAGELIST --

/// One picture of an image library.
#[derive(Debug, Clone)]
pub struct DxPicture {
    pub name: String,
    /// Its pixels (Transparent / TransparentColor set on it); shared so a
    /// Draw doesn't copy it.
    pub picture: Rc<Bitmap>,
    /// PatternWidth, PatternHeight (0: the whole picture).
    pub pattern: (i64, i64),
    /// SkipWidth, SkipHeight: the gap between patterns.
    pub skip: (i64, i64),
}

impl DxPicture {
    /// Pattern `index`'s rectangle in the picture (DelphiX's patterns run
    /// left to right, then down); `None` past the last.
    pub fn pattern_rect(&self, index: i64) -> Option<(i64, i64, i64, i64)> {
        let (w, h) = (self.picture.img.width as i64, self.picture.img.height as i64);
        let pw = if self.pattern.0 > 0 { self.pattern.0 } else { w };
        let ph = if self.pattern.1 > 0 { self.pattern.1 } else { h };
        let (sw, sh) = (self.skip.0.max(0), self.skip.1.max(0));
        let across = ((w + sw) / (pw + sw)).max(1);
        let down = ((h + sh) / (ph + sh)).max(1);
        if index < 0 || index >= across * down {
            return None;
        }
        let (x, y) = ((index % across) * (pw + sw), (index / across) * (ph + sh));
        Some((x, y, x + pw, y + ph))
    }
}

/// A QDXIMAGELIST.
#[derive(Debug, Clone, Default)]
pub struct DxImageList {
    pub items: Vec<DxPicture>,
}

impl DxImageList {
    /// Loads a `.DXG` image library (replacing what was loaded).
    pub fn load(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.items = parse_dxg(bytes)?;
        Ok(())
    }

    pub fn get(&self, prop: &str) -> Option<Value> {
        (prop == "count").then(|| v_int(self.items.len() as i64))
    }
}

/// Delphi's colour names (TColor, &HBBGGRR) an image library's
/// TransparentColor is streamed as.
fn delphi_color(name: &str) -> Option<u32> {
    Some(match name.to_ascii_lowercase().as_str() {
        "clblack" => 0x000000,
        "clmaroon" => 0x000080,
        "clgreen" => 0x008000,
        "clolive" => 0x008080,
        "clnavy" => 0x800000,
        "clpurple" => 0x800080,
        "clteal" => 0x808000,
        "clgray" => 0x808080,
        "clsilver" => 0xC0C0C0,
        "clred" => 0x0000FF,
        "cllime" => 0x00FF00,
        "clyellow" => 0x00FFFF,
        "clblue" => 0xFF0000,
        "clfuchsia" => 0xFF00FF,
        "claqua" => 0xFFFF00,
        "clwhite" => 0xFFFFFF,
        _ => return None,
    })
}

/// A value of Delphi's binary form streams (TPF0).
#[derive(Debug, Clone, PartialEq)]
enum Streamed {
    Int(i64),
    Text(String),
    Bool(bool),
    Binary(Vec<u8>),
    Collection(Vec<Vec<(String, Streamed)>>),
    Other,
}

/// A reader of Delphi's binary form streams (Classes.pas' TReader).
struct Reader<'a> {
    b: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn bytes(&mut self, n: usize) -> Result<&[u8], String> {
        let end = self.at.checked_add(n).filter(|&e| e <= self.b.len()).ok_or("the image library ends early")?;
        let s = &self.b[self.at..end];
        self.at = end;
        Ok(s)
    }

    fn byte(&mut self) -> Result<u8, String> {
        Ok(self.bytes(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().map_err(|_| "bad image library")?))
    }

    /// A short string (a length byte, then Latin-1).
    fn name(&mut self) -> Result<String, String> {
        let n = self.byte()? as usize;
        Ok(self.bytes(n)?.iter().map(|&c| char::from(c)).collect())
    }

    fn value(&mut self) -> Result<Streamed, String> {
        let latin1 = |b: &[u8]| b.iter().map(|&c| char::from(c)).collect::<String>();
        Ok(match self.byte()? {
            0 => Streamed::Other,
            // vaList: values until a vaNull
            1 => {
                while self.b.get(self.at).is_some_and(|&t| t != 0) {
                    self.value()?;
                }
                self.at += 1;
                Streamed::Other
            }
            2 => Streamed::Int(i64::from(self.byte()? as i8)),
            3 => Streamed::Int(i64::from(i16::from_le_bytes(self.bytes(2)?.try_into().map_err(|_| "bad image library")?))),
            4 => Streamed::Int(i64::from(self.u32()? as i32)),
            5 => {
                self.bytes(10)?;
                Streamed::Other
            }
            6 | 7 => Streamed::Text(self.name()?),
            8 => Streamed::Bool(false),
            9 => Streamed::Bool(true),
            10 => {
                let n = self.u32()? as usize;
                Streamed::Binary(self.bytes(n)?.to_vec())
            }
            11 => {
                while !self.name()?.is_empty() {}
                Streamed::Other
            }
            12 | 20 => {
                let n = self.u32()? as usize;
                Streamed::Text(latin1(self.bytes(n)?))
            }
            13 => Streamed::Other,
            14 => {
                let mut items = Vec::new();
                while self.b.get(self.at).is_some_and(|&t| t != 0) {
                    // (an item's order, when streamed, then vaList)
                    if matches!(self.b.get(self.at), Some(2..=4)) {
                        self.value()?;
                    }
                    if self.b.get(self.at) == Some(&1) {
                        self.at += 1;
                    }
                    items.push(self.properties()?);
                }
                self.at += 1;
                Streamed::Collection(items)
            }
            15 => {
                self.bytes(4)?;
                Streamed::Other
            }
            16 | 17 | 19 => {
                self.bytes(8)?;
                Streamed::Other
            }
            18 => {
                let n = self.u32()? as usize;
                let w = self.bytes(n * 2)?;
                Streamed::Text(char::decode_utf16(w.as_chunks::<2>().0.iter().map(|p| u16::from_le_bytes(*p))).map(|c| c.unwrap_or('?')).collect())
            }
            t => return Err(format!("not an image library RapidR can read (value type {t})")),
        })
    }

    /// Named values until a vaNull.
    fn properties(&mut self) -> Result<Vec<(String, Streamed)>, String> {
        let mut props = Vec::new();
        while self.b.get(self.at).is_some_and(|&t| t != 0) {
            let name = self.name()?;
            props.push((name, self.value()?));
        }
        self.at += 1;
        Ok(props)
    }
}

/// A picture's `Picture.Data` (a graphic class's name, then its data) as
/// a BMP file: DelphiX's TDIB streams a BITMAPINFOHEADER, its palette and
/// its rows; Delphi's TBitmap the size and a BMP file.
fn picture_bmp(data: &[u8]) -> Result<Vec<u8>, String> {
    let bad = || "a picture in the image library RapidR can't read".to_string();
    let class_len = *data.first().ok_or_else(bad)? as usize;
    let rest = data.get(1 + class_len..).ok_or_else(bad)?;
    if rest.starts_with(b"BM") {
        return Ok(rest.to_vec());
    }
    if rest.get(4..6) == Some(b"BM") {
        return Ok(rest[4..].to_vec());
    }
    let u32_at = |i: usize| rest.get(i..i + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    let u16_at = |i: usize| rest.get(i..i + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
    let header = u32_at(0).ok_or_else(bad)? as usize;
    if !matches!(header, 40 | 52 | 56 | 108 | 124) {
        return Err(bad());
    }
    let bpp = u16_at(14).ok_or_else(bad)?;
    let compression = u32_at(16).ok_or_else(bad)?;
    let used = u32_at(32).unwrap_or(0) as usize;
    let palette = if bpp <= 8 { (if used > 0 { used.min(256) } else { 1 << bpp }) * 4 } else { 0 };
    let masks = if compression == 3 && header == 40 { 12 } else { 0 };
    let offset = 14 + header + palette + masks;
    let mut bmp = Vec::with_capacity(14 + rest.len());
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&((14 + rest.len()) as u32).to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&(offset as u32).to_le_bytes());
    bmp.extend_from_slice(rest);
    Ok(bmp)
}

/// The pictures of a DelphiX image library (`.DXG`: a resource header,
/// then a TPictureCollectionComponent streamed as a Delphi form, `TPF0`).
pub fn parse_dxg(bytes: &[u8]) -> Result<Vec<DxPicture>, String> {
    let start = bytes.windows(4).position(|w| w == b"TPF0").ok_or("not a DelphiX image library (.DXG)")?;
    let mut r = Reader { b: bytes, at: start + 4 };
    r.name()?; // the class
    r.name()?; // its name
    let mut pictures = Vec::new();
    for (prop, value) in r.properties()? {
        let Streamed::Collection(items) = value else { continue };
        if !prop.eq_ignore_ascii_case("list") {
            continue;
        }
        for props in items {
            let int = |n: &str| props.iter().find(|(p, _)| p.eq_ignore_ascii_case(n)).and_then(|(_, v)| if let Streamed::Int(i) = v { Some(*i) } else { None });
            let value = |n: &str| props.iter().find(|(p, _)| p.eq_ignore_ascii_case(n)).map(|(_, v)| v);
            let Some(Streamed::Binary(data)) = value("Picture.Data") else { continue };
            let mut b = Bitmap::default();
            b.load_bmp_bytes(&picture_bmp(data)?)?;
            b.transparent = matches!(value("Transparent"), Some(Streamed::Bool(true)) | None);
            b.transparent_color = match value("TransparentColor") {
                Some(Streamed::Int(c)) => *c as u32 & 0xFF_FFFF,
                Some(Streamed::Text(name)) => delphi_color(name).unwrap_or(0),
                _ => 0,
            };
            let name = match value("Name") {
                Some(Streamed::Text(n)) => n.clone(),
                _ => String::new(),
            };
            pictures.push(DxPicture {
                name,
                picture: Rc::new(b),
                pattern: (int("PatternWidth").unwrap_or(0), int("PatternHeight").unwrap_or(0)),
                skip: (int("SkipWidth").unwrap_or(0), int("SkipHeight").unwrap_or(0)),
            });
        }
    }
    Ok(pictures)
}

/// Picture `item`'s pattern `pattern` as a bitmap to draw (with its
/// transparency): the shared picture itself when it's the whole of it.
pub fn pattern_bitmap(p: &mut DxPicture, pattern: i64) -> Option<Rc<Bitmap>> {
    let rect = p.pattern_rect(pattern)?;
    // (what it shows at the screen's scale made once, not at every Draw)
    Rc::make_mut(&mut p.picture).display_revision();
    let (w, h) = (p.picture.img.width as i64, p.picture.img.height as i64);
    if rect == (0, 0, w, h) {
        return Some(p.picture.clone());
    }
    let mut part = Bitmap { transparent: p.picture.transparent, transparent_color: p.picture.transparent_color, ..Bitmap::default() };
    part.resize(rect.2 - rect.0, rect.3 - rect.1);
    part.copy_rect((0, 0, rect.2 - rect.0, rect.3 - rect.1), &p.picture, rect);
    Some(Rc::new(part))
}

// ------------------------------------------------------------ QDXSOUND --

/// A WAV file's sound: uncompressed PCM, as DirectSound buffers held it.
#[derive(Debug, Clone, PartialEq)]
pub struct Wav {
    pub channels: u16,
    /// Frames a second (a frame: a sample per channel).
    pub rate: u32,
    /// 8 (unsigned) or 16 (signed) bits a sample.
    pub bits: u16,
    /// The sound's bytes (the `data` chunk).
    pub data: Rc<[u8]>,
}

impl Wav {
    /// Bytes a frame.
    pub fn block_align(&self) -> usize {
        usize::from(self.channels) * usize::from(self.bits / 8)
    }

    /// Its frames.
    pub fn frames(&self) -> usize {
        self.data.len() / self.block_align().max(1)
    }

    /// Its frames as stereo samples (-1.0 … 1.0, left then right), with
    /// `gains` (left, right) applied: what a device plays.
    pub fn stereo(&self, gains: (f32, f32)) -> Vec<f32> {
        let sample = |i: usize| -> f32 {
            match self.bits {
                8 => (f32::from(self.data[i]) - 128.0) / 128.0,
                _ => f32::from(i16::from_le_bytes([self.data[i], self.data[i + 1]])) / 32768.0,
            }
        };
        let (block, width) = (self.block_align(), usize::from(self.bits / 8));
        let mut out = Vec::with_capacity(self.frames() * 2);
        for f in 0..self.frames() {
            let at = f * block;
            let left = sample(at);
            let right = if self.channels > 1 { sample(at + width) } else { left };
            out.push(left * gains.0);
            out.push(right * gains.1);
        }
        out
    }
}

/// Reads a WAV file (RIFF WAVE, PCM — WAVE_FORMAT_PCM, or an extensible
/// format holding PCM; 8 or 16 bits, mono or stereo: what DirectSound
/// buffers took).
pub fn parse_wav(b: &[u8]) -> Result<Wav, String> {
    let bad = |why: &str| format!("not a WAV file RapidR can play ({why})");
    if b.get(0..4) != Some(b"RIFF") || b.get(8..12) != Some(b"WAVE") {
        return Err(bad("no RIFF WAVE header"));
    }
    let u16_at = |i: usize| b.get(i..i + 2).map(|s| u16::from_le_bytes([s[0], s[1]]));
    let u32_at = |i: usize| b.get(i..i + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]));
    let mut at = 12;
    let mut format: Option<(u16, u16, u32, u16)> = None;
    let mut data: Option<&[u8]> = None;
    while at + 8 <= b.len() {
        let len = u32_at(at + 4).unwrap_or(0) as usize;
        let body = &b[at + 8..(at + 8).saturating_add(len).min(b.len())];
        match &b[at..at + 4] {
            b"fmt " => {
                let tag = u16_at(at + 8).ok_or_else(|| bad("short fmt chunk"))?;
                // (WAVE_FORMAT_EXTENSIBLE: its sub-format's first two bytes)
                let tag = if tag == 0xFFFE { u16_at(at + 8 + 24).unwrap_or(0) } else { tag };
                format = Some((tag, u16_at(at + 10).unwrap_or(0), u32_at(at + 12).unwrap_or(0), u16_at(at + 22).unwrap_or(0)));
            }
            b"data" => data = Some(body),
            _ => {}
        }
        at = at.saturating_add(8 + len + (len & 1));
    }
    let (tag, channels, rate, bits) = format.ok_or_else(|| bad("no fmt chunk"))?;
    if tag != 1 {
        return Err(bad("compressed"));
    }
    if !matches!(bits, 8 | 16) || !(1..=2).contains(&channels) || rate == 0 {
        return Err(bad("not 8 or 16-bit mono or stereo"));
    }
    let data = data.ok_or_else(|| bad("no data chunk"))?;
    Ok(Wav { channels, rate, bits, data: Rc::from(data) })
}

/// What a runtime's sound device is asked to play for a QDXSOUND.
#[derive(Debug, Clone)]
pub struct SoundPlay {
    /// The QDXSOUND's id (a new play of it replaces the old).
    pub id: String,
    pub wav: Rc<Wav>,
    /// The frame to start at.
    pub from: usize,
    /// Playback speed: Frequency over the file's own rate.
    pub speed: f64,
    /// Volume's gain, and Pan's (left, right) gains.
    pub gain: f32,
    pub pan: (f32, f32),
    pub looped: bool,
}

/// A runtime's sound device: plays (replacing what the QDXSOUND played),
/// sets a playing sound's gain, stops. None: silent — the GUI tests, a
/// build without audio, a machine without a sound card.
#[derive(Clone, Copy)]
pub struct SoundDevice {
    pub play: fn(&SoundPlay),
    pub volume: fn(&str, f32),
    pub stop: fn(&str),
}

thread_local! {
    static DEVICE: Cell<Option<SoundDevice>> = const { Cell::new(None) };
    static CLOCK: Cell<fn() -> f64> = const { Cell::new(default_clock) };
}

fn default_clock() -> f64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        START.get_or_init(std::time::Instant::now).elapsed().as_secs_f64() * 1000.0
    }
    #[cfg(target_arch = "wasm32")]
    {
        0.0
    }
}

/// The runtime's sound device for QDXSOUND (once per process).
pub fn set_sound_device(d: SoundDevice) {
    DEVICE.with(|c| c.set(Some(d)));
}

/// The runtime's clock in milliseconds: native builds and the interpreter
/// have their own; the web gives `Date.now`.
pub fn set_clock(f: fn() -> f64) {
    CLOCK.with(|c| c.set(f));
}

fn now() -> f64 {
    CLOCK.with(Cell::get)()
}

/// The runtime's clock (ms): what QDXSOUND's — and the media objects'
/// (media.rs) — positions run by.
pub fn clock_ms() -> f64 {
    now()
}

/// Plays on the runtime's sound device (QWAVE: media.rs); nothing
/// without one.
pub fn device_play(p: &SoundPlay) {
    if let Some(d) = device() {
        (d.play)(p);
    }
}

/// Stops what `id` plays on the sound device.
pub fn device_stop(id: &str) {
    if let Some(d) = device() {
        (d.stop)(id);
    }
}

/// The gain of what `id` plays on the sound device.
pub fn device_volume(id: &str, gain: f32) {
    if let Some(d) = device() {
        (d.volume)(id, gain);
    }
}

fn device() -> Option<SoundDevice> {
    DEVICE.with(Cell::get)
}

/// Volume (0 … 100, "a percentage" with 100 "normal volume") as a gain:
/// DirectSound attenuates in hundredths of a decibel and a percent is
/// 100 − dB — the corpus's `qdxsound2.bas` gives its volume scroll bar 70 …
/// 100, below which it's next to silence; 0 is silent.
pub fn volume_gain(volume: i64) -> f32 {
    if volume <= 0 {
        0.0
    } else {
        10f32.powf((volume.min(100) - 100) as f32 / 20.0)
    }
}

/// Pan (−100 … 100) as (left, right) gains: the other side attenuated by
/// |Pan| dB, as DirectSound pans — −100 mutes the right channel, 100 the
/// left (the manual).
pub fn pan_gains(pan: i64) -> (f32, f32) {
    let p = pan.clamp(-100, 100) as f32;
    let cut = |db: f32| if db >= 100.0 { 0.0 } else { 10f32.powf(-db / 20.0) };
    if p >= 0.0 { (cut(p), 1.0) } else { (1.0, cut(-p)) }
}

/// A QDXSOUND (manual, Appendix B; DelphiX's wave stream on a DirectSound
/// buffer): a WAV file played from Position — bytes into its sound, Size
/// the sound's bytes (what Position runs to: the manual's example gives a
/// track bar Max = Size, Position = Position) — at Frequency (the file's
/// rate once it's loaded), Volume and Pan, Looped or once. Playing and
/// Position follow the clock rather than the device, so they read the same
/// on every runtime and without a sound card; a sound played to its end
/// stops back at 0, as a DirectSound buffer's play cursor did.
#[derive(Debug, Clone)]
pub struct DxSound {
    pub file_name: String,
    pub wav: Option<Rc<Wav>>,
    pub frequency: i64,
    pub volume: i64,
    pub pan: i64,
    pub looped: bool,
    /// Bytes into the sound while stopped.
    position: i64,
    /// Playing since (clock ms), from (bytes).
    started: Option<(f64, i64)>,
}

impl Default for DxSound {
    fn default() -> Self {
        Self { file_name: String::new(), wav: None, frequency: 0, volume: 100, pan: 0, looped: false, position: 0, started: None }
    }
}

impl DxSound {
    /// Size: the sound's bytes.
    pub fn size(&self) -> i64 {
        self.wav.as_ref().map_or(0, |w| w.data.len() as i64)
    }

    fn block(&self) -> i64 {
        self.wav.as_ref().map_or(1, |w| w.block_align().max(1) as i64)
    }

    /// Where it plays now (bytes, on a frame), and whether it still plays.
    fn current(&mut self) -> (i64, bool) {
        let Some((since, from)) = self.started else { return (self.position, false) };
        let (size, block) = (self.size(), self.block());
        let frames = ((now() - since).max(0.0) * self.frequency.max(0) as f64 / 1000.0) as i64;
        let at = from + frames * block;
        if size > 0 && at >= size {
            if self.looped {
                return (at % size, true);
            }
            self.started = None;
            self.position = 0;
            return (0, false);
        }
        (at, true)
    }

    /// Plays from where it is (again, after a change while it plays).
    fn play(&mut self, id: &str) {
        let (at, _) = self.current();
        let Some(wav) = self.wav.clone() else { return };
        self.started = Some((now(), at));
        if let Some(d) = device() {
            let speed = self.frequency.max(1) as f64 / f64::from(wav.rate.max(1));
            let from = (at / self.block()) as usize;
            (d.play)(&SoundPlay { id: id.to_lowercase(), wav, from, speed, gain: volume_gain(self.volume), pan: pan_gains(self.pan), looped: self.looped });
        }
    }

    fn stop(&mut self, id: &str) {
        let (at, playing) = self.current();
        self.position = at;
        self.started = None;
        if let (Some(d), true) = (device(), playing) {
            (d.stop)(&id.to_lowercase());
        }
    }

    /// A WAV file's bytes as its sound (FileName); `Err` if it isn't one.
    pub fn load(&mut self, id: &str, name: &str, bytes: &[u8]) -> Result<(), String> {
        self.stop(id);
        self.file_name = name.to_string();
        let wav = parse_wav(bytes)?;
        // (loading a sound sets Frequency to the file's own rate, as
        // RapidQ's manual says)
        self.frequency = i64::from(wav.rate);
        self.wav = Some(Rc::new(wav));
        self.position = 0;
        Ok(())
    }

    pub fn get(&mut self, prop: &str) -> Option<Value> {
        Some(match prop {
            "filename" => crate::v_str(&self.file_name),
            "frequency" => v_int(self.frequency),
            "volume" => v_int(self.volume),
            "pan" => v_int(self.pan),
            "looped" => v_int(if self.looped { -1 } else { 0 }),
            "playing" => v_int(if self.current().1 { -1 } else { 0 }),
            "position" => v_int(self.current().0),
            "size" => v_int(self.size()),
            _ => return None,
        })
    }

    /// Sets a property (FileName: objects::set reads the file); a change
    /// while it plays is heard at once.
    pub fn set(&mut self, id: &str, prop: &str, val: &Value) -> Option<Result<(), String>> {
        let playing = self.current().1;
        match prop {
            "frequency" => self.frequency = val.to_i64().max(0),
            "volume" => {
                self.volume = val.to_i64().clamp(0, 100);
                if let (Some(d), true) = (device(), playing) {
                    (d.volume)(&id.to_lowercase(), volume_gain(self.volume));
                }
                return Some(Ok(()));
            }
            "pan" => self.pan = val.to_i64().clamp(-100, 100),
            "looped" => self.looped = val.to_bool(),
            "position" => {
                let block = self.block();
                let at = (val.to_i64().clamp(0, self.size()) / block) * block;
                if playing {
                    self.started = Some((now(), at));
                } else {
                    self.position = at;
                }
            }
            _ => return None,
        }
        if playing {
            self.play(id);
        }
        Some(Ok(()))
    }

    /// Play, Stop, Update (DirectSound's streaming: nothing to do).
    pub fn call(&mut self, id: &str, method: &str) -> Option<Value> {
        match method {
            "play" => self.play(id),
            "stop" => self.stop(id),
            // (DirectSound's buffer made again after the device lost it:
            // RapidR's sound never loses its buffer — nothing to do)
            "update" | "recreatebuf" => {}
            _ => return None,
        }
        Some(Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v_str;

    /// A one-picture library as DelphiX streams it: an 8-bit TDIB, 3 × 2,
    /// Transparent with clWhite.
    pub(crate) fn tiny_dxg() -> Vec<u8> {
        let mut dib = Vec::new();
        for v in [40u32, 3, 2] {
            dib.extend_from_slice(&v.to_le_bytes());
        }
        dib.extend_from_slice(&1u16.to_le_bytes());
        dib.extend_from_slice(&8u16.to_le_bytes());
        for v in [0u32, 8, 0, 0, 0, 0] {
            dib.extend_from_slice(&v.to_le_bytes());
        }
        let mut palette = vec![0u8; 1024];
        palette[4..8].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0]); // 1: white
        palette[8..12].copy_from_slice(&[0, 0, 0xFF, 0]); // 2: red (B, G, R)
        dib.extend_from_slice(&palette);
        // bottom row first: [1 2 1], then [2 1 2]
        dib.extend_from_slice(&[1, 2, 1, 0, 2, 1, 2, 0]);
        let mut data = vec![4u8];
        data.extend_from_slice(b"TDIB");
        data.extend_from_slice(&dib);
        let mut s = Vec::new();
        s.extend_from_slice(b"\xFF\x0A\x00DELPHIXPICTURECOLLECTION\x00\x30\x10\x00\x00\x00\x00TPF0");
        let name = |s: &mut Vec<u8>, n: &str| {
            s.push(n.len() as u8);
            s.extend_from_slice(n.as_bytes());
        };
        name(&mut s, "TPictureCollectionComponent");
        name(&mut s, "");
        name(&mut s, "List");
        s.push(14);
        s.push(1);
        name(&mut s, "Name");
        s.push(6);
        name(&mut s, "dot");
        name(&mut s, "PatternWidth");
        s.extend_from_slice(&[2, 0]);
        name(&mut s, "Picture.Data");
        s.push(10);
        s.extend_from_slice(&(data.len() as u32).to_le_bytes());
        s.extend_from_slice(&data);
        name(&mut s, "Transparent");
        s.push(9);
        name(&mut s, "TransparentColor");
        s.push(7);
        name(&mut s, "clWhite");
        s.extend_from_slice(&[0, 0, 0, 0]);
        s
    }

    #[test]
    fn dxg_pictures() {
        let p = parse_dxg(&tiny_dxg()).unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].name, "dot");
        let b = &p[0].picture;
        assert_eq!((b.img.width, b.img.height), (3, 2));
        // (top row first now; red is &H0000FF)
        assert_eq!(b.img.pixels, vec![0x0000FF, 0xFFFFFF, 0x0000FF, 0xFFFFFF, 0x0000FF, 0xFFFFFF]);
        assert!(b.transparent);
        assert_eq!(b.transparent_color, 0xFFFFFF);
        assert_eq!(p[0].pattern_rect(0), Some((0, 0, 3, 2)));
        assert_eq!(p[0].pattern_rect(1), None);
    }

    #[test]
    fn patterns() {
        let mut p = parse_dxg(&tiny_dxg()).unwrap().remove(0);
        p.pattern = (1, 1);
        assert_eq!(p.pattern_rect(4), Some((1, 1, 2, 2)));
        p.skip = (1, 0);
        assert_eq!(p.pattern_rect(1), Some((2, 0, 3, 1)));
        let part = pattern_bitmap(&mut p, 1).unwrap();
        assert_eq!(part.img.pixels, vec![0x0000FF]);
    }

    /// Nothing shows until Flip; the back buffer keeps its pixels after
    /// one; Fill takes DirectDraw's &HRRGGBB, the rest RapidQ's &HBBGGRR.
    #[test]
    fn flip_and_colors() {
        let mut s = DxScreen::default();
        s.call("init", &[v_int(4), v_int(3)]);
        assert!(initialize(&mut s, 100, 100, true));
        assert!(!initialize(&mut s, 100, 100, true));
        assert_eq!((s.back.img.width, s.back.img.height), (4, 3));
        s.call("fill", &[v_int(0xFF)]);
        s.call("pixel", &[v_int(1), v_int(1), v_int(0xFF)]);
        assert_eq!(s.call("pixel", &[v_int(0), v_int(0)]).unwrap().to_i64(), 0xFF0000, "&HFF is blue for Fill");
        assert_eq!(s.call("pixel", &[v_int(1), v_int(1)]).unwrap().to_i64(), 0xFF, "&HFF is red elsewhere");
        assert_eq!(s.front.pixel(0, 0), Some(0), "not shown before Flip");
        s.flip();
        assert_eq!(s.front.pixel(1, 1), Some(0xFF));
        assert_eq!(s.back.pixel(1, 1), Some(0xFF), "the back buffer keeps its pixels");
        assert_eq!(s.flips, 1);
        assert!(s.call("textwidth", &[v_str("Hi")]).unwrap().to_i64() > 0);
    }

    /// Drawn before the screen is set up: lost, as DirectX had no surface.
    #[test]
    fn drawn_before_initialize_is_cleared() {
        let mut s = DxScreen::default();
        s.call("init", &[v_int(2), v_int(2)]);
        s.call("pixel", &[v_int(0), v_int(0), v_int(0xFFFFFF)]);
        initialize(&mut s, 10, 10, true);
        assert_eq!(s.back.pixel(0, 0), Some(0));
        // Without Init the surface is the control's size.
        let mut t = DxScreen::default();
        initialize(&mut t, 7, 5, true);
        assert_eq!((t.back.img.width, t.back.img.height), (7, 5));
    }

    /// AutoSize: Init sizes the surface; a control resized after it (an
    /// Align set after Init) gives the surface its size, unless AutoSize
    /// is off; a control that keeps its size keeps Init's surface.
    #[test]
    fn autosize_follows_a_resized_control() {
        let mut s = DxScreen::default();
        s.follow_control(100, 100, true);
        s.call("init", &[v_int(94), v_int(183)]);
        initialize(&mut s, 183, 94, true);
        assert_eq!((s.back.img.width, s.back.img.height), (183, 94));
        s.follow_control(200, 50, true);
        assert_eq!((s.front.img.width, s.front.img.height), (200, 50));
        let mut kept = DxScreen::default();
        kept.follow_control(300, 300, true);
        kept.call("init", &[v_int(64), v_int(48)]);
        initialize(&mut kept, 300, 300, true);
        assert_eq!((kept.back.img.width, kept.back.img.height), (64, 48), "not resized since Init");
        let mut fixed = DxScreen::default();
        fixed.follow_control(100, 100, false);
        fixed.call("init", &[v_int(32), v_int(16)]);
        initialize(&mut fixed, 183, 94, false);
        assert_eq!((fixed.back.img.width, fixed.back.img.height), (32, 16), "AutoSize off");
    }

    /// Text in MS Sans Serif 8 by default; View.*; Rotate a quarter turn
    /// (64) clockwise about a point.
    #[test]
    fn font_view_rotate() {
        let mut s = DxScreen::default();
        assert_eq!(s.back.font.name, "MS Sans Serif");
        assert_eq!(s.back.font.size, 8);
        s.call("init", &[v_int(40), v_int(30)]);
        initialize(&mut s, 40, 30, true);
        assert_eq!(s.get("view.getfront").unwrap().to_f64(), 1.0);
        assert_eq!(s.call("view.getback", &[]).unwrap().to_f64(), 5000.0);
        s.call("view.setfront", &[Value::Double(10.5)]);
        assert_eq!(s.get("view.getfront").unwrap().to_f64(), 10.5);
        s.call("line", &[v_int(10), v_int(10), v_int(20), v_int(10), v_int(0xFF)]);
        s.call("rotate", &[v_int(10), v_int(10), v_int(64)]);
        assert_eq!(s.back.pixel(10, 18), Some(0xFF), "the line points down now");
        assert_eq!(s.back.pixel(18, 10), Some(0), "and no longer right");
        s.call("view.clear", &[]);
        assert_eq!(s.back.pixel(10, 18), Some(0));
    }

    #[test]
    fn picture_placement() {
        assert_eq!(picture_rect((100, 50), (200, 200), true, false), (0, 0, 200, 200));
        assert_eq!(picture_rect((100, 50), (200, 200), false, false), (0, 0, 100, 50));
        assert_eq!(picture_rect((640, 480), (1920, 1080), true, true), (240, 0, 1440, 1080), "4:3 on a 16:9 screen");
        assert!(DxTimer::fires(false, false) && DxTimer::fires(true, true) && !DxTimer::fires(true, false));
    }

    #[test]
    fn frame_rate() {
        let mut t = DxTimer::default();
        for i in 0..30 {
            t.tick(f64::from(i) * 40.0);
        }
        assert_eq!(t.rate, 26, "26 OnTimers in the first second (0 … 1000 ms)");
        assert_eq!(timer_interval_ms(0), DX_FRAME_MS);
        assert_eq!(timer_interval_ms(10), 10);
    }

    /// A WAV of 8-bit mono at 8000 Hz, `frames` long.
    pub(crate) fn tiny_wav(frames: usize) -> Vec<u8> {
        let mut w = Vec::new();
        w.extend_from_slice(b"RIFF");
        w.extend_from_slice(&((36 + frames) as u32).to_le_bytes());
        w.extend_from_slice(b"WAVEfmt ");
        w.extend_from_slice(&16u32.to_le_bytes());
        for v in [1u16, 1] {
            w.extend_from_slice(&v.to_le_bytes());
        }
        w.extend_from_slice(&8000u32.to_le_bytes());
        w.extend_from_slice(&8000u32.to_le_bytes());
        w.extend_from_slice(&1u16.to_le_bytes());
        w.extend_from_slice(&8u16.to_le_bytes());
        w.extend_from_slice(b"data");
        w.extend_from_slice(&(frames as u32).to_le_bytes());
        w.extend((0..frames).map(|i| if i % 2 == 0 { 0xFF } else { 0x00 }));
        w
    }

    thread_local! {
        static FAKE_NOW: Cell<f64> = const { Cell::new(0.0) };
        static PLAYED: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    fn fake_now() -> f64 {
        FAKE_NOW.with(Cell::get)
    }

    /// Size, Frequency from the file; Playing and Position by the clock;
    /// Stop keeps the place; the end stops back at 0 unless Looped; a
    /// change while it plays plays again from where it is; the device hears
    /// gains in dB.
    #[test]
    fn sound() {
        set_clock(fake_now);
        set_sound_device(SoundDevice {
            play: |p| PLAYED.with(|l| l.borrow_mut().push(format!("play {} from {} x{:.2} g{:.3} {:.2},{:.2}{}", p.id, p.from, p.speed, p.gain, p.pan.0, p.pan.1, if p.looped { " loop" } else { "" }))),
            volume: |id, g| PLAYED.with(|l| l.borrow_mut().push(format!("volume {id} {g:.3}"))),
            stop: |id| PLAYED.with(|l| l.borrow_mut().push(format!("stop {id}"))),
        });
        let wav = parse_wav(&tiny_wav(4000)).unwrap();
        assert_eq!((wav.channels, wav.rate, wav.bits, wav.frames()), (1, 8000, 8, 4000));
        assert_eq!(&wav.stereo((1.0, 0.5))[..4], &[127.0 / 128.0, 127.0 / 256.0, -1.0, -0.5]);
        let mut s = DxSound::default();
        s.load("Snd", "beep.wav", &tiny_wav(4000)).unwrap();
        assert_eq!((s.get("size").unwrap().to_i64(), s.get("frequency").unwrap().to_i64()), (4000, 8000));
        FAKE_NOW.with(|n| n.set(1000.0));
        s.call("Snd", "play");
        FAKE_NOW.with(|n| n.set(1250.0));
        assert_eq!((s.get("playing").unwrap().to_i64(), s.get("position").unwrap().to_i64()), (-1, 2000));
        s.call("Snd", "stop");
        FAKE_NOW.with(|n| n.set(5000.0));
        assert_eq!((s.get("playing").unwrap().to_i64(), s.get("position").unwrap().to_i64()), (0, 2000), "Stop keeps the place");
        s.call("Snd", "play");
        s.set("Snd", "volume", &v_int(80));
        s.set("Snd", "frequency", &v_int(16000));
        FAKE_NOW.with(|n| n.set(5100.0));
        assert_eq!(s.get("position").unwrap().to_i64(), 2000 + 1600, "at 16000 Hz: 1600 bytes in 100 ms");
        FAKE_NOW.with(|n| n.set(5200.0));
        assert_eq!((s.get("playing").unwrap().to_i64(), s.get("position").unwrap().to_i64()), (0, 0), "played to its end: stopped, back at 0");
        s.set("Snd", "looped", &v_int(-1));
        s.call("Snd", "play");
        FAKE_NOW.with(|n| n.set(5200.0 + 750.0));
        assert_eq!((s.get("playing").unwrap().to_i64(), s.get("position").unwrap().to_i64()), (-1, 12000 % 4000));
        s.set("Snd", "pan", &v_int(-100));
        assert_eq!(pan_gains(-100), (1.0, 0.0));
        assert_eq!(pan_gains(20).1, 1.0);
        assert!((volume_gain(80) - 0.1).abs() < 1e-6 && volume_gain(0) == 0.0 && volume_gain(100) == 1.0);
        let log = PLAYED.with(|l| l.borrow().clone());
        assert_eq!(log[0], "play snd from 0 x1.00 g1.000 1.00,1.00");
        assert_eq!(log[1], "stop snd");
        assert_eq!(log[2], "play snd from 2000 x1.00 g1.000 1.00,1.00");
        assert_eq!(log[3], "volume snd 0.100");
        assert_eq!(log[4], "play snd from 2000 x2.00 g0.100 1.00,1.00", "Frequency while it plays: again from where it is");
        assert!(log.last().unwrap().ends_with("1.00,0.00 loop"));
        assert!(parse_wav(b"RIFF\0\0\0\0WAVE").is_err());
        set_clock(default_clock);
        DEVICE.with(|c| c.set(None));
    }
}
