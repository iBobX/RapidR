//! The desktop's looks (ROADMAP Phase 1B, "kernel themes"): what Windows
//! keeps as its system colours (GetSysColor) and its visual style, as one
//! table — [`Theme`] — that every component the UI kernel draws takes its
//! colours, metrics and glyph styles from, and so do the shared models it
//! draws (tab controls, track bars, scroll bars, list views, headers): as
//! Windows' own controls ask GetSysColor, they ask [`current`].
//!
//! - [`RAPIDR`], [`RAPIDR_DARK`], [`RAPIDR_HIGH_CONTRAST`]: RapidR's own
//!   look — every program's unless it names another (`$THEME rapidr`, or
//!   none), the one RapidR Studio draws itself in: flat controls with thin
//!   borders and rounded corners, RapidR's blue ([`ACCENT`]) for what's
//!   chosen, focus rings, thin scroll bars, a light window frame; Inter
//!   for RapidQ's default font ([`Theme::ui_face`]). Light, dark and high
//!   contrast (Windows' High Contrast Black: white on black, cyan
//!   selections, yellow for what's under the mouse and the focus, green for
//!   what's disabled, every control framed, thick focus rings); `rapidr`
//!   alone follows the system's setting ([`Choice::Auto`]). The same on
//!   every OS and on the web. Vector shapes only; no artwork.
//! - [`CLASSIC`]: Windows' classic look, RapidQ's (`$THEME classic`). Its
//!   tokens are the very colours the kernel drew before there were themes:
//!   an old program looks as it always did, pixel for pixel.
//!
//! A theme only changes how things are drawn: never a size or a place (a
//! form's layout, its components' sizes, ClientWidth / ClientHeight are
//! the same in every theme; text measures in the face the theme draws it
//! in), and a colour the program chose (Color, Font.Color) is the program's
//! in every theme, as in RapidQ — Windows' system colours (clBtnFace,
//! clWindow …) are the theme's ([`Theme::system_color`]).
//!
//! The current theme is the UI thread's ([`current`], [`set`]); `$THEME`,
//! `Application.Theme` and `RAPIDR_THEME` name one ([`choose`]).

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
    /// A high-contrast look (the state said by shape and strong colour).
    pub contrast: bool,
    /// The face RapidQ's default font (MS Sans Serif, MS Shell Dlg, a font
    /// with no name) is drawn and measured in, and how much larger, its
    /// pixel size times this (`objects::text::family_name`,
    /// `Font::pixel_size`): `None` RapidR Sans at MS Sans Serif's sizes
    /// (the classic look). Fonts a program names otherwise (Arial, Courier
    /// New …) are the same in every theme.
    pub ui_face: Option<(&'static str, f64)>,

    // ---- Windows' system colours (the names the classic look has) ----
    /// COLOR_BTNFACE: forms, buttons, panels, the menu bar.
    pub face: u32,
    /// COLOR_BTNHIGHLIGHT: a raised edge's lit side.
    pub light: u32,
    /// COLOR_BTNSHADOW: its shaded side.
    pub shadow: u32,
    /// COLOR_3DDKSHADOW: the outer shaded line.
    pub dark_shadow: u32,
    /// COLOR_3DLIGHT: a raised edge's inner lit line (a push button's), a
    /// sunken one's inner lower line (a text box's, a check box's).
    pub light3d: u32,
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
    /// Elevation: what floats above a window — a menu, a drop-down list, a
    /// window on a web page — casts a soft shadow of this colour, at this
    /// alpha (of 255) where it's darkest; 0: no shadows (the classic look's
    /// raised edges, high contrast's frames say it instead).
    pub shadow_ink: u32,
    pub shadow_alpha: u8,
}

/// Windows' classic look (RapidQ's): its system colours as Windows 11 has
/// them, where RapidQ's programs run unthemed today (their captures are
/// tests/visual/rapidq: the 3D greys A0A0A0 / 696969 / E3E3E3, grey text
/// 6D6D6D, the default button's frame 646464).
pub const CLASSIC: Theme = Theme {
    name: "classic",
    look: Look::Classic,
    dark: false,
    contrast: false,
    ui_face: None,
    face: 0xF0F0F0,
    light: 0xFFFFFF,
    shadow: 0xA0A0A0,
    dark_shadow: 0x696969,
    light3d: 0xE3E3E3,
    window: 0xFFFFFF,
    text: 0x000000,
    gray_text: 0x6D6D6D,
    highlight: 0x0078D7,
    highlight_text: 0xFFFFFF,
    unfocused: 0xF0F0F0,
    unfocused_strong: 0xD0D0D0,
    hot_text: 0x003CB4,
    hot: 0xE5F1FB,
    frame: 0x646464,
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
    fixed_lines: 0x000000,
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
    shadow_ink: 0x000000,
    shadow_alpha: 0,
};

