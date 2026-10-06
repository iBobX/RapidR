//! Keyboard and mouse events as RapidQ passes them, the same in both
//! runtimes (RapidQ manual, chapter 5 and the component tables):
//!
//! * `OnKeyDown(Key AS WORD, Shift AS INTEGER)`, `OnKeyUp(Key, Shift)`: the
//!   Windows virtual-key code (`A` = 65 whatever the case, arrows 37–40,
//!   F1 = 112, …) and the Shift state;
//! * `OnKeyPress(Key AS BYTE)`: the character typed (shifted, as typed),
//!   Enter 13, Backspace 8, Tab 9, Escape 27; keys that type nothing don't
//!   fire it;
//! * `OnMouseDown(Button, X, Y, Shift)`, `OnMouseUp(…)` and
//!   `OnMouseMove(X, Y, Shift)`: `mbLeft` 0, `mbRight` 1, `mbMiddle` 2; X
//!   and Y in the component.
//!
//! The Shift state is RAPIDQ.INC's `ssShift` 256, `ssCtrl` 16, `ssAlt` 1.
//! A key event goes to the focused component, then to its form; a mouse
//! event to the component under the mouse (the one pressed, while a button
//! is held).

use crate::{v_int, Value};

pub const SS_SHIFT: i64 = 256;
pub const SS_CTRL: i64 = 16;
pub const SS_ALT: i64 = 1;

/// The Shift state of modifier keys held.
pub fn shift_state(shift: bool, ctrl: bool, alt: bool) -> i64 {
    (if shift { SS_SHIFT } else { 0 }) | (if ctrl { SS_CTRL } else { 0 }) | (if alt { SS_ALT } else { 0 })
}

/// RapidQ's button: `mbLeft` 0, `mbRight` 1, `mbMiddle` 2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Left = 0,
    Right = 1,
    Middle = 2,
}

impl Button {
    /// From a DOM `MouseEvent.button` (0 left, 1 middle, 2 right).
    pub fn from_dom(b: i16) -> Button {
        match b {
            1 => Button::Middle,
            2 => Button::Right,
            _ => Button::Left,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mouse {
    Down,
    Up,
    Move,
}

impl Mouse {
    pub fn event(self) -> &'static str {
        match self {
            Mouse::Down => "onmousedown",
            Mouse::Up => "onmouseup",
            Mouse::Move => "onmousemove",
        }
    }

    /// The event's arguments: (Button, X, Y, Shift), or (X, Y, Shift) for
    /// OnMouseMove.
    pub fn args(self, button: Button, x: i64, y: i64, shift: i64) -> Vec<Value> {
        match self {
            Mouse::Move => vec![v_int(x), v_int(y), v_int(shift)],
            _ => vec![v_int(button as i64), v_int(x), v_int(y), v_int(shift)],
        }
    }
}

/// The virtual-key code of a key by its DOM name (`KeyboardEvent.key`,
/// with `code` telling the numeric keypad apart), or a single character.
pub fn vk_of_key(key: &str, code: &str) -> Option<i64> {
    if let Some(d) = code.strip_prefix("Numpad").and_then(|d| d.parse::<i64>().ok()) {
        return Some(96 + d);
    }
    let named = match key {
        "Backspace" => 8,
        "Tab" => 9,
        "Enter" => 13,
        "Shift" => 16,
        "Control" => 17,
        "Alt" => 18,
        "Pause" => 19,
        "CapsLock" => 20,
        "Escape" | "Esc" => 27,
        " " | "Spacebar" => 32,
        "PageUp" => 33,
        "PageDown" => 34,
        "End" => 35,
        "Home" => 36,
        "ArrowLeft" | "Left" => 37,
        "ArrowUp" | "Up" => 38,
        "ArrowRight" | "Right" => 39,
        "ArrowDown" | "Down" => 40,
        "PrintScreen" => 44,
        "Insert" => 45,
        "Delete" | "Del" => 46,
        "Meta" | "OS" => 91,
        "ContextMenu" => 93,
        "NumLock" => 144,
        "ScrollLock" => 145,
        _ => 0,
    };
    if named != 0 {
        return Some(named);
    }
    if let Some(n) = key.strip_prefix('F').and_then(|n| n.parse::<i64>().ok()).filter(|n| (1..=24).contains(n)) {
        return Some(111 + n);
    }
    let mut chars = key.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    vk_of_char(c)
}

/// The virtual-key code of the key that types `c` (US layout).
pub fn vk_of_char(c: char) -> Option<i64> {
    Some(match c {
        'a'..='z' => c.to_ascii_uppercase() as i64,
        'A'..='Z' | '0'..='9' => c as i64,
        ')' => 48,
        '!' => 49,
        '@' => 50,
        '#' => 51,
        '$' => 52,
        '%' => 53,
        '^' => 54,
        '&' => 55,
        '*' => 56,
        '(' => 57,
        ' ' => 32,
        ';' | ':' => 186,
        '=' | '+' => 187,
        ',' | '<' => 188,
        '-' | '_' => 189,
        '.' | '>' => 190,
        '/' | '?' => 191,
        '`' | '~' => 192,
        '[' | '{' => 219,
        '\\' | '|' => 220,
        ']' | '}' => 221,
        '\'' | '"' => 222,
        _ => return None,
    })
}

/// OnKeyPress's Key for a key that typed `text` (virtual-key `vk`): the
/// character's code, or Enter / Backspace / Tab / Escape's; `None` if the
/// key types nothing (arrows, F-keys, modifiers).
pub fn press_code(vk: i64, text: &str) -> Option<i64> {
    if matches!(vk, 8 | 9 | 13 | 27) {
        return Some(vk);
    }
    let mut chars = text.chars();
    let c = chars.next()?;
    if chars.next().is_some() || (c as u32) < 32 || c as u32 == 127 {
        return None;
    }
    Some(if (c as u32) < 256 { c as i64 } else { '?' as i64 })
}

/// For tests: what typing the key `vk` without modifiers types.
pub fn text_of_vk(vk: i64) -> String {
    match vk {
        65..=90 => ((vk as u8 + 32) as char).to_string(),
        48..=57 | 32 => (vk as u8 as char).to_string(),
        _ => String::new(),
    }
}

/// A mouse pointer: RAPIDQ.INC's `crDefault` 0, `crNone` -1, `crArrow` -2,
/// `crCross` -3, `crIBeam` -4, `crSize` -5, `crSizeNESW` -6, `crSizeNS` -7,
/// `crSizeNWSE` -8, `crSizeWE` -9, `crUpArrow` -10, `crHourGlass` -11,
/// `crDrag` -12, `crNoDrop` -13, `crHSplit` -14, `crVSplit` -15,
/// `crMultiDrag` -16, `crSQLWait` -17, `crNo` -18, `crAppStart` -19,
/// `crHelp` -20, `crHandPoint` -21.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cursor {
    Default,
    None,
    Arrow,
    Cross,
    IBeam,
    Move,
    SizeNESW,
    SizeNS,
    SizeNWSE,
    SizeWE,
    UpArrow,
    Wait,
    NoDrop,
    Help,
    Hand,
    Progress,
}

impl Cursor {
    pub fn of(code: i64) -> Cursor {
        match code {
            -1 => Cursor::None,
            -2 => Cursor::Arrow,
            -3 => Cursor::Cross,
            -4 => Cursor::IBeam,
            -5 => Cursor::Move,
            -6 => Cursor::SizeNESW,
            -7 | -15 => Cursor::SizeNS,
            -8 => Cursor::SizeNWSE,
            -9 | -14 => Cursor::SizeWE,
            -10 => Cursor::UpArrow,
            -11 | -17 => Cursor::Wait,
            -12 | -16 => Cursor::Arrow,
            -13 | -18 => Cursor::NoDrop,
            -19 => Cursor::Progress,
            -20 => Cursor::Help,
            -21 => Cursor::Hand,
            _ => Cursor::Default,
        }
    }

