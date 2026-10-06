//! The desktop's looks (ROADMAP Phase 1B, "kernel themes"): what Windows
//! keeps as its system colours (GetSysColor) and its visual style, as one
//! table — [`Theme`] — that every component the UI kernel draws takes its
//! colours, metrics and glyph styles from, and so do the shared models it
//! draws (tab controls, track bars, scroll bars, list views, headers): as
//! Windows' own controls ask GetSysColor, they ask [`current`].
//!
//! - [`CLASSIC`]: Windows' classic look, RapidQ's — every program's unless
//!   it asks for another. Its tokens are the very colours the kernel drew
//!   before there were themes: an old program looks the same, pixel for
//!   pixel.
//! - [`MODERN`]: a flat look after Windows 11's (Fluent): rounded buttons
//!   and boxes with thin borders, an accent colour, check boxes and radio
//!   buttons filled with it, thin scroll bars, focus rings. Vector shapes
//!   only; no artwork.
//! - [`DARK`]: the modern look, dark.
//! - [`HIGH_CONTRAST`]: Windows' High Contrast Black palette — white on
//!   black, cyan selections, yellow for what's under the mouse and for the
//!   focus, green for what's disabled — every control framed, thick focus
//!   rings.
//!
//! A theme only changes how things are drawn: never a size, a place or a
//! font (a form's layout, ClientWidth, TextWidth and AutoSize are the same
//! in every theme), and a colour the program chose (Color, Font.Color) is
//! the program's in every theme, as in RapidQ.
//!
//! The current theme is the UI thread's ([`current`], [`set`]); `$THEME`,
//! `Application.Theme` and `RAPIDR_THEME` name one ([`choose`]). The web
//! runtime never sets it: it draws the shared models in the classic colours
//! (its DOM is styled by rrcss) until it moves onto the kernel.

use std::cell::Cell;

/// How a theme draws controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    /// Windows' classic 3D (DrawEdge, DrawFrameControl): raised buttons,
    /// sunken boxes, dotted focus rectangles, the pixel check mark, scroll
    /// bars of raised buttons.
    Classic,
    /// Flat (Fluent): rounded rectangles with thin borders, accent-filled
    /// check boxes and radio buttons, a vector check mark, thin scroll
    /// bars, focus rings.
    Fluent,
}

