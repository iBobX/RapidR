//! The buttons of RapidQ's MESSAGEBOX and MESSAGEDLG (manual), shared by the
//! desktop runtime (FLTK dialogs) and the web runtime (browser dialogs) —
//! and what every runtime's message box shows: its caption, its icon
//! ([`icon_shapes`]) and its layout ([`message_layout`]).

use crate::objects::trackbar::Shape;

/// A dialog button: its label and the value the call returns when chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Button {
    pub label: &'static str,
    pub result: i64,
}

const fn b(label: &'static str, result: i64) -> Button {
    Button { label, result }
}

// Windows MessageBox results (IDOK …), returned by MESSAGEBOX.
const IDOK: i64 = 1;
const IDCANCEL: i64 = 2;
const IDABORT: i64 = 3;
const IDRETRY: i64 = 4;
const IDIGNORE: i64 = 5;
const IDYES: i64 = 6;
const IDNO: i64 = 7;

/// `MESSAGEBOX(text, title, flags)`: the buttons for `flags` (MB_OK = 0,
/// MB_OKCANCEL = 1, MB_ABORTRETRYIGNORE = 2, MB_YESNOCANCEL = 3, MB_YESNO =
/// 4, MB_RETRYCANCEL = 5; icon and default-button bits are ignored), the
/// affirmative choice first and the negative/cancelling one second.
pub fn message_box_buttons(flags: i64) -> Vec<Button> {
    match flags & 0xF {
        1 => vec![b("OK", IDOK), b("Cancel", IDCANCEL)],
        2 => vec![b("Retry", IDRETRY), b("Abort", IDABORT), b("Ignore", IDIGNORE)],
        3 => vec![b("Yes", IDYES), b("No", IDNO), b("Cancel", IDCANCEL)],
        4 => vec![b("Yes", IDYES), b("No", IDNO)],
        5 => vec![b("Retry", IDRETRY), b("Cancel", IDCANCEL)],
        _ => vec![b("OK", IDOK)],
    }
}

/// `MESSAGEDLG(text, msgType, msgButtons, helpContext)`: the buttons for the
/// OR-ed `mb*` flags (mbYes = 1, mbNo = 2, mbOK = 4, mbCancel = 8, mbAbort =
/// 32, mbRetry = 64, mbIgnore = 128, mbAll = 256; mbHelp = 16 has no help
/// system to open and is left out), returning the `mr*` values (mrOk = 1,
/// mrCancel = 2, mrAbort = 3, mrRetry = 4, mrIgnore = 5, mrYes = 6, mrNo = 7,
/// mrAll = 8). Affirmative choices come first.
pub fn message_dlg_buttons(flags: i64) -> Vec<Button> {
    let all = [
        (1, b("Yes", 6)),
        (4, b("OK", 1)),
        (64, b("Retry", 4)),
        (256, b("All", 8)),
        (2, b("No", 7)),
        (8, b("Cancel", 2)),
        (32, b("Abort", 3)),
        (128, b("Ignore", 5)),
    ];
    let buttons: Vec<Button> = all.iter().filter(|(bit, _)| flags & bit != 0).map(|(_, button)| *button).collect();
    if buttons.is_empty() {
        vec![b("OK", 1)]
    } else {
        buttons
    }
}

/// Title of a MESSAGEDLG box for its `mt*` type (mtWarning = 0, mtError = 1,
/// mtInformation = 2, mtConfirmation = 3, mtCustom = 4).
pub fn message_dlg_title(msg_type: i64) -> &'static str {
    match msg_type {
        0 => "Warning",
        1 => "Error",
        2 => "Information",
        3 => "Confirm",
        _ => "",
    }
}

/// A MESSAGEDLG box's caption, as Delphi's MessageDlg gives it: its type's
/// ([`message_dlg_title`]), and mtCustom's (or an unknown type's) the
/// application's title (`app_title`: Application.Title).
pub fn message_dlg_caption(msg_type: i64, app_title: &str) -> String {
    match message_dlg_title(msg_type) {
        "" => app_title.to_string(),
        t => t.to_string(),
    }
}