/// RapidR's brand palette (`design/brand/README.md`, the Pencil design
/// `design/rapidr-brand.pen`): every colour of RapidR's look is one of
/// these or a mix of two ([`mix`]); docs/theme-tokens.md maps each token.
pub mod brand {
    /// Text, dark tiles and backgrounds.
    pub const INK: u32 = 0x0E1525;
    /// Light backgrounds.
    pub const PAPER: u32 = 0xF6F8FC;
    /// The primary colour: what's chosen, the default button, the focus.
    pub const BLUE: u32 = 0x2F5BFF;
    /// Blue text on light; the accent pressed.
    pub const BLUE_DEEP: u32 = 0x1E3FD8;
    /// Blue text (and the accent) on Ink.
    pub const BLUE_ON_DARK: u32 = 0x6E93FF;
    /// Accents on dark (the brand gradient's end).
    pub const CYAN: u32 = 0x19C6E6;
    /// "Run": the Runtime, compiled programs.
    pub const AMBER: u32 = 0xFFB224;
    /// BASIC sources (graphics only), and its text colour on light.
    pub const TEAL: u32 = 0x12B48A;
    pub const TEAL_DEEP: u32 = 0x0B7B5E;
    /// Secondary text on light.
    pub const SLATE: u32 = 0x5B6478;
    /// Secondary text and hairlines on dark.
    pub const MIST: u32 = 0xC9D1E3;
    /// The design's board: Paper's darker step.
    pub const BOARD: u32 = 0xE9EDF5;
    pub const WHITE: u32 = 0xFFFFFF;
}

use brand::{BLUE, BLUE_DEEP, BLUE_ON_DARK, BOARD, INK, MIST, PAPER, SLATE, WHITE};

/// RapidR's blue (design/brand: "RapidR Blue"), the accent of RapidR's
/// look: what's chosen, the default button, checked boxes, the focus ring —
/// one token, every RapidR variant's accent derived from it.
pub const ACCENT: u32 = BLUE;

/// `a` toward `b` by `permille` / 1000, per channel (rounded).
pub const fn mix(a: u32, b: u32, permille: u32) -> u32 {
    let mut out = 0;
    let mut s = 0;
    while s < 24 {
        let (x, y) = ((a >> s) & 0xFF, (b >> s) & 0xFF);
        let v = if y >= x { x + ((y - x) * permille + 500) / 1000 } else { x - ((x - y) * permille + 500) / 1000 };
        out |= (v & 0xFF) << s;
        s += 8;
    }
    out
}

/// Ink lightened by `permille` toward white: the dark variant's surfaces,
/// one ramp from the brand's Ink.
const fn ink(permille: u32) -> u32 {
    mix(INK, WHITE, permille)
}

/// The face RapidR's look draws RapidQ's default font in, and its size
/// against MS Sans Serif's: Inter at MS Sans Serif's pixels (8 pt: 11).
/// Chosen by the clipping audit (`cargo run --release -p rapidr-designer
/// --example caption_audit`, docs/theme-tokens.md): of the RapidQ corpus'
/// 539 texts in components of a fixed size, 11 clip in RapidQ's own font,
/// 36 in Inter 11 px, 67 at 12 px, 89 at 13 px (`examples/`: 0, 0, 1, 1).
/// Inter's large x-height makes its 11 pixels read as Windows 11's Segoe
/// UI 9 pt.
pub const UI_FACE: Option<(&str, f64)> = Some(("Inter", 1.0));