/// A look's colours (0xRRGGBB, as the kernel's ops take them), metrics
/// and style. Every theme defines every token — there is no default to
/// fall back on, so a theme can't quietly take another's colours
/// ([`Theme::colors`] lists them all).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    /// Its name, as `Application.Theme` reads it.
    pub name: &'static str,
    pub look: Look,
    /// A dark look (the system's window frames follow where they can).
    pub dark: bool,

    // ---- Windows' system colours (the names the classic look has) ----
    /// COLOR_BTNFACE: forms, buttons, panels, the menu bar.
    pub face: u32,
    /// COLOR_BTNHIGHLIGHT: a raised edge's lit side.
    pub light: u32,
    /// COLOR_BTNSHADOW: its shaded side.
    pub shadow: u32,
    /// COLOR_3DDKSHADOW: the outer shaded line.
    pub dark_shadow: u32,
    /// COLOR_WINDOW: text boxes, lists, trees, grids.
    pub window: u32,
    /// COLOR_WINDOWTEXT / COLOR_BTNTEXT: text whose colour the program
    /// didn't choose; arrows and glyphs.
    pub text: u32,
    /// COLOR_GRAYTEXT: what's disabled.
    pub gray_text: u32,
    /// COLOR_HIGHLIGHT / COLOR_HIGHLIGHTTEXT: selected text, items, cells.
    pub highlight: u32,
    pub highlight_text: u32,
    /// A selection without the focus (a tree's, a list view's).
    pub unfocused: u32,
    /// A directory tree's selection without the focus.
    pub unfocused_strong: u32,
    /// COLOR_HOTLIGHT: a tab under the mouse (HotTrack).
    pub hot_text: u32,
    /// A push button under the mouse (the classic look's is additive:
    /// Windows' classic buttons have no hover).
    pub hot: u32,
    /// COLOR_WINDOWFRAME: the default button's frame; a drop-down list's,
    /// an in-place editor's and a gauge's border.
    pub frame: u32,
    /// The focus rectangle's dots (classic), the focus ring (fluent).
    pub focus: u32,
    /// A QCOOLBTN that's Down (classic: Windows' dithered face).
    pub toggled: u32,
    /// COLOR_ACTIVECAPTION / COLOR_CAPTIONTEXT: an MDI child's title bar
    /// while it's active; COLOR_INACTIVECAPTION / …TEXT otherwise.
    pub caption: u32,
    pub caption_text: u32,
    pub inactive_caption: u32,
    pub inactive_caption_text: u32,
    /// COLOR_MENU / COLOR_MENUTEXT, COLOR_MENUHILIGHT and its text: the
    /// kernel-drawn menus (the in-window bar is the face).
    pub menu: u32,
    pub menu_text: u32,
    pub menu_highlight: u32,
    pub menu_highlight_text: u32,
    /// A tree's lines; a grid's lines and its fixed cells' lines.
    pub lines: u32,
    pub grid_lines: u32,
    pub fixed_lines: u32,
    /// The scroll bars' track, and the part of it held down.
    pub track: u32,
    pub track_pressed: u32,
    /// A track bar's channel and its edge, its thumb and the thumb's edge,
    /// a disabled thumb; its ticks, enabled and not.
    pub channel: u32,
    pub channel_edge: u32,
    pub slider: u32,
    pub slider_edge: u32,
    pub slider_disabled: u32,
    pub ticks: u32,
    pub ticks_disabled: u32,
    /// A list view's own: an item under the mouse (HotTrack), its grid
    /// lines, its border, its scroll bars' thumb (and held) and arrows, a
    /// check box's ink.
    pub view_hot: u32,
    pub view_grid: u32,
    pub view_border: u32,
    pub view_thumb: u32,
    pub view_thumb_held: u32,
    pub view_arrow: u32,
    pub view_check: u32,

    // ---- the fluent look's own ----
    /// The accent: the default button, checked boxes and radio buttons, a
    /// toggle that's down, a text box's focus line, progress, a track
    /// bar's value; text on it; under the mouse; pressed.
    pub accent: u32,
    pub accent_text: u32,
    pub accent_hot: u32,
    pub accent_pressed: u32,
    /// A button's fill: at rest, under the mouse, pressed, disabled.
    pub control: u32,
    pub control_hot: u32,
    pub control_pressed: u32,
    pub control_disabled: u32,
    /// A selected item of a list, a combo box's list, a menu (a soft fill
    /// with an accent mark beside it); its text.
    pub selected: u32,
    pub selected_text: u32,
    /// Controls' thin borders, under the mouse, and the strong ones (an
    /// unchecked box's rim, a text box's bottom line, the thin scroll
    /// bars' thumb and arrows).
    pub border: u32,
    pub border_hot: u32,
    pub border_strong: u32,

    // ---- metrics ----
    /// Corners' radius (fluent), in pixels.
    pub radius: f64,
    /// The focus ring's width (fluent), in pixels.
    pub focus_width: f64,
    /// A text box with the focus is ringed too, not only underlined in the
    /// accent (high contrast: the focus shows whatever it's on).
    pub ring_fields: bool,
}

/// Windows' classic look (RapidQ's): the colours the kernel always drew.
pub const CLASSIC: Theme = Theme {
    name: "classic",
    look: Look::Classic,
    dark: false,
    face: 0xF0F0F0,
    light: 0xFFFFFF,
    shadow: 0x808080,
    dark_shadow: 0x404040,
    window: 0xFFFFFF,
    text: 0x000000,
    gray_text: 0x808080,
    highlight: 0x0078D7,
    highlight_text: 0xFFFFFF,
    unfocused: 0xF0F0F0,
    unfocused_strong: 0xD0D0D0,
    hot_text: 0x003CB4,
    hot: 0xE5F1FB,
    frame: 0x000000,
    focus: 0x000000,
    toggled: 0xF8F8F8,
    caption: 0x0A246A,
    caption_text: 0xFFFFFF,
    inactive_caption: 0x808080,
    inactive_caption_text: 0xFFFFFF,
    menu: 0xF0F0F0,
    menu_text: 0x000000,
    menu_highlight: 0x0078D7,
    menu_highlight_text: 0xFFFFFF,
    lines: 0xA0A0A0,
    grid_lines: 0xC0C0C0,
    fixed_lines: 0x808080,
    track: 0xE6E6E6,
    track_pressed: 0x9A9A9A,
    channel: 0xE7EAEA,
    channel_edge: 0xA0A0A0,
    slider: 0x007AD9,
    slider_edge: 0x005A9E,
    slider_disabled: 0xCCCCCC,
    ticks: 0x808080,
    ticks_disabled: 0xC0C0C0,
    view_hot: 0xE5F3FF,
    view_grid: 0xF0F0F0,
    view_border: 0x828790,
    view_thumb: 0xCDCDCD,
    view_thumb_held: 0xA6A6A6,
    view_arrow: 0x606060,
    view_check: 0x333333,
    // (the fluent tokens: the classic look doesn't draw them; its nearest)
    accent: 0x0078D7,
    accent_text: 0xFFFFFF,
    accent_hot: 0x0078D7,
    accent_pressed: 0x0078D7,
    control: 0xF0F0F0,
    control_hot: 0xE5F1FB,
    control_pressed: 0xF0F0F0,
    control_disabled: 0xF0F0F0,
    selected: 0x0078D7,
    selected_text: 0xFFFFFF,
    border: 0x808080,
    border_hot: 0x808080,
    border_strong: 0x404040,
    radius: 0.0,
    focus_width: 1.0,
    ring_fields: false,
};

