//! `Application.SendKeys` (RapidR's; Visual Basic's notation): keystrokes
//! a program sends to its own windows, queued here and delivered by the
//! host one per turn of its loop to the frontmost window that takes input,
//! through the UI kernel's keyboard path — exactly as the user's keys go
//! (focus, shortcuts, the focused component's key handling, OnKeyDown /
//! OnKeyPress / OnKeyUp). RapidR Studio's scripted flows type with it; a
//! program can drive its own forms (a demo, a macro, a test).
//!
//! The notation: each character types itself; `+` Shift, `^` Ctrl, `%` Alt
//! hold for the next key or for a group in parentheses (`+(ab)`); `~` is
//! Enter; `{NAME}` names a key (`{TAB}`, `{ENTER}`, `{BS}`, `{DEL}`, `{ESC}`,
//! `{UP}` `{DOWN}` `{LEFT}` `{RIGHT}`, `{HOME}` `{END}`, `{PGUP}` `{PGDN}`,
//! `{INS}`, `{SPACE}`, `{F1}`…`{F16}`), `{+}` `{^}` `{%}` `{~}` `{(}` `{)}`
//! `{{}` `{}}` type those characters, and `{NAME n}` presses a key n times.

use std::cell::RefCell;
use std::collections::VecDeque;

/// One keystroke: the virtual key, what it types, the modifiers held.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stroke {
    pub vk: i64,
    pub text: String,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

thread_local! {
    static QUEUE: RefCell<VecDeque<Stroke>> = const { RefCell::new(VecDeque::new()) };
}

/// The keystrokes of `keys`, queued for the windows; an error (and nothing
/// queued) for a malformed string.
pub fn send(keys: &str) -> Result<usize, String> {
    let strokes = parse(keys)?;
    let n = strokes.len();
    QUEUE.with(|q| q.borrow_mut().extend(strokes));
    Ok(n)
}

/// How many keystrokes wait to be delivered.
pub fn pending() -> usize {
    QUEUE.with(|q| q.borrow().len())
}

/// The next keystroke to deliver (the host's).
pub fn next() -> Option<Stroke> {
    QUEUE.with(|q| q.borrow_mut().pop_front())
}

/// The keystrokes queued dropped (the program ends, a test resets).
pub fn clear() {
    QUEUE.with(|q| q.borrow_mut().clear());
}

/// A key's name inside braces, as its virtual key (any case).
fn named(name: &str) -> Option<i64> {
    let n = name.to_ascii_uppercase();
    Some(match n.as_str() {
        "TAB" => 9,
        "ENTER" | "RETURN" => 13,
        "BS" | "BKSP" | "BACKSPACE" => 8,
        "DEL" | "DELETE" => 46,
        "ESC" | "ESCAPE" => 27,
        "UP" => 38,
        "DOWN" => 40,
        "LEFT" => 37,
        "RIGHT" => 39,
        "HOME" => 36,
        "END" => 35,
        "PGUP" => 33,
        "PGDN" => 34,
        "INS" | "INSERT" => 45,
        "SPACE" => 32,
        _ => match n.strip_prefix('F').and_then(|d| d.parse::<i64>().ok()) {
            Some(f @ 1..=16) => 111 + f,
            _ => return None,
        },
    })
}

/// The virtual key a character's key has on a US keyboard, and whether
/// Shift makes it.
pub fn key_of_char(c: char) -> (i64, bool) {
    match c {
        'a'..='z' => (c.to_ascii_uppercase() as i64, false),
        'A'..='Z' => (c as i64, true),
        '0'..='9' | ' ' => (c as i64, false),
        '\n' | '\r' => (13, false),
        '\t' => (9, false),
        ')' => (48, true),
        '!' => (49, true),
        '@' => (50, true),
        '#' => (51, true),
        '$' => (52, true),
        '%' => (53, true),
        '^' => (54, true),
        '&' => (55, true),
        '*' => (56, true),
        '(' => (57, true),
        ';' => (186, false),
        ':' => (186, true),
        '=' => (187, false),
        '+' => (187, true),
        ',' => (188, false),
        '<' => (188, true),
        '-' => (189, false),
        '_' => (189, true),
        '.' => (190, false),
        '>' => (190, true),
        '/' => (191, false),
        '?' => (191, true),
        '`' => (192, false),
        '~' => (192, true),
        '[' => (219, false),
        '{' => (219, true),
        '\\' => (220, false),
        '|' => (220, true),
        ']' => (221, false),
        '}' => (221, true),
        '\'' => (222, false),
        '"' => (222, true),
        // (any other character: typed as itself, no key of its own)
        _ => (0, false),
    }
}

#[derive(Clone, Copy, Default)]
struct Held {
    shift: bool,
    ctrl: bool,
    alt: bool,
}