/// RapidR's look, light: RapidR Studio's — Paper surfaces, white fields
/// and buttons on hairline borders, Ink text, RapidR Blue for what's
/// chosen and for the focus ring.
pub const RAPIDR: Theme = Theme {
    name: "rapidr light",
    look: Look::Fluent,
    dark: false,
    contrast: false,
    ui_face: UI_FACE,
    face: PAPER,
    light: WHITE,
    shadow: 0xD5DBE7,
    dark_shadow: 0xA3ACBE,
    light3d: 0xFBFCFE,
    window: WHITE,
    text: INK,
    gray_text: 0x858EA1,
    highlight: BLUE,
    highlight_text: WHITE,
    unfocused: 0xE3E8F1,
    unfocused_strong: 0xD5DCE8,
    hot_text: BLUE_DEEP,
    hot: 0xF0F3F9,
    frame: 0x9AA3B5,
    focus: BLUE,
    toggled: BLUE,
    // (a window's title bar: its surface, the title dimmed while it's not
    // the active one — the frame's shadow says which is in front)
    caption: 0xEEF1F7,
    caption_text: INK,
    inactive_caption: PAPER,
    inactive_caption_text: SLATE,
    menu: WHITE,
    menu_text: INK,
    menu_highlight: mix(WHITE, BLUE, 110),
    menu_highlight_text: INK,
    lines: MIST,
    grid_lines: 0xE6EAF2,
    fixed_lines: 0xD5DCE8,
    track: PAPER,
    track_pressed: BOARD,
    channel: 0xC3CAD9,
    channel_edge: 0xC3CAD9,
    slider: BLUE,
    slider_edge: 0xD5DCE8,
    slider_disabled: MIST,
    ticks: 0x8A93A6,
    ticks_disabled: MIST,
    view_hot: 0xF1F4FA,
    view_grid: 0xEBEEF4,
    view_border: 0xD5DCE8,
    view_thumb: 0xB4BCCC,
    view_thumb_held: 0x8A93A6,
    view_arrow: 0x8A93A6,
    view_check: SLATE,
    accent: BLUE,
    accent_text: WHITE,
    accent_hot: mix(BLUE, BLUE_DEEP, 500),
    accent_pressed: BLUE_DEEP,
    control: WHITE,
    control_hot: 0xF3F6FB,
    control_pressed: BOARD,
    control_disabled: PAPER,
    selected: mix(WHITE, BLUE, 120),
    selected_text: INK,
    border: 0xD5DCE8,
    border_hot: 0xBCC4D6,
    border_strong: 0x8A93A6,
    radius: 5.0,
    focus_width: 2.0,
    shadow_ink: INK,
    shadow_alpha: 38,
};

/// RapidR's look, dark: the brand's Ink as the ground — surfaces one ramp
/// lighter from it, Paper's Board for text, Blue on Dark for what's chosen.
pub const RAPIDR_DARK: Theme = Theme {
    name: "rapidr dark",
    look: Look::Fluent,
    dark: true,
    contrast: false,
    ui_face: UI_FACE,
    face: ink(45),
    light: ink(200),
    shadow: mix(INK, 0x000000, 300),
    dark_shadow: mix(INK, 0x000000, 600),
    light3d: ink(150),
    window: ink(15),
    text: BOARD,
    gray_text: 0x6B7590,
    highlight: BLUE,
    highlight_text: WHITE,
    unfocused: ink(150),
    unfocused_strong: ink(200),
    hot_text: BLUE_ON_DARK,
    hot: ink(90),
    frame: 0x7D879E,
    focus: BLUE_ON_DARK,
    toggled: BLUE_ON_DARK,
    caption: ink(70),
    caption_text: BOARD,
    inactive_caption: ink(45),
    inactive_caption_text: 0x8F99B0,
    menu: ink(90),
    menu_text: BOARD,
    menu_highlight: mix(ink(90), BLUE, 300),
    menu_highlight_text: WHITE,
    lines: ink(250),
    grid_lines: ink(110),
    fixed_lines: ink(180),
    track: ink(45),
    track_pressed: ink(120),
    channel: ink(300),
    channel_edge: ink(300),
    slider: BLUE_ON_DARK,
    slider_edge: ink(200),
    slider_disabled: ink(250),
    ticks: 0x7D879E,
    ticks_disabled: ink(250),
    view_hot: ink(70),
    view_grid: ink(110),
    view_border: ink(160),
    view_thumb: ink(330),
    view_thumb_held: ink(500),
    view_arrow: ink(400),
    view_check: MIST,
    accent: BLUE_ON_DARK,
    accent_text: INK,
    accent_hot: mix(BLUE_ON_DARK, WHITE, 120),
    accent_pressed: mix(BLUE_ON_DARK, INK, 150),
    control: ink(95),
    control_hot: ink(135),
    control_pressed: ink(65),
    control_disabled: ink(60),
    selected: mix(ink(15), BLUE, 300),
    selected_text: WHITE,
    border: ink(150),
    border_hot: ink(220),
    border_strong: 0x7D879E,
    radius: 5.0,
    focus_width: 2.0,
    shadow_ink: 0x000000,
    shadow_alpha: 140,
};