/// The picture a message box shows left of its text: Windows' MB_ICONxxx /
/// IDI_xxx, Delphi's mtWarning … mtConfirmation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MsgIcon {
    /// A yellow triangle with a black "!" (MB_ICONEXCLAMATION, mtWarning).
    Warning,
    /// A red circle with a white "X" (MB_ICONHAND / MB_ICONSTOP, mtError).
    Error,
    /// A white speech balloon with a blue "i" (MB_ICONASTERISK /
    /// MB_ICONINFORMATION, mtInformation).
    Information,
    /// A white speech balloon with a blue "?" (MB_ICONQUESTION,
    /// mtConfirmation).
    Question,
}

impl MsgIcon {
    /// What a screen reader calls it.
    pub fn name(self) -> &'static str {
        match self {
            MsgIcon::Warning => "Warning",
            MsgIcon::Error => "Error",
            MsgIcon::Information => "Information",
            MsgIcon::Question => "Question",
        }
    }

    /// As a number (a kernel dialog keeps it as a property).
    pub fn code(self) -> i64 {
        match self {
            MsgIcon::Warning => 0,
            MsgIcon::Error => 1,
            MsgIcon::Information => 2,
            MsgIcon::Question => 3,
        }
    }

    pub fn from_code(n: i64) -> Option<MsgIcon> {
        message_dlg_icon(n)
    }
}

/// `MESSAGEBOX`'s icon: its flags' MB_ICONMASK bits (&H10 hand, &H20
/// question, &H30 exclamation, &H40 asterisk; none without).
pub fn message_box_icon(flags: i64) -> Option<MsgIcon> {
    match flags & 0xF0 {
        0x10 => Some(MsgIcon::Error),
        0x20 => Some(MsgIcon::Question),
        0x30 => Some(MsgIcon::Warning),
        0x40 => Some(MsgIcon::Information),
        _ => None,
    }
}

/// `MESSAGEDLG`'s icon for its `mt*` type (mtCustom: none).
pub fn message_dlg_icon(msg_type: i64) -> Option<MsgIcon> {
    match msg_type {
        0 => Some(MsgIcon::Warning),
        1 => Some(MsgIcon::Error),
        2 => Some(MsgIcon::Information),
        3 => Some(MsgIcon::Question),
        _ => None,
    }
}

/// An icon's side (logical pixels): Windows' 32 × 32.
pub const ICON_SIZE: i64 = 32;

/// A message box's icon as vector shapes in its 32 × 32 box: what the
/// kernel draws (`Op::Shape`), and, as [`icon_svg`], the FLTK and web
/// runtimes. Drawn after Windows' classic icons (no artwork copied): a
/// soft shadow, the shape, its glyph.
pub fn icon_shapes(icon: MsgIcon) -> Vec<Shape> {
    const SHADOW: u32 = 0xA8A8A8;
    let shadowed = |points: Vec<(f64, f64)>, fill: u32, stroke: u32| {
        vec![
            Shape { points: points.iter().map(|(x, y)| (x + 1.5, y + 1.5)).collect(), fill: Some(SHADOW), stroke: None },
            Shape { points, fill: Some(fill), stroke: Some(stroke) },
        ]
    };
    let glyph = |points: Vec<(f64, f64)>, color: u32| Shape { points, fill: Some(color), stroke: None };
    match icon {
        MsgIcon::Error => {
            let mut out = shadowed(ellipse(15.5, 15.5, 14.0, 14.0, 40), 0xE81010, 0x800000);
            out.push(glyph(bar((10.0, 10.0), (21.0, 21.0), 1.9), 0xFFFFFF));
            out.push(glyph(bar((21.0, 10.0), (10.0, 21.0), 1.9), 0xFFFFFF));
            out
        }
        MsgIcon::Warning => {
            let mut out = shadowed(vec![(15.5, 1.5), (30.0, 27.5), (1.0, 27.5)], 0xFFE800, 0x000000);
            // (the "!": a bar narrowing downward, a dot)
            out.push(glyph(vec![(13.6, 8.5), (17.4, 8.5), (16.6, 19.5), (14.4, 19.5)], 0x000000));
            out.push(glyph(ellipse(15.5, 23.0, 2.0, 2.0, 12), 0x000000));
            out
        }
        MsgIcon::Information => {
            let mut out = shadowed(balloon(), 0xFFFFFF, 0x000000);
            // (a serif "i": its dot, the stem with a foot and a flag)
            out.push(glyph(ellipse(16.0, 5.8, 2.1, 2.1, 12), 0x0000F0));
            out.push(glyph(vec![(12.5, 9.5), (17.8, 9.5), (17.8, 17.5), (19.5, 17.5), (19.5, 19.5), (12.5, 19.5), (12.5, 17.5), (14.2, 17.5), (14.2, 11.5), (12.5, 11.5)], 0x0000F0));
            out
        }
        MsgIcon::Question => {
            let mut out = shadowed(balloon(), 0xFFFFFF, 0x000000);
            // (the "?": a hook over the top, a stem down, a dot)
            out.push(glyph(arc_band(16.0, 8.0, 2.6, 5.6, 160.0, 410.0, 16), 0x0000F0));
            out.push(glyph(vec![(17.0, 11.8), (19.2, 13.6), (17.5, 15.0), (17.5, 16.6), (14.5, 16.6), (14.5, 14.2)], 0x0000F0));
            out.push(glyph(ellipse(16.0, 19.6, 1.9, 1.9, 12), 0x0000F0));
            out
        }
    }
}