/// The modern look: Windows 11's light theme (Fluent), its translucent
/// fills as they show over a window's background.
pub const MODERN: Theme = Theme {
    name: "modern",
    look: Look::Fluent,
    dark: false,
    face: 0xF3F3F3,
    light: 0xFFFFFF,
    shadow: 0xD1D1D1,
    dark_shadow: 0x9A9A9A,
    window: 0xFFFFFF,
    text: 0x1B1B1B,
    gray_text: 0x8A8A8A,
    highlight: 0x0067C0,
    highlight_text: 0xFFFFFF,
    unfocused: 0xE6E6E6,
    unfocused_strong: 0xDADADA,
    hot_text: 0x005FB8,
    hot: 0xF6F6F6,
    frame: 0x8A8A8A,
    focus: 0x1B1B1B,
    toggled: 0x005FB8,
    caption: 0x005FB8,
    caption_text: 0xFFFFFF,
    inactive_caption: 0xEBEBEB,
    inactive_caption_text: 0x666666,
    menu: 0xF9F9F9,
    menu_text: 0x1B1B1B,
    menu_highlight: 0xEAEAEA,
    menu_highlight_text: 0x1B1B1B,
    lines: 0xC8C8C8,
    grid_lines: 0xE5E5E5,
    fixed_lines: 0xD1D1D1,
    track: 0xF9F9F9,
    track_pressed: 0xE6E6E6,
    channel: 0x8A8A8A,
    channel_edge: 0x8A8A8A,
    slider: 0x005FB8,
    slider_edge: 0xE5E5E5,
    slider_disabled: 0xC5C5C5,
    ticks: 0x8A8A8A,
    ticks_disabled: 0xC5C5C5,
    view_hot: 0xF0F0F0,
    view_grid: 0xEBEBEB,
    view_border: 0xD9D9D9,
    view_thumb: 0x8A8A8A,
    view_thumb_held: 0x5E5E5E,
    view_arrow: 0x8A8A8A,
    view_check: 0x5E5E5E,
    accent: 0x005FB8,
    accent_text: 0xFFFFFF,
    accent_hot: 0x196EBF,
    accent_pressed: 0x317CC6,
    control: 0xFDFDFD,
    control_hot: 0xF6F6F6,
    control_pressed: 0xF0F0F0,
    control_disabled: 0xF5F5F5,
    selected: 0xE5EEF8,
    selected_text: 0x1B1B1B,
    border: 0xD4D4D4,
    border_hot: 0xC4C4C4,
    border_strong: 0x8A8A8A,
    radius: 4.0,
    focus_width: 2.0,
    ring_fields: false,
};