/// RapidR's look, high contrast: Windows' High Contrast Black — white on
/// black, cyan selections, yellow for what's under the mouse and the
/// focus, green for what's disabled; every control framed, thick focus
/// rings.
pub const RAPIDR_HIGH_CONTRAST: Theme = Theme {
    name: "rapidr high contrast",
    look: Look::Fluent,
    dark: true,
    contrast: true,
    ui_face: UI_FACE,
    face: 0x000000,
    light: 0xFFFFFF,
    shadow: 0xFFFFFF,
    dark_shadow: 0xFFFFFF,
    light3d: 0xFFFFFF,
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
    shadow_ink: 0x000000,
    shadow_alpha: 0,
};

/// Every theme, the default's variants first.
pub const ALL: [&Theme; 4] = [&RAPIDR, &RAPIDR_DARK, &RAPIDR_HIGH_CONTRAST, &CLASSIC];

/// What a `$THEME` / `Application.Theme` name asks for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Choice {
    Theme(&'static Theme),
    /// RapidR's look as the system is: high contrast or dark when it is,
    /// else light ([`auto`]) — what a program that names no theme gets.
    Auto,
    /// A name no theme has: the default (the runtime says so once).
    Unknown,
}

/// The theme a name asks for (case, spaces, `-` and `_` ignored):
///
/// - RapidR's look: `rapidr` (and `""`, `auto`: as the system is), `rapidr
///   light`, `rapidr dark`, `rapidr high contrast`. The names of the looks
///   it grew from stay theirs as aliases: `modern` (and `fluent`, the
///   platform names — Windows 7 – 11, macOS, the Linux desktops — and
///   fltk-theme's and FLTK's schemes, accepted since v2.62.0) its light
///   variant, `dark` its dark one, `highcontrast` (`high contrast`, `hc`)
///   its high-contrast one.
/// - The classic look, under every name it answered to before RapidR's
///   look was the default (v2.114.0): `classic`, `rapidq`, `system`,
///   `light`, `windows`, `win95`, `win98`, `win2k` (and their long forms),
///   `base`.
pub fn choose(name: &str) -> Choice {
    let n: String = name.chars().filter(|c| !matches!(c, ' ' | '-' | '_')).flat_map(char::to_lowercase).collect();
    Choice::Theme(match n.as_str() {
        "" | "rapidr" | "auto" => return Choice::Auto,
        "classic" | "rapidq" | "system" | "light" | "windows" | "win95" | "win98" | "win2k" | "windows95" | "windows98" | "win2000" | "windows2000" | "base" => &CLASSIC,
        "rapidrlight" | "modern" | "fluent" | "win11" | "windows11" | "win10" | "windows10" | "metro" | "win8" | "windows8" | "aero" | "win7" | "windows7"
        | "aqua" | "aquaclassic" | "mac" | "macos" | "linux" | "greybird" | "xfce" | "gtk" | "gleam" | "clean" | "crystal" | "svg" | "sweet" | "fleet1" | "fleet2"
        | "plastic" | "oxy" | "blue" => &RAPIDR,
        "rapidrdark" | "dark" | "darkmode" | "moderndark" | "night" => &RAPIDR_DARK,
        "rapidrhighcontrast" | "rapidrcontrast" | "highcontrast" | "contrast" | "hc" | "highcontrastblack" => &RAPIDR_HIGH_CONTRAST,
        _ => return Choice::Unknown,
    })
}

/// RapidR's look for what the system says: high contrast, dark, else
/// light.
pub fn auto(dark: bool, high_contrast: bool) -> &'static Theme {
    if high_contrast {
        &RAPIDR_HIGH_CONTRAST
    } else if dark {
        &RAPIDR_DARK
    } else {
        &RAPIDR
    }
}