/// An ellipse as a polygon of `n` points.
fn ellipse(cx: f64, cy: f64, rx: f64, ry: f64, n: usize) -> Vec<(f64, f64)> {
    (0..n)
        .map(|i| {
            let a = i as f64 / n as f64 * std::f64::consts::TAU;
            (cx + rx * a.cos(), cy + ry * a.sin())
        })
        .collect()
}

/// A straight bar `half` wide on each side from `a` to `b`.
fn bar(a: (f64, f64), b: (f64, f64), half: f64) -> Vec<(f64, f64)> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = (dx * dx + dy * dy).sqrt().max(f64::EPSILON);
    let (nx, ny) = (-dy / len * half, dx / len * half);
    vec![(a.0 + nx, a.1 + ny), (b.0 + nx, b.1 + ny), (b.0 - nx, b.1 - ny), (a.0 - nx, a.1 - ny)]
}

/// The band between radii `r0` and `r1` around (cx, cy) from angle `from`
/// to `to` (degrees, clockwise on the screen: 0 right, 90 down).
fn arc_band(cx: f64, cy: f64, r0: f64, r1: f64, from: f64, to: f64, n: usize) -> Vec<(f64, f64)> {
    let at = |r: f64, i: usize| {
        let a = (from + (to - from) * i as f64 / n as f64).to_radians();
        (cx + r * a.cos(), cy + r * a.sin())
    };
    let mut out: Vec<(f64, f64)> = (0..=n).map(|i| at(r1, i)).collect();
    out.extend((0..=n).rev().map(|i| at(r0, i)));
    out
}

/// A speech balloon: an ellipse with a tail down to the left, one outline.
fn balloon() -> Vec<(f64, f64)> {
    let (cx, cy, rx, ry) = (16.0, 12.5, 14.5, 11.5);
    let mut out = Vec::new();
    for i in 0..48 {
        let deg = 7.5 * f64::from(i);
        // (the tail replaces the arc between 105° and 135°: down to the
        // lower left)
        if deg > 105.0 && deg < 135.0 {
            continue;
        }
        if deg == 135.0 {
            out.push((5.5, 30.5));
        }
        let a = deg.to_radians();
        out.push((cx + rx * a.cos(), cy + ry * a.sin()));
    }
    out
}