/// The modern look, dark: Windows 11's dark theme.
pub const DARK: Theme = Theme {
    name: "dark",
    look: Look::Fluent,
    dark: true,
    face: 0x202020,
    light: 0x454545,
    shadow: 0x151515,
    dark_shadow: 0x0B0B0B,
    window: 0x2B2B2B,
    text: 0xFFFFFF,
    gray_text: 0x858585,
    highlight: 0x0078D4,
    highlight_text: 0xFFFFFF,
    unfocused: 0x3D3D3D,
    unfocused_strong: 0x454545,
    hot_text: 0x60CDFF,
    hot: 0x323232,
    frame: 0x9A9A9A,
    focus: 0xFFFFFF,
    toggled: 0x60CDFF,
    caption: 0x005A9E,
    caption_text: 0xFFFFFF,
    inactive_caption: 0x2E2E2E,
    inactive_caption_text: 0x9D9D9D,
    menu: 0x2C2C2C,
    menu_text: 0xFFFFFF,
    menu_highlight: 0x3D3D3D,
    menu_highlight_text: 0xFFFFFF,
    lines: 0x5A5A5A,
    grid_lines: 0x3A3A3A,
    fixed_lines: 0x4A4A4A,
    track: 0x262626,
    track_pressed: 0x3A3A3A,
    channel: 0x9A9A9A,
    channel_edge: 0x9A9A9A,
    slider: 0x60CDFF,
    slider_edge: 0x454545,
    slider_disabled: 0x5A5A5A,
    ticks: 0x9A9A9A,
    ticks_disabled: 0x5A5A5A,
    view_hot: 0x333333,
    view_grid: 0x3A3A3A,
    view_border: 0x3F3F3F,
    view_thumb: 0x9F9F9F,
    view_thumb_held: 0xC8C8C8,
    view_arrow: 0x9F9F9F,
    view_check: 0xC8C8C8,
    accent: 0x60CDFF,
    accent_text: 0x000000,
    accent_hot: 0x5BB9E6,
    accent_pressed: 0x56A6CD,
    control: 0x2D2D2D,
    control_hot: 0x353535,
    control_pressed: 0x272727,
    control_disabled: 0x2A2A2A,
    selected: 0x3A3F46,
    selected_text: 0xFFFFFF,
    border: 0x434343,
    border_hot: 0x505050,
    border_strong: 0x9A9A9A,
    radius: 4.0,
    focus_width: 2.0,
    ring_fields: false,
};

/// Windows' High Contrast Black: white on black, cyan selections, yellow
/// for what's under the mouse and the focus, green for what's disabled;
/// every control framed, thick focus rings.
pub const HIGH_CONTRAST: Theme = Theme {
    name: "highcontrast",
    look: Look::Fluent,
    dark: true,
    face: 0x000000,
    light: 0xFFFFFF,
    shadow: 0xFFFFFF,
    dark_shadow: 0xFFFFFF,
    window: 0x000000,
    text: 0xFFFFFF,
    gray_text: 0x3FF23F,
    highlight: 0x1AEBFF,
    highlight_text: 0x000000,
    unfocused: 0x404040,
    unfocused_strong: 0x404040,
    hot_text: 0xFFFF00,
    hot: 0x000000,
    frame: 0xFFFFFF,
    focus: 0xFFFF00,
    toggled: 0x1AEBFF,
    caption: 0x1AEBFF,
    caption_text: 0x000000,
    inactive_caption: 0x000000,
    inactive_caption_text: 0xFFFFFF,
    menu: 0x000000,
    menu_text: 0xFFFFFF,
    menu_highlight: 0x1AEBFF,
    menu_highlight_text: 0x000000,
    lines: 0xFFFFFF,
    grid_lines: 0xFFFFFF,
    fixed_lines: 0xFFFFFF,
    track: 0x000000,
    track_pressed: 0x1AEBFF,
    channel: 0xFFFFFF,
    channel_edge: 0xFFFFFF,
    slider: 0x1AEBFF,
    slider_edge: 0xFFFFFF,
    slider_disabled: 0x3FF23F,
    ticks: 0xFFFFFF,
    ticks_disabled: 0x3FF23F,
    view_hot: 0x000000,
    view_grid: 0xFFFFFF,
    view_border: 0xFFFFFF,
    view_thumb: 0xFFFFFF,
    view_thumb_held: 0xFFFF00,
    view_arrow: 0xFFFFFF,
    view_check: 0xFFFFFF,
    accent: 0x1AEBFF,
    accent_text: 0x000000,
    accent_hot: 0xFFFF00,
    accent_pressed: 0xFFFF00,
    control: 0x000000,
    control_hot: 0x000000,
    control_pressed: 0x1AEBFF,
    control_disabled: 0x000000,
    selected: 0x1AEBFF,
    selected_text: 0x000000,
    border: 0xFFFFFF,
    border_hot: 0xFFFF00,
    border_strong: 0xFFFFFF,
    radius: 4.0,
    focus_width: 3.0,
    ring_fields: true,
};