    /// The CSS cursor.
    pub fn css(self) -> &'static str {
        match self {
            Cursor::Default | Cursor::Arrow => "default",
            Cursor::None => "none",
            Cursor::Cross => "crosshair",
            Cursor::IBeam => "text",
            Cursor::Move => "move",
            Cursor::SizeNESW => "nesw-resize",
            Cursor::SizeNS => "ns-resize",
            Cursor::SizeNWSE => "nwse-resize",
            Cursor::SizeWE => "ew-resize",
            Cursor::UpArrow => "n-resize",
            Cursor::Wait => "wait",
            Cursor::NoDrop => "not-allowed",
            Cursor::Help => "help",
            Cursor::Hand => "pointer",
            Cursor::Progress => "progress",
        }
    }
}

/// Who gets a key's OnKeyDown / OnKeyPress / OnKeyUp, as RapidQ (Delphi)
/// sends them: `chain` is the focused component up to its form. The form
/// gets the keys typed in its controls only with KeyPreview on — and then
/// first, before the focused control (RapidQ's manual, QFORM's
/// KeyPreview); with no control focused, the form itself.
pub fn key_targets<'a>(chain: &'a [String], key_preview: impl Fn(&str) -> bool) -> Vec<&'a String> {
    match (chain.first(), chain.last()) {
        (Some(control), Some(form)) if chain.len() > 1 && control != form => {
            if key_preview(form) {
                vec![form, control]
            } else {
                vec![control]
            }
        }
        (Some(only), _) => vec![only],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursors() {
        assert_eq!(Cursor::of(0), Cursor::Default);
        assert_eq!(Cursor::of(-21).css(), "pointer");
        assert_eq!(Cursor::of(-11).css(), "wait");
        assert_eq!(Cursor::of(-4).css(), "text");
        assert_eq!(Cursor::of(-15), Cursor::SizeNS);
        assert_eq!(Cursor::of(5), Cursor::Default);
    }

    #[test]
    fn keys() {
        assert_eq!(vk_of_key("a", "KeyA"), Some(65));
        assert_eq!(vk_of_key("A", "KeyA"), Some(65));
        assert_eq!(vk_of_key("ArrowUp", "ArrowUp"), Some(38));
        assert_eq!(vk_of_key("F5", "F5"), Some(116));
        assert_eq!(vk_of_key("7", "Numpad7"), Some(103));
        assert_eq!(vk_of_key("!", "Digit1"), Some(49));
        assert_eq!(vk_of_key("Dead", ""), None);
        assert_eq!(press_code(65, "A"), Some(65));
        assert_eq!(press_code(65, "a"), Some(97));
        assert_eq!(press_code(13, "\r"), Some(13));
        assert_eq!(press_code(38, ""), None);
        assert_eq!(shift_state(true, true, false), 272);
    }

    #[test]
    fn mouse_arguments() {
        let a = Mouse::Down.args(Button::from_dom(2), 3, 4, SS_CTRL);
        assert_eq!(a.iter().map(Value::to_i64).collect::<Vec<_>>(), vec![1, 3, 4, 16]);
        let m = Mouse::Move.args(Button::Left, 3, 4, 0);
        assert_eq!(m.iter().map(Value::to_i64).collect::<Vec<_>>(), vec![3, 4, 0]);
    }
}