fn stroke_char(c: char, held: Held) -> Stroke {
    let (vk, shifted) = key_of_char(c);
    let shift = held.shift || shifted;
    // (a shortcut types nothing; Enter and Tab type no text, as the
    // hosts send them)
    let text = if held.ctrl || held.alt || matches!(vk, 9 | 13) {
        String::new()
    } else if held.shift && c.is_ascii_lowercase() {
        c.to_ascii_uppercase().to_string()
    } else {
        c.to_string()
    };
    Stroke { vk, text, shift, ctrl: held.ctrl, alt: held.alt }
}

fn stroke_key(vk: i64, held: Held) -> Stroke {
    let text = if vk == 32 && !held.ctrl && !held.alt { " ".to_string() } else { String::new() };
    Stroke { vk, text, shift: held.shift, ctrl: held.ctrl, alt: held.alt }
}

/// `keys` read as keystrokes.
pub fn parse(keys: &str) -> Result<Vec<Stroke>, String> {
    let chars: Vec<char> = keys.chars().collect();
    let mut out = Vec::new();
    let mut held = Held::default();
    // (the modifiers held for a parenthesized group)
    let mut group: Option<Held> = None;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        i += 1;
        let current = group.map_or(held, |g| Held { shift: g.shift || held.shift, ctrl: g.ctrl || held.ctrl, alt: g.alt || held.alt });
        match c {
            '+' => {
                held.shift = true;
                continue;
            }
            '^' => {
                held.ctrl = true;
                continue;
            }
            '%' => {
                held.alt = true;
                continue;
            }
            '(' => {
                if group.is_some() {
                    return Err("SendKeys: a group inside a group".into());
                }
                group = Some(held);
                held = Held::default();
                continue;
            }
            ')' => {
                if group.take().is_none() {
                    return Err("SendKeys: ')' without '('".into());
                }
                held = Held::default();
                continue;
            }
            '~' => out.push(stroke_key(13, current)),
            '{' => {
                // ({{} and {}}: the braces themselves)
                let close = if chars.get(i) == Some(&'}') && chars.get(i + 1) == Some(&'}') {
                    i + 1
                } else {
                    chars[i..].iter().position(|&c| c == '}').map(|p| i + p).ok_or("SendKeys: '{' without '}'")?
                };
                let inner: String = chars[i..close].iter().collect();
                i = close + 1;
                let (name, count) = match inner.rsplit_once(' ') {
                    Some((n, k)) if !n.is_empty() && k.parse::<usize>().is_ok() => (n.to_string(), k.parse::<usize>().unwrap_or(1)),
                    _ => (inner.clone(), 1),
                };
                let mut name_chars = name.chars();
                let stroke = match (name_chars.next(), name_chars.next()) {
                    (Some(one), None) => stroke_char(one, current),
                    _ => stroke_key(named(&name).ok_or_else(|| format!("SendKeys: no key called {{{name}}}"))?, current),
                };
                out.extend(std::iter::repeat_n(stroke, count.min(10_000)));
            }
            c => out.push(stroke_char(c, current)),
        }
        // (a modifier holds for one key; a group's for all of it)
        held = Held::default();
    }
    if group.is_some() {
        return Err("SendKeys: '(' without ')'".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(vk: i64, text: &str, shift: bool, ctrl: bool, alt: bool) -> Stroke {
        Stroke { vk, text: text.into(), shift, ctrl, alt }
    }

    #[test]
    fn visual_basic_notation() {
        assert_eq!(parse("aB.").unwrap(), vec![s(65, "a", false, false, false), s(66, "B", true, false, false), s(190, ".", false, false, false)]);
        assert_eq!(parse("^ ").unwrap(), vec![s(32, "", false, true, false)]);
        assert_eq!(parse("+{TAB}").unwrap(), vec![s(9, "", true, false, false)]);
        assert_eq!(parse("{F12}~").unwrap(), vec![s(123, "", false, false, false), s(13, "", false, false, false)]);
        assert_eq!(parse("{LEFT 3}").unwrap().len(), 3);
        assert_eq!(parse("+(ab)c").unwrap(), vec![s(65, "A", true, false, false), s(66, "B", true, false, false), s(67, "c", false, false, false)]);
        assert_eq!(parse("{+}{(}{{}{}}").unwrap().iter().map(|k| k.text.clone()).collect::<String>(), "+({}");
        assert!(parse("{NOPE}").is_err() && parse("(a").is_err() && parse("a)").is_err() && parse("{TAB").is_err());
    }

    #[test]
    fn queued_until_delivered() {
        clear();
        assert_eq!(send("ab{ENTER}"), Ok(3));
        assert_eq!(pending(), 3);
        assert_eq!(next().map(|k| k.vk), Some(65));
        clear();
        assert_eq!(pending(), 0);
    }
}