/// Every theme, the default first.
pub const ALL: [&Theme; 4] = [&CLASSIC, &MODERN, &DARK, &HIGH_CONTRAST];

/// What a `$THEME` / `Application.Theme` name asks for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Choice {
    Theme(&'static Theme),
    /// The system's: high contrast or dark when it is, else modern
    /// ([`auto`]).
    Auto,
    /// A name no theme has: the classic look (the runtime says so once).
    Unknown,
}

/// The theme a name asks for (case, spaces, `-` and `_` ignored). Besides
/// the four themes' own names, the names RapidR has accepted since `$THEME`
/// came (v2.62.0: fltk-theme's themes and schemes, FLTK's schemes,
/// platform names) go to the nearest look, and every name the classic look
/// answers to now (v2.114.0: `Classic`, `System`, `Light`, `Windows`,
/// `Win95`, `Win98`, `Win2K`) stays classic.
pub fn choose(name: &str) -> Choice {
    let n: String = name.chars().filter(|c| !matches!(c, ' ' | '-' | '_')).flat_map(char::to_lowercase).collect();
    Choice::Theme(match n.as_str() {
        "" | "classic" | "system" | "light" | "windows" | "win95" | "win98" | "win2k" | "windows95" | "windows98" | "win2000" | "windows2000" | "base" | "rapidq" => &CLASSIC,
        // flat and light: Windows 7 – 11, macOS, the Linux desktops,
        // fltk-theme's and FLTK's other looks
        "modern" | "fluent" | "win11" | "windows11" | "win10" | "windows10" | "metro" | "win8" | "windows8" | "aero" | "win7" | "windows7" | "aqua" | "aquaclassic" | "mac"
        | "macos" | "linux" | "greybird" | "xfce" | "gtk" | "gleam" | "clean" | "crystal" | "svg" | "sweet" | "fleet1" | "fleet2" | "plastic" | "oxy" | "blue" => &MODERN,
        "dark" | "darkmode" | "moderndark" | "night" => &DARK,
        "highcontrast" | "contrast" | "hc" | "highcontrastblack" => &HIGH_CONTRAST,
        // RapidR's own look (ROADMAP Phase 3B: the default to come), in its
        // three variants — light, dark, high contrast — today's modern,
        // dark and highcontrast; `rapidr` alone follows the system's setting
        "rapidrlight" => &MODERN,
        "rapidrdark" => &DARK,
        "rapidrhighcontrast" | "rapidrcontrast" => &HIGH_CONTRAST,
        "auto" | "rapidr" => return Choice::Auto,
        _ => return Choice::Unknown,
    })
}

/// `auto`'s theme for what the system says: high contrast, dark, else
/// the modern look.
pub fn auto(dark: bool, high_contrast: bool) -> &'static Theme {
    if high_contrast {
        &HIGH_CONTRAST
    } else if dark {
        &DARK
    } else {
        &MODERN
    }
}

thread_local! {
    /// The UI thread's theme (`None` until it's first asked for), and how
    /// many times it changed.
    static CURRENT: Cell<(Option<&'static Theme>, u64)> = const { Cell::new((None, 0)) };
    /// `auto` was asked for and the system hasn't said yet how it looks
    /// ([`wants_system`]).
    static WANTS_SYSTEM: Cell<bool> = const { Cell::new(false) };
}

/// The theme of a program that names none: the user's `RAPIDR_THEME`
/// (any name [`choose`] knows; `auto` the modern look until the system
/// answers, [`system_answer`]), else the classic look.
fn default_theme() -> &'static Theme {
    match choose(&std::env::var("RAPIDR_THEME").unwrap_or_default()) {
        Choice::Theme(t) => t,
        Choice::Auto => {
            WANTS_SYSTEM.with(|w| w.set(true));
            &MODERN
        }
        Choice::Unknown => &CLASSIC,
    }
}

/// The theme the desktop draws with now.
pub fn current() -> &'static Theme {
    CURRENT.with(|c| {
        let (now, generation) = c.get();
        now.unwrap_or_else(|| {
            let t = default_theme();
            c.set((Some(t), generation));
            t
        })
    })
}

/// Draws with `theme` from now on (what the program named: the system's
/// look is no longer waited for).
pub fn set(theme: &'static Theme) {
    let now = current();
    WANTS_SYSTEM.with(|w| w.set(false));
    if now != theme {
        CURRENT.with(|c| c.set((Some(theme), c.get().1 + 1)));
    }
}