thread_local! {
    /// The UI thread's theme (`None` until it's first asked for), and how
    /// many times it changed.
    static CURRENT: Cell<(Option<&'static Theme>, u64)> = const { Cell::new((None, 0)) };
    /// RapidR's look waits for the system to say how it looks
    /// ([`wants_system`]).
    static WANTS_SYSTEM: Cell<bool> = const { Cell::new(false) };
    /// The user's `RAPIDR_THEME` where a process has no environment (a
    /// browser page's, [`set_user_choice`]).
    static USER_CHOICE: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// The user's `RAPIDR_THEME` for a host without a process environment (a
/// web page's), before anything is drawn.
pub fn set_user_choice(name: Option<String>) {
    USER_CHOICE.with(|u| *u.borrow_mut() = name);
    // (a theme already taken as the default's: the user's instead)
    if CURRENT.with(|c| c.get().0.is_some()) && WANTS_SYSTEM.with(Cell::get) {
        let chosen = default_theme();
        if !matches!(choose(&USER_CHOICE.with(|u| u.borrow().clone()).unwrap_or_default()), Choice::Auto | Choice::Unknown) {
            WANTS_SYSTEM.with(|w| w.set(false));
        }
        switch(chosen);
    }
}

/// The theme of a program that names none: the user's `RAPIDR_THEME` (any
/// name [`choose`] knows), else RapidR's look as the system is — light
/// until the system answers ([`system_answer`]).
fn default_theme() -> &'static Theme {
    let name = USER_CHOICE.with(|u| u.borrow().clone()).or_else(|| std::env::var("RAPIDR_THEME").ok()).unwrap_or_default();
    match choose(&name) {
        Choice::Theme(t) => t,
        Choice::Auto | Choice::Unknown => {
            WANTS_SYSTEM.with(|w| w.set(true));
            &RAPIDR
        }
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

/// Draws with `theme` from now on (a theme the program named: the
/// system's look is no longer followed).
pub fn set(theme: &'static Theme) {
    current();
    WANTS_SYSTEM.with(|w| w.set(false));
    switch(theme);
}

fn switch(theme: &'static Theme) {
    if current() != theme {
        CURRENT.with(|c| c.set((Some(theme), c.get().1 + 1)));
    }
}

/// RapidR's look as the system is, from now on (`$THEME rapidr`): what
/// it says now — dark, high contrast — and what it says later
/// ([`system_answer`]).
pub fn follow_system(dark: bool, high_contrast: bool) {
    current();
    WANTS_SYSTEM.with(|w| w.set(true));
    switch(auto(dark, high_contrast));
}

/// Whether the theme follows the system's look (RapidR's look, named by
/// no theme or `rapidr`): the host then tells it how the system looks,
/// at the start and whenever that changes ([`system_answer`]).
pub fn wants_system() -> bool {
    current();
    WANTS_SYSTEM.with(Cell::get)
}

/// How the system looks (dark, high contrast), for a theme that follows
/// it.
pub fn system_answer(dark: bool, high_contrast: bool) {
    if wants_system() {
        switch(auto(dark, high_contrast));
    }
}

/// `f` measured as RapidQ measures, whatever the theme draws in: the
/// classic look's faces for the default font (RapidR Sans, MS Sans Serif's
/// metrics). What a component's size comes from — an AutoSize label's
/// Width and Height — so sizes never change with the theme (RC.EXE's
/// numbers in every look); the text is drawn in the theme's face.
pub fn rapidq_metrics<R>(f: impl FnOnce() -> R) -> R {
    drawn_in(&CLASSIC, f)
}

/// `f` with `theme` as the current one — what it measures and draws is
/// `theme`'s — and the current theme back after it (nothing follows the
/// change: no window is drawn again). A designer's preview in another look.
pub fn drawn_in<R>(theme: &'static Theme, f: impl FnOnce() -> R) -> R {
    current();
    let saved = CURRENT.with(Cell::get);
    CURRENT.with(|c| c.set((Some(theme), saved.1)));
    // (put back however `f` ends)
    struct Restore((Option<&'static Theme>, u64));
    impl Drop for Restore {
        fn drop(&mut self) {
            CURRENT.with(|c| c.set(self.0));
        }
    }
    let _restore = Restore(saved);
    f()
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

    /// The corners of what floats or frames a whole surface — menus,
    /// drop-down lists, tooltips, windows on the page — rounder than a
    /// control's ([`Theme::radius`]); 0 in the classic look.
    pub fn panel_radius(&self) -> f64 {
        if self.fluent() { (self.radius * 1.6).round() } else { 0.0 }
    }

    /// The icons' palette for this theme (`rapidr_icons`' palettes:
    /// `classic`, `modern` — RapidR's light —, `dark`, `highcontrast`).
    pub fn icon_palette(&self) -> &'static str {
        if self.look == Look::Classic {
            "classic"
        } else if self.contrast {
            "highcontrast"
        } else if self.dark {
            "dark"
        } else {
            "modern"
        }
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
            22 => self.light3d,        // 3DLIGHT
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
            contrast: _,
            ui_face: _,
            face,
            light,
            shadow,
            dark_shadow,
            light3d,
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
            shadow_ink,
            shadow_alpha: _,
        } = *self;
        vec![
            ("face", face),
            ("light", light),
            ("shadow", shadow),
            ("dark_shadow", dark_shadow),
            ("light3d", light3d),
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
            ("elevation", shadow_ink),
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

    /// Every pair of a theme's text and what it's drawn on, by name.
    fn text_pairs(t: &Theme) -> Vec<(&'static str, u32, u32)> {
        vec![
            ("text on face", t.text, t.face),
            ("text on window", t.text, t.window),
            ("text on control", t.text, t.control),
            ("text on control (hot)", t.text, t.control_hot),
            ("highlight", t.highlight_text, t.highlight),
            ("menu", t.menu_text, t.menu),
            ("menu highlight", t.menu_highlight_text, t.menu_highlight),
            ("caption", t.caption_text, t.caption),
            ("selected", t.selected_text, t.selected),
            ("accent", t.accent_text, t.accent),
            ("accent (hot)", t.accent_text, t.accent_hot),
            ("accent (pressed)", t.accent_text, t.accent_pressed),
            ("pressed", t.text_on(t.control_pressed), t.control_pressed),
            ("hot text on window", t.hot_text, t.window),
            ("hot text on face", t.hot_text, t.face),
            ("an unfocused selection", t.text, t.unfocused),
        ]
    }

    #[test]
    fn text_reads_on_every_theme() {
        // WCAG 2.x: AA (4.5:1) for text in every theme, AAA (7:1) in high
        // contrast.
        for t in ALL {
            let need = if t.contrast { 7.0 } else { 4.5 };
            for (what, fg, bg) in text_pairs(t) {
                // (the classic look is Windows' as it was: its selection,
                // white on 0078D7, is 4.497:1 — WCAG's 4.5 by a rounding)
                if t.look == Look::Classic && bg == 0x0078D7 {
                    assert!(contrast(fg, bg) >= 4.49, "{what}");
                    continue;
                }
                assert!(contrast(fg, bg) >= need, "{}: {what} {fg:06X} on {bg:06X}: {:.2}", t.name, contrast(fg, bg));
            }
            // (what's disabled, the focus ring, the strong borders and the
            // accent show: 3:1, WCAG's non-text contrast — RapidR's look)
            if t.fluent() {
                for (what, fg, bg) in [("disabled", t.gray_text, t.face), ("focus", t.focus, t.face), ("rim", t.border_strong, t.window), ("accent", t.accent, t.face)] {
                    assert!(contrast(fg, bg) >= 3.0, "{}: {what} {:.2}", t.name, contrast(fg, bg));
                }
                // (an inactive window's title: still text)
                assert!(contrast(t.inactive_caption_text, t.inactive_caption) >= need, "{}: inactive caption", t.name);
            }
        }
        // (High Contrast Black: Windows' own colours)
        let hc = &RAPIDR_HIGH_CONTRAST;
        assert_eq!((hc.face, hc.text, hc.highlight, hc.hot_text, hc.gray_text), (0, 0xFFFFFF, 0x1AEBFF, 0xFFFF00, 0x3FF23F));
        assert!(hc.focus_width > RAPIDR.focus_width);
        // (the classic look's inactive caption — Windows' white on grey,
        // 3.9:1 — is the one pair under AA, kept as Windows drew it)
        assert!(contrast(CLASSIC.inactive_caption_text, CLASSIC.inactive_caption) < 4.5);
    }

    #[test]
    fn one_accent() {
        assert_eq!((RAPIDR.accent, RAPIDR.highlight, RAPIDR.toggled, RAPIDR.slider), (ACCENT, ACCENT, ACCENT, ACCENT));
        assert_eq!(RAPIDR_DARK.highlight, ACCENT);
        assert_eq!((RAPIDR_DARK.accent, RAPIDR_DARK.focus), (brand::BLUE_ON_DARK, brand::BLUE_ON_DARK));
        // (the brand's: Ink text on Paper, the focus ring RapidR Blue)
        assert_eq!((RAPIDR.focus, RAPIDR.text, RAPIDR.face), (ACCENT, brand::INK, brand::PAPER));
        assert_eq!(mix(0x000000, 0xFFFFFF, 500), 0x808080);
        assert_eq!(mix(0x123456, 0xABCDEF, 0), 0x123456);
        assert_eq!(mix(0x123456, 0xABCDEF, 1000), 0xABCDEF);
    }

    #[test]
    fn names_go_to_the_nearest_look() {
        for n in ["", "rapidr", "RapidR", "auto"] {
            assert_eq!(choose(n), Choice::Auto, "{n}");
        }
        // (every name the classic look answered to stays classic)
        for n in ["Classic", "rapidq", "System", "Light", "Windows", "win95", "Win98", "Win2K", "base"] {
            assert_eq!(choose(n), Choice::Theme(&CLASSIC), "{n}");
        }
        for n in ["rapidr light", "RapidR-Light", "modern", "Fluent", "win11", "metro", "aero", "aquaclassic", "greybird", "gleam", "plastic", "blue"] {
            assert_eq!(choose(n), Choice::Theme(&RAPIDR), "{n}");
        }
        for n in ["rapidr dark", "RapidR_Dark", "Dark"] {
            assert_eq!(choose(n), Choice::Theme(&RAPIDR_DARK), "{n}");
        }
        for n in ["rapidr high contrast", "rapidr-high-contrast", "highcontrast", "High Contrast", "high-contrast", "HC"] {
            assert_eq!(choose(n), Choice::Theme(&RAPIDR_HIGH_CONTRAST), "{n}");
        }
        assert_eq!(choose("purple"), Choice::Unknown);
        assert_eq!((auto(false, false), auto(true, false), auto(true, true), auto(false, true)), (&RAPIDR, &RAPIDR_DARK, &RAPIDR_HIGH_CONTRAST, &RAPIDR_HIGH_CONTRAST));
        // (RapidR's look draws RapidQ's default font in Inter; the classic
        // look in RapidR Sans)
        assert_eq!(CLASSIC.ui_face, None);
        assert!(ALL.iter().filter(|t| t.fluent()).all(|t| t.ui_face.is_some_and(|(face, _)| face == "Inter")));
    }

    #[test]
    fn the_current_theme_follows_the_system_until_one_is_named() {
        // (a program that names no theme: RapidR's look, light until the
        // system says otherwise; RAPIDR_THEME isn't set under cargo test)
        if std::env::var_os("RAPIDR_THEME").is_none() {
            assert_eq!(current(), &RAPIDR);
            assert!(wants_system());
        }
        let g = generation();
        system_answer(true, false);
        assert_eq!(current(), &RAPIDR_DARK);
        system_answer(false, true);
        assert_eq!((current(), wants_system()), (&RAPIDR_HIGH_CONTRAST, true));
        // a theme named: the system no longer followed
        set(&CLASSIC);
        assert!(generation() > g);
        let g = generation();
        system_answer(true, false);
        assert_eq!((current(), generation(), wants_system()), (&CLASSIC, g, false));
        set(&CLASSIC);
        assert_eq!(generation(), g);
        // `$THEME rapidr`: followed again
        follow_system(false, false);
        assert_eq!((current(), wants_system()), (&RAPIDR, true));
        system_answer(true, false);
        assert_eq!(current(), &RAPIDR_DARK);
        set(&CLASSIC);
    }

    #[test]
    fn text_on_the_programs_colours() {
        // The classic look: RapidQ's black, whatever it's on.
        assert_eq!(CLASSIC.text_on(0x000000), 0x000000);
        // Dark: white on its own colours, black on a white the program chose.
        assert_eq!(RAPIDR_DARK.text_on(RAPIDR_DARK.face), RAPIDR_DARK.text);
        assert_eq!(RAPIDR_DARK.text_on(0xFFFFFF), 0x000000);
        assert_eq!(RAPIDR.text_on(0x000080), 0xFFFFFF);
        assert_eq!(bgr(0x0078D7), 0xD77800);
    }

    #[test]
    fn system_colours_are_the_themes() {
        // clBtnFace (15), clWindow (5), clWindowText (8), clHighlight (13)
        for t in ALL {
            assert_eq!((t.system_color(15), t.system_color(5), t.system_color(8), t.system_color(13)), (t.face, t.window, t.text, t.highlight), "{}", t.name);
        }
    }
}