/// An icon as an SVG picture of `size` pixels (the FLTK and web runtimes
/// draw [`icon_shapes`] so).
pub fn icon_svg(icon: MsgIcon, size: i64) -> String {
    let mut svg = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" viewBox="0 0 32 32">"#);
    for s in icon_shapes(icon) {
        let points: Vec<String> = s.points.iter().map(|(x, y)| format!("{x:.2},{y:.2}")).collect();
        let fill = s.fill.map_or("none".to_string(), |c| format!("#{c:06x}"));
        let stroke = s.stroke.map_or(String::new(), |c| format!(r##" stroke="#{c:06x}" stroke-width="1""##));
        svg.push_str(&format!(r#"<polygon points="{}" fill="{fill}"{stroke}/>"#, points.join(" ")));
    }
    svg.push_str("</svg>");
    svg
}

/// A message's lines are wrapped at this width (logical pixels).
pub const WRAP: i64 = 420;

/// `text`'s lines (CR LF, LF or CR), each wrapped at word breaks to fit
/// `width` in `font` (as `text_size` measures: TextWidth's).
pub fn wrap(text: &str, font: &crate::objects::font::Font, width: i64) -> Vec<String> {
    use crate::objects::text::text_size;
    let mut out = Vec::new();
    for para in text.replace("\r\n", "\n").replace('\r', "\n").split('\n') {
        let mut line = String::new();
        for word in para.split(' ') {
            let candidate = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if !line.is_empty() && text_size(&candidate, font).0 > width {
                out.push(std::mem::take(&mut line));
                line = word.to_string();
            } else {
                line = candidate;
            }
        }
        out.push(line);
    }
    out
}

/// The mnemonic Windows gives a message box's button (`&Yes`).
pub fn button_caption(label: &str) -> String {
    match label {
        "Yes" | "No" | "Retry" | "Abort" | "Ignore" | "All" => format!("&{label}"),
        _ => label.to_string(),
    }
}

/// Where a message box's parts go (logical pixels), as Delphi's
/// MessageDlg lays them out (CreateMessageDialog, in MS Sans Serif's
/// dialog units): the icon at the top left, the text right of it, the
/// buttons centred under both.
#[derive(Clone, Debug, PartialEq)]
pub struct MessageLayout {
    /// The window's inside.
    pub size: (i64, i64),
    pub icon: Option<(i64, i64, i64, i64)>,
    /// The text's block: its lines from its top, `line_height` apart.
    pub text: (i64, i64, i64, i64),
    pub buttons: Vec<(i64, i64, i64, i64)>,
}

/// Delphi's MessageDlg metrics (8 / 10 / 4 dialog units, 50 × 14 buttons).
pub const MARGIN: (i64, i64) = (12, 13);
pub const SPACING: (i64, i64) = (15, 16);
pub const BUTTON: (i64, i64) = (75, 23);
pub const BUTTON_GAP: i64 = 6;

/// The layout for a text `text_w` × `text_h`, `buttons` buttons and an
/// icon or not.
pub fn message_layout(text_w: i64, text_h: i64, buttons: usize, icon: bool) -> MessageLayout {
    let (mx, my) = MARGIN;
    let (mut block_w, mut block_h) = (text_w, text_h);
    if icon {
        block_w += ICON_SIZE + SPACING.0;
        block_h = block_h.max(ICON_SIZE);
    }
    let n = buttons.max(1) as i64;
    let group = n * BUTTON.0 + (n - 1) * BUTTON_GAP;
    let w = block_w.max(group) + 2 * mx;
    let top = my + block_h + SPACING.1;
    let mut x = (w - group) / 2;
    let buttons = (0..buttons)
        .map(|_| {
            let r = (x, top, BUTTON.0, BUTTON.1);
            x += BUTTON.0 + BUTTON_GAP;
            r
        })
        .collect();
    MessageLayout {
        size: (w, top + BUTTON.1 + my),
        icon: icon.then_some((mx, my, ICON_SIZE, ICON_SIZE)),
        text: (mx + block_w - text_w, my, text_w, text_h),
        buttons,
    }
}

/// What closing the dialog without choosing means: Cancel/No/Abort if the
/// dialog has one, otherwise its only button.
pub fn dismissed(buttons: &[Button]) -> i64 {
    ["Cancel", "No", "Abort"]
        .iter()
        .find_map(|label| buttons.iter().find(|b| b.label == *label))
        .or(buttons.first())
        .map_or(0, |b| b.result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buttons_and_results() {
        let yes_no = message_box_buttons(4 | 32); // MB_YESNO | MB_ICONQUESTION
        assert_eq!(yes_no.iter().map(|b| (b.label, b.result)).collect::<Vec<_>>(), [("Yes", 6), ("No", 7)]);
        assert_eq!(message_box_buttons(0), vec![b("OK", 1)]);
        let dlg = message_dlg_buttons(1 | 2 | 8); // mbYes OR mbNo OR mbCancel
        assert_eq!(dlg.iter().map(|b| b.label).collect::<Vec<_>>(), ["Yes", "No", "Cancel"]);
        assert_eq!(dismissed(&dlg), 2); // closing = Cancel
        assert_eq!(dismissed(&message_box_buttons(4)), 7); // Yes/No: closing = No
        assert_eq!(message_dlg_buttons(0), vec![b("OK", 1)]);
        assert_eq!(message_dlg_title(1), "Error");
    }

    #[test]
    fn captions_and_icons() {
        // (Delphi: the type's caption; mtCustom's is Application.Title)
        assert_eq!(message_dlg_caption(3, "app"), "Confirm");
        assert_eq!(message_dlg_caption(4, "app"), "app");
        assert_eq!(message_dlg_icon(0), Some(MsgIcon::Warning));
        assert_eq!(message_dlg_icon(3), Some(MsgIcon::Question));
        assert_eq!(message_dlg_icon(4), None);
        // MB_ICONHAND &H10, MB_ICONQUESTION &H20, MB_ICONEXCLAMATION &H30,
        // MB_ICONASTERISK &H40, with the buttons' and default's bits around
        assert_eq!(message_box_icon(0x10 | 4), Some(MsgIcon::Error));
        assert_eq!(message_box_icon(0x20 | 0x100 | 3), Some(MsgIcon::Question));
        assert_eq!(message_box_icon(0x30), Some(MsgIcon::Warning));
        assert_eq!(message_box_icon(0x40 | 1), Some(MsgIcon::Information));
        assert_eq!(message_box_icon(4), None);
        for i in [MsgIcon::Warning, MsgIcon::Error, MsgIcon::Information, MsgIcon::Question] {
            assert_eq!(MsgIcon::from_code(i.code()), Some(i));
        }
    }

    #[test]
    fn icons_are_shapes_inside_their_box() {
        for icon in [MsgIcon::Warning, MsgIcon::Error, MsgIcon::Information, MsgIcon::Question] {
            let shapes = icon_shapes(icon);
            // (a shadow, the outlined shape, its glyph's parts)
            assert!(shapes.len() >= 3, "{icon:?}");
            assert!(shapes[1].stroke.is_some() && shapes[2].stroke.is_none());
            for s in &shapes {
                assert!(s.points.len() >= 3);
                assert!(s.points.iter().all(|(x, y)| (0.0..=32.0).contains(x) && (0.0..=32.0).contains(y)), "{icon:?} {:?}", s.points);
            }
            let svg = icon_svg(icon, 32);
            assert!(svg.starts_with("<svg") && svg.matches("<polygon").count() == shapes.len());
        }
        // the error's disc is red, the warning's triangle yellow, the
        // glyphs white, black and blue
        assert_eq!(icon_shapes(MsgIcon::Error)[1].fill, Some(0xE81010));
        assert_eq!(icon_shapes(MsgIcon::Error)[2].fill, Some(0xFFFFFF));
        assert_eq!(icon_shapes(MsgIcon::Warning)[1].fill, Some(0xFFE800));
        assert_eq!(icon_shapes(MsgIcon::Question)[2].fill, Some(0x0000F0));
    }

    #[test]
    fn layout_puts_the_text_right_of_the_icon() {
        // Delphi's metrics: margins 12 × 13, the icon's 32 + 15, buttons
        // 75 × 23 six apart, 16 under the text
        let l = message_layout(200, 26, 3, true);
        assert_eq!(l.icon, Some((12, 13, 32, 32)));
        assert_eq!(l.text, (12 + 32 + 15, 13, 200, 26));
        assert_eq!(l.size.0, 12 + 32 + 15 + 200 + 12);
        assert_eq!(l.buttons[0].1, 13 + 32 + 16);
        assert_eq!(l.size.1, 13 + 32 + 16 + 23 + 13);
        let group = l.buttons[2].0 + 75 - l.buttons[0].0;
        assert_eq!(group, 3 * 75 + 2 * 6);
        assert_eq!(l.buttons[0].0, (l.size.0 - group) / 2);
        // without an icon the text starts at the margin; a short text is as
        // wide as the buttons need
        let l = message_layout(20, 13, 2, false);
        assert_eq!((l.icon, l.text.0), (None, 12));
        assert_eq!(l.size.0, 2 * 75 + 6 + 24);
        // a tall text: the buttons under it
        let l = message_layout(100, 80, 1, true);
        assert_eq!(l.buttons[0].1, 13 + 80 + 16);
    }
}