/// Whether `auto` (`RAPIDR_THEME`'s) waits for the system's look: the
/// desktop runtime then answers with [`system_answer`].
pub fn wants_system() -> bool {
    current();
    WANTS_SYSTEM.with(Cell::get)
}

/// The system's look (dark, high contrast), for `auto`.
pub fn system_answer(dark: bool, high_contrast: bool) {
    if WANTS_SYSTEM.with(Cell::get) {
        set(auto(dark, high_contrast));
    }
}

/// A count that changes whenever the theme does (a host's windows follow).
pub fn generation() -> u64 {
    current();
    CURRENT.with(|c| c.get().1)
}

/// &HBBGGRR (RapidQ's colours, a bitmap's pixels) from 0xRRGGBB, and back.
pub const fn bgr(rgb: u32) -> u32 {
    (rgb & 0xFF) << 16 | (rgb & 0xFF00) | (rgb >> 16 & 0xFF)
}

/// A colour's relative luminance (WCAG 2.x), 0 (black) to 1 (white).
pub fn luminance(rgb: u32) -> f64 {
    let lin = |c: u32| {
        let c = f64::from(c & 0xFF) / 255.0;
        if c <= 0.039_28 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * lin(rgb >> 16) + 0.7152 * lin(rgb >> 8) + 0.0722 * lin(rgb)
}

/// WCAG 2.x's contrast ratio of two colours, 1 to 21.
pub fn contrast(a: u32, b: u32) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

impl Theme {
    pub fn fluent(&self) -> bool {
        self.look == Look::Fluent
    }

    /// Windows' system colour `index` (GetSysColor's COLOR_…, a Delphi
    /// TColor `&H80000000 + index` such as clBtnFace) in this theme, as
    /// 0xRRGGBB — what a program's `Color = clBtnFace` is drawn in and what
    /// Pixel then reads (RC.EXE on Windows 11: clBtnFace F0F0F0, clWindow
    /// FFFFFF). Colours the theme has no token for: Windows 10 / 11's.
    pub fn system_color(&self, index: u32) -> u32 {
        match index {
            0 => 0xC8C8C8,             // COLOR_SCROLLBAR
            1 => 0x000000,             // COLOR_BACKGROUND (the desktop)
            2 | 27 => self.caption,    // ACTIVECAPTION, GRADIENTACTIVECAPTION
            3 | 28 => self.inactive_caption,
            4 => self.menu,
            5 => self.window,
            6 => self.frame,
            7 => self.menu_text,
            8 | 18 => self.text,       // WINDOWTEXT, BTNTEXT
            9 => self.caption_text,
            10 => 0xB4B4B4,            // ACTIVEBORDER
            11 => 0xF4F7FC,            // INACTIVEBORDER
            12 => 0xABABAB,            // APPWORKSPACE
            13 => self.highlight,
            14 => self.highlight_text,
            15 | 30 => self.face,      // BTNFACE, MENUBAR
            16 => self.shadow,
            17 => self.gray_text,
            19 => self.inactive_caption_text,
            20 => self.light,          // BTNHIGHLIGHT
            21 => self.dark_shadow,
            22 => 0xE3E3E3,            // 3DLIGHT
            23 => 0x000000,            // INFOTEXT
            24 => 0xFFFFE1,            // INFOBK
            26 => self.hot_text,
            29 => self.menu_highlight,
            _ => 0x000000,
        }
    }

    /// Text the program left uncoloured, drawn on `background`: the theme's
    /// text — or, on a colour the program chose where that wouldn't read,
    /// black or white, whichever reads better. (The classic look keeps
    /// RapidQ's black everywhere, as RapidQ does.)
    pub fn text_on(&self, background: u32) -> u32 {
        if !self.fluent() || contrast(self.text, background) >= 3.0 {
            return self.text;
        }
        if contrast(0x000000, background) >= contrast(0xFFFFFF, background) { 0x000000 } else { 0xFFFFFF }
    }

    /// Every colour token by name (the tests check them all; a token added
    /// to [`Theme`] must be listed here or this doesn't build).
    pub fn colors(&self) -> Vec<(&'static str, u32)> {
        let Theme {
            name: _,
            look: _,
            dark: _,
            face,
            light,
            shadow,
            dark_shadow,
            window,
            text,
            gray_text,
            highlight,
            highlight_text,
            unfocused,
            unfocused_strong,
            hot_text,
            hot,
            frame,
            focus,
            toggled,
            caption,
            caption_text,
            inactive_caption,
            inactive_caption_text,
            menu,
            menu_text,
            menu_highlight,
            menu_highlight_text,
            lines,
            grid_lines,
            fixed_lines,
            track,
            track_pressed,
            channel,
            channel_edge,
            slider,
            slider_edge,
            slider_disabled,
            ticks,
            ticks_disabled,
            view_hot,
            view_grid,
            view_border,
            view_thumb,
            view_thumb_held,
            view_arrow,
            view_check,
            accent,
            accent_text,
            accent_hot,
            accent_pressed,
            control,
            control_hot,
            control_pressed,
            control_disabled,
            selected,
            selected_text,
            border,
            border_hot,
            border_strong,
            radius: _,
            focus_width: _,
            ring_fields: _,
        } = *self;
        vec![
            ("face", face),
            ("light", light),
            ("shadow", shadow),
            ("dark_shadow", dark_shadow),
            ("window", window),
            ("text", text),
            ("gray_text", gray_text),
            ("highlight", highlight),
            ("highlight_text", highlight_text),
            ("unfocused", unfocused),
            ("unfocused_strong", unfocused_strong),
            ("hot_text", hot_text),
            ("hot", hot),
            ("frame", frame),
            ("focus", focus),
            ("toggled", toggled),
            ("caption", caption),
            ("caption_text", caption_text),
            ("inactive_caption", inactive_caption),
            ("inactive_caption_text", inactive_caption_text),
            ("menu", menu),
            ("menu_text", menu_text),
            ("menu_highlight", menu_highlight),
            ("menu_highlight_text", menu_highlight_text),
            ("lines", lines),
            ("grid_lines", grid_lines),
            ("fixed_lines", fixed_lines),
            ("track", track),
            ("track_pressed", track_pressed),
            ("channel", channel),
            ("channel_edge", channel_edge),
            ("slider", slider),
            ("slider_edge", slider_edge),
            ("slider_disabled", slider_disabled),
            ("ticks", ticks),
            ("ticks_disabled", ticks_disabled),
            ("view_hot", view_hot),
            ("view_grid", view_grid),
            ("view_border", view_border),
            ("view_thumb", view_thumb),
            ("view_thumb_held", view_thumb_held),
            ("view_arrow", view_arrow),
            ("view_check", view_check),
            ("accent", accent),
            ("accent_text", accent_text),
            ("accent_hot", accent_hot),
            ("accent_pressed", accent_pressed),
            ("control", control),
            ("control_hot", control_hot),
            ("control_pressed", control_pressed),
            ("control_disabled", control_disabled),
            ("selected", selected),
            ("selected_text", selected_text),
            ("border", border),
            ("border_hot", border_hot),
            ("border_strong", border_strong),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_defines_every_token_and_its_own_name() {
        let names: Vec<_> = CLASSIC.colors().into_iter().map(|(n, _)| n).collect();
        for t in ALL {
            let colors = t.colors();
            assert_eq!(colors.iter().map(|(n, _)| *n).collect::<Vec<_>>(), names, "{}", t.name);
            assert!(colors.iter().all(|(_, c)| *c <= 0xFFFFFF), "{}", t.name);
            assert_eq!(choose(t.name), Choice::Theme(t));
            assert!(t.focus_width >= 1.0 && t.radius >= 0.0);
        }
        // (a theme isn't another one under a new name)
        for (i, a) in ALL.iter().enumerate() {
            for b in &ALL[i + 1..] {
                assert_ne!(a.colors(), b.colors(), "{} = {}", a.name, b.name);
            }
        }
    }

    #[test]
    fn text_reads_on_every_new_theme() {
        // WCAG 2.x AA (4.5:1) for text; high contrast: AAA (7:1). (The
        // classic look is Windows' as it was, and stays so.)
        for t in [&MODERN, &DARK, &HIGH_CONTRAST] {
            let need = if *t == HIGH_CONTRAST { 7.0 } else { 4.5 };
            let pairs = [
                ("text on face", t.text, t.face),
                ("text on window", t.text, t.window),
                ("text on control", t.text, t.control),
                ("highlight", t.highlight_text, t.highlight),
                ("menu", t.menu_text, t.menu),
                ("menu highlight", t.menu_highlight_text, t.menu_highlight),
                ("caption", t.caption_text, t.caption),
                ("selected", t.selected_text, t.selected),
            ];
            for (what, fg, bg) in pairs {
                assert!(contrast(fg, bg) >= need, "{}: {what} {:.2}", t.name, contrast(fg, bg));
            }
            // (a pressed button's text: the theme's, or what reads on it)
            for (what, fg, bg) in [("accent", t.accent_text, t.accent), ("pressed", t.text_on(t.control_pressed), t.control_pressed)] {
                assert!(contrast(fg, bg) >= need, "{}: {what} {:.2}", t.name, contrast(fg, bg));
            }
            // (what's disabled, the focus ring, the strong borders and the
            // accent show: 3:1, WCAG's non-text contrast)
            for (what, fg, bg) in [("disabled", t.gray_text, t.face), ("focus", t.focus, t.face), ("rim", t.border_strong, t.window), ("accent", t.accent, t.face)] {
                assert!(contrast(fg, bg) >= 3.0, "{}: {what} {:.2}", t.name, contrast(fg, bg));
            }
        }
        // (High Contrast Black: Windows' own colours)
        assert_eq!((HIGH_CONTRAST.face, HIGH_CONTRAST.text, HIGH_CONTRAST.highlight, HIGH_CONTRAST.hot_text, HIGH_CONTRAST.gray_text), (0, 0xFFFFFF, 0x1AEBFF, 0xFFFF00, 0x3FF23F));
        assert!(HIGH_CONTRAST.focus_width > MODERN.focus_width);
    }

    #[test]
    fn names_go_to_the_nearest_look() {
        for n in ["", "Classic", "SYSTEM", "light", "Windows", "win95", "Win98", "Win2K", "base"] {
            assert_eq!(choose(n), Choice::Theme(&CLASSIC), "{n}");
        }
        for n in ["modern", "Fluent", "win11", "metro", "aero", "aquaclassic", "greybird", "gleam", "plastic", "blue"] {
            assert_eq!(choose(n), Choice::Theme(&MODERN), "{n}");
        }
        assert_eq!(choose("Dark"), Choice::Theme(&DARK));
        for n in ["highcontrast", "High Contrast", "high-contrast", "HC"] {
            assert_eq!(choose(n), Choice::Theme(&HIGH_CONTRAST), "{n}");
        }
        assert_eq!(choose("auto"), Choice::Auto);
        assert_eq!(choose("RapidR"), Choice::Auto);
        assert_eq!(choose("rapidr light"), Choice::Theme(&MODERN));
        assert_eq!(choose("RapidR-Dark"), Choice::Theme(&DARK));
        assert_eq!(choose("rapidr high contrast"), Choice::Theme(&HIGH_CONTRAST));
        assert_eq!(choose("purple"), Choice::Unknown);
        assert_eq!((auto(false, false), auto(true, false), auto(true, true), auto(false, true)), (&MODERN, &DARK, &HIGH_CONTRAST, &HIGH_CONTRAST));
    }

    #[test]
    fn the_current_theme_and_text_on_the_programs_colours() {
        assert_eq!(current(), &CLASSIC);
        let g = generation();
        set(&DARK);
        assert_eq!((current().name, generation()), ("dark", g + 1));
        set(&DARK);
        assert_eq!(generation(), g + 1);
        // `auto` waiting for the system (RAPIDR_THEME=auto): its answer
        // decides; once answered, or a theme named, it's no longer asked.
        WANTS_SYSTEM.with(|w| w.set(true));
        assert!(wants_system());
        system_answer(false, true);
        assert_eq!((current(), wants_system()), (&HIGH_CONTRAST, false));
        system_answer(true, false);
        assert_eq!(current(), &HIGH_CONTRAST);
        set(&CLASSIC);
        // The classic look: RapidQ's black, whatever it's on.
        assert_eq!(CLASSIC.text_on(0x000000), 0x000000);
        // Dark: white on its own colours, black on a white the program chose.
        assert_eq!(DARK.text_on(DARK.face), 0xFFFFFF);
        assert_eq!(DARK.text_on(0xFFFFFF), 0x000000);
        assert_eq!(MODERN.text_on(0x000080), 0xFFFFFF);
        assert_eq!(bgr(0x0078D7), 0xD77800);
    }
}
