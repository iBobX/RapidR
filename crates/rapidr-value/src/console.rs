//! RapidQ's console statements (manual appendix C: CLS, COLOR, LOCATE,
//! CSRLIN, POS) as ANSI/VT escape sequences, the portable way to drive a
//! terminal (macOS, Linux, Windows 10+). The web IDE's output panel
//! understands the same sequences. Also tracks the output cursor for
//! CSRLIN/POS and PRINT zones; escape sequences don't move it.
//!
//! The console's *pages* (manual chapter 6.3, checked with RC.EXE on screen:
//! docs/windows-dll-calls.md §2): page 0 is the screen, 80 × 25 cells of a
//! character and an attribute byte (4000 bytes: even addresses the
//! character, odd the attribute `background SHL 4 OR foreground`); pages 1
//! to 7 are off-screen buffers. `PEEK([#page,] address)` reads a byte of
//! one, `POKE [#page,] address, byte` writes one (on page 0 it shows at
//! once), `PCOPY from, to` copies a page. Page 0 is a model of what the
//! program printed: every PRINT, LOCATE, COLOR and CLS keeps it current.

use std::cell::{Cell, RefCell};

use crate::objects::codec::{bytes_to_string, string_to_bytes};
use crate::Value;

thread_local! {
    /// Output cursor, 1-based (row, column).
    static CURSOR: Cell<(usize, usize)> = const { Cell::new((1, 1)) };
    /// The cursor `ESC[s` saved for `ESC[u`.
    static SAVED: Cell<(usize, usize)> = const { Cell::new((1, 1)) };
    /// The current colour as a page attribute (`bg SHL 4 OR fg`): light
    /// grey on black until COLOR says otherwise.
    static ATTR: Cell<u8> = const { Cell::new(7) };
    /// The eight pages' bytes.
    static PAGES: RefCell<Vec<u8>> = RefCell::new(new_pages());
}

/// The screen's size (RapidQ's console is 80 × 25).
pub const COLUMNS: usize = 80;
pub const ROWS: usize = 25;
/// One page's bytes.
pub const PAGE_SIZE: usize = COLUMNS * ROWS * 2;
/// Pages 0 (the screen) to 7.
pub const PAGE_COUNT: usize = 8;

fn new_pages() -> Vec<u8> {
    let mut v = vec![0u8; PAGE_SIZE * PAGE_COUNT];
    for cell in v[..PAGE_SIZE].chunks_mut(2) {
        cell[0] = b' ';
        cell[1] = 7;
    }
    v
}

/// Where the cursor ends up after printing `text` from `(row, col)`:
/// text advances it, `\n` starts a new row, `ESC[r;cH` moves it, and other
/// escape sequences (colors, clearing) leave it alone.
pub fn advance(row: usize, col: usize, text: &str) -> (usize, usize) {
    walk(row, col, text, &mut |_, _, _| {})
}

/// What an SGR sequence's parameters make of attribute `attr`.
fn sgr(params: &str, mut attr: u8) -> u8 {
    const ANSI_TO_QB: [u8; 8] = [0, 4, 2, 6, 1, 5, 3, 7];
    for p in params.split(';') {
        let n: u32 = p.parse().unwrap_or(0);
        match n {
            0 => attr = 7,
            30..=37 => attr = (attr & 0xF0) | ANSI_TO_QB[(n - 30) as usize],
            90..=97 => attr = (attr & 0xF0) | (8 + ANSI_TO_QB[(n - 90) as usize]),
            40..=47 => attr = (attr & 0x0F) | (ANSI_TO_QB[(n - 40) as usize] << 4),
            100..=107 => attr = (attr & 0x0F) | ((8 + ANSI_TO_QB[(n - 100) as usize]) << 4),
            _ => {}
        }
    }
    attr
}

/// Walks `text` from `(row, col)` and returns where the cursor ends:
/// `f(row, col, what)` at each step, `what` the character printed there
/// (None for a move, a clear, a new line). Understands `ESC[r;cH` / `f`
/// (move), `ESC[s` / `ESC[u` (save / restore), `ESC[2J` (clear), `ESC[…m`
/// (colours, kept in `ATTR`).
fn walk(mut row: usize, mut col: usize, text: &str, f: &mut dyn FnMut(usize, usize, Option<char>)) -> (usize, usize) {
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\n' => {
                row += 1;
                col = 1;
                f(row, col, None);
            }
            '\r' => {
                col = 1;
                f(row, col, None);
            }
            '\x1b' if chars.peek() == Some(&'[') => {
                chars.next();
                let mut params = String::new();
                for p in chars.by_ref() {
                    if ('\x40'..='\x7e').contains(&p) {
                        match p {
                            'H' | 'f' => {
                                let mut it = params.split(';').map(|n| n.parse::<usize>().unwrap_or(1).max(1));
                                row = it.next().unwrap_or(1);
                                col = it.next().unwrap_or(1);
                            }
                            's' => SAVED.with(|s| s.set((row, col))),
                            'u' => (row, col) = SAVED.with(Cell::get),
                            'J' if params == "2" => clear_page0(),
                            'm' => ATTR.with(|a| a.set(sgr(&params, a.get()))),
                            _ => {}
                        }
                        break;
                    }
                    params.push(p);
                }
                f(row, col, None);
            }
            _ => {
                f(row, col, Some(c));
                col += 1;
            }
        }
    }
    (row, col)
}

fn clear_page0() {
    let attr = ATTR.with(Cell::get);
    PAGES.with(|p| {
        for cell in p.borrow_mut()[..PAGE_SIZE].chunks_mut(2) {
            cell[0] = b' ';
            cell[1] = attr;
        }
    });
}

/// The page-0 address of a cell, when it is on the screen.
fn cell_address(row: usize, col: usize) -> Option<usize> {
    ((1..=ROWS).contains(&row) && (1..=COLUMNS).contains(&col)).then(|| (row - 1) * COLUMNS * 2 + (col - 1) * 2)
}

/// Records printed text (called by every runtime's PRINT): the cursor, and
/// the screen page's cells.
pub fn track(text: &str) {
    let (row, col) = CURSOR.with(Cell::get);
    let end = walk(row, col, text, &mut |r, c, what| {
        if let Some(ch) = what {
            if let Some(a) = cell_address(r, c) {
                let byte = string_to_bytes(&ch.to_string()).first().copied().unwrap_or(b'?');
                let attr = ATTR.with(Cell::get);
                PAGES.with(|p| {
                    let mut p = p.borrow_mut();
                    p[a] = byte;
                    p[a + 1] = attr;
                });
            }
        }
    });
    CURSOR.with(|c| c.set(end));
}

/// A program starting: the cursor home, the colour the default, the pages
/// blank (the web host runs programs one after another in one page).
pub fn reset() {
    CURSOR.with(|c| c.set((1, 1)));
    SAVED.with(|c| c.set((1, 1)));
    ATTR.with(|a| a.set(7));
    PAGES.with(|p| *p.borrow_mut() = new_pages());
}

/// Current output column, 0-based (for PRINT zones).
pub fn column() -> usize {
    CURSOR.with(|c| c.get().1 - 1)
}

/// `CLS`: clear the screen and home the cursor.
pub fn cls() -> String {
    "\x1b[2J\x1b[H".to_string()
}

/// QBasic/RapidQ color number → ANSI color number (0-7) and brightness.
fn ansi(qb: i64) -> (i64, bool) {
    const MAP: [i64; 8] = [0, 4, 2, 6, 1, 5, 3, 7]; // black blue green cyan red magenta brown white
    let n = qb.rem_euclid(16);
    (MAP[(n % 8) as usize], n >= 8)
}

/// `COLOR [fg][, bg]`: foreground 0-15 (16-31 blink, shown without
/// blinking), background 0-7. No arguments resets the colors.
pub fn color(fg: &Value, bg: &Value) -> String {
    let mut codes = Vec::new();
    if matches!(fg, Value::Null) && matches!(bg, Value::Null) {
        codes.push("0".to_string());
    }
    if !matches!(fg, Value::Null) {
        let (c, bright) = ansi(fg.to_i64());
        codes.push(format!("{}", if bright { 90 + c } else { 30 + c }));
    }
    if !matches!(bg, Value::Null) {
        let (c, bright) = ansi(bg.to_i64());
        codes.push(format!("{}", if bright { 100 + c } else { 40 + c }));
    }
    format!("\x1b[{}m", codes.join(";"))
}

/// The escape sequence for a page attribute (foreground 0-15, background
/// 0-15).
fn attr_sequence(attr: u8) -> String {
    color(&Value::Integer((attr & 15) as i64), &Value::Integer((attr >> 4) as i64))
}

/// `LOCATE [row][, col]`: move the cursor (1-based); an omitted value keeps
/// the current one.
pub fn locate(row: &Value, col: &Value) -> String {
    let (cur_row, cur_col) = CURSOR.with(|c| c.get());
    let pick = |v: &Value, current: usize| if matches!(v, Value::Null) { current } else { v.to_i64().max(1) as usize };
    format!("\x1b[{};{}H", pick(row, cur_row), pick(col, cur_col))
}

// ---------------------------------------------------------------------------
// The pages: PEEK, POKE, PCOPY
// ---------------------------------------------------------------------------

/// Whether `addr` is a page address (0 to 3999).
pub fn is_page_address(addr: i64) -> bool {
    (0..PAGE_SIZE as i64).contains(&addr)
}

fn page_index(page: &Value) -> Result<usize, String> {
    let n = page.to_i64();
    if !(0..PAGE_COUNT as i64).contains(&n) {
        return Err(format!("console page {n}: the pages are 0 (the screen) to 7"));
    }
    Ok(n as usize)
}

fn page_offset(page: usize, addr: &Value) -> Result<usize, String> {
    let a = addr.to_i64();
    if !is_page_address(a) {
        return Err(format!("address {a} isn't on a console page (0 to 3999: 80 columns × 25 rows × a character and its attribute); a variable's address comes from VARPTR"));
    }
    Ok(page * PAGE_SIZE + a as usize)
}

/// `PEEK(page, address)`: the byte there.
pub fn peek(page: &Value, addr: &Value) -> Result<Value, String> {
    let off = page_offset(page_index(page)?, addr)?;
    Ok(Value::Integer(PAGES.with(|p| p.borrow()[off]) as i64))
}

/// `POKE page, address, byte`: writes the byte; returns what to print so
/// a change to the screen page shows (nothing for the other pages).
pub fn poke(page: &Value, addr: &Value, byte: &Value) -> Result<String, String> {
    let page = page_index(page)?;
    let off = page_offset(page, addr)?;
    let b = byte.to_i64() as u8;
    PAGES.with(|p| p.borrow_mut()[off] = b);
    if page != 0 {
        return Ok(String::new());
    }
    // The cell, redrawn where it is: the cursor saved and restored, the
    // colour put back afterwards.
    let cell = off & !1;
    let (ch, attr) = PAGES.with(|p| {
        let p = p.borrow();
        (p[cell], p[cell + 1])
    });
    let (row, col) = (cell / (COLUMNS * 2) + 1, (cell % (COLUMNS * 2)) / 2 + 1);
    let text = bytes_to_string(&[ch]);
    let current = ATTR.with(Cell::get);
    Ok(format!("\x1b[s\x1b[{row};{col}H{}{text}{}\x1b[u", attr_sequence(attr), attr_sequence(current)))
}

/// `PCOPY from, to`: copies a page onto another; returns what to print
/// when the screen page changed.
pub fn pcopy(from: &Value, to: &Value) -> Result<String, String> {
    let (from, to) = (page_index(from)?, page_index(to)?);
    if from == to {
        return Ok(String::new());
    }
    PAGES.with(|p| {
        let mut p = p.borrow_mut();
        let src: Vec<u8> = p[from * PAGE_SIZE..(from + 1) * PAGE_SIZE].to_vec();
        p[to * PAGE_SIZE..(to + 1) * PAGE_SIZE].copy_from_slice(&src);
    });
    if to != 0 {
        return Ok(String::new());
    }
    // The screen page redrawn whole.
    let mut out = String::from("\x1b[s");
    let mut last_attr = None;
    PAGES.with(|p| {
        let p = p.borrow();
        for row in 0..ROWS {
            out.push_str(&format!("\x1b[{};1H", row + 1));
            for col in 0..COLUMNS {
                let a = row * COLUMNS * 2 + col * 2;
                if last_attr != Some(p[a + 1]) {
                    out.push_str(&attr_sequence(p[a + 1]));
                    last_attr = Some(p[a + 1]);
                }
                out.push_str(&bytes_to_string(&[p[a]]));
            }
        }
    });
    out.push_str(&attr_sequence(ATTR.with(Cell::get)));
    out.push_str("\x1b[u");
    Ok(out)
}

/// `CSRLIN`: the cursor's row (1-based).
pub fn csrlin() -> Value {
    Value::Integer(CURSOR.with(|c| c.get().0) as i64)
}

/// `POS(0)`: the cursor's column (1-based).
pub fn pos() -> Value {
    Value::Integer(CURSOR.with(|c| c.get().1) as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_move_the_cursor_but_colors_do_not() {
        assert_eq!(advance(1, 1, "ab\ncd"), (2, 3));
        assert_eq!(advance(4, 7, &cls()), (1, 1));
        assert_eq!(advance(1, 1, &format!("{}xy", color(&Value::Integer(14), &Value::Integer(1)))), (1, 3));
        assert_eq!(advance(9, 9, "\x1b[5;10H!"), (5, 11));
    }

    #[test]
    fn qbasic_colors_map_to_ansi() {
        assert_eq!(color(&Value::Integer(4), &Value::Null), "\x1b[31m"); // red
        assert_eq!(color(&Value::Integer(14), &Value::Integer(1)), "\x1b[93;44m"); // yellow on blue
        assert_eq!(color(&Value::Null, &Value::Null), "\x1b[0m");
        track("\x1b[3;4H");
        assert_eq!(locate(&Value::Null, &Value::Integer(9)), "\x1b[3;9H");
        track(&cls());
        assert_eq!((csrlin(), pos()), (Value::Integer(1), Value::Integer(1)));
    }

    #[test]
    fn pages_follow_what_is_printed() {
        let i = |n: i64| Value::Integer(n);
        track(&color(&Value::Null, &Value::Null));
        track(&cls());
        track("Hello");
        // (RC.EXE on screen: 72,7,101 … a blank cell 32,7)
        assert_eq!(peek(&i(0), &i(0)).unwrap(), i(72));
        assert_eq!(peek(&i(0), &i(1)).unwrap(), i(7));
        assert_eq!(peek(&i(0), &i(2)).unwrap(), i(101));
        assert_eq!(peek(&i(0), &i(1132)).unwrap(), i(32));
        assert_eq!(peek(&i(0), &i(3999)).unwrap(), i(7));
        // COLOR 14, 1: LOCATE 4, 1: PRINT "Y"; → 89, 30
        track(&color(&i(14), &i(1)));
        track(&locate(&i(4), &i(1)));
        track("Y");
        assert_eq!((peek(&i(0), &i(480)).unwrap(), peek(&i(0), &i(481)).unwrap()), (i(89), i(30)));
        // A POKE shows: the cell redrawn, the cursor kept.
        let shown = poke(&i(0), &i(0), &i(65)).unwrap();
        assert!(shown.starts_with("\x1b[s\x1b[1;1H") && shown.ends_with("\x1b[u"), "{shown}");
        let before = CURSOR.with(Cell::get);
        track(&shown);
        assert_eq!(CURSOR.with(Cell::get), before);
        assert_eq!(peek(&i(0), &i(0)).unwrap(), i(65));
        // Off-screen pages start as zeros and keep what is poked.
        assert_eq!(peek(&i(1), &i(0)).unwrap(), i(0));
        assert_eq!(poke(&i(1), &i(0), &i(66)).unwrap(), "");
        assert_eq!(peek(&i(1), &i(0)).unwrap(), i(66));
        assert!(pcopy(&i(0), &i(2)).unwrap().is_empty());
        assert_eq!(peek(&i(2), &i(0)).unwrap(), i(65));
        // CLS fills the screen with the current attribute (RC.EXE: 32, 30).
        track(&cls());
        assert_eq!((peek(&i(0), &i(0)).unwrap(), peek(&i(0), &i(1)).unwrap()), (i(32), i(30)));
        track(&color(&Value::Null, &Value::Null));
        assert!(peek(&i(0), &i(4000)).is_err());
        assert!(peek(&i(8), &i(0)).is_err());
        assert!(peek(&i(0), &i(-1)).is_err());
    }
}

// ---------------------------------------------------------------------------
// INKEY$ (manual chapter 6: a non-blocking keyboard check, as QBasic's)
// ---------------------------------------------------------------------------

thread_local! {
    /// Keys pressed and not read yet, as INKEY$ returns them.
    static KEYS: std::cell::RefCell<std::collections::VecDeque<String>> = const { std::cell::RefCell::new(std::collections::VecDeque::new()) };
}

/// Most keys kept unread (a program that never reads them can't grow it).
const MAX_KEYS: usize = 256;

/// A key pressed (as [`inkey_of`] names it) for INKEY$ to return.
pub fn push_key(key: String) {
    KEYS.with(|k| {
        let mut k = k.borrow_mut();
        if k.len() < MAX_KEYS {
            k.push_back(key);
        }
    });
}

thread_local! {
    static TRAP_ALL: Cell<bool> = const { Cell::new(false) };
}

/// `$OPTION INKEY$ TRAPALL` (true) / `$OPTION INKEY$ DEFAULT` (false): whether
/// INKEY$ also returns Shift, Ctrl, Alt, the lock keys and the menu key.
pub fn set_inkey_trap_all(on: bool) {
    TRAP_ALL.with(|t| t.set(on));
}

/// Whether INKEY$ has a key to return.
pub fn key_waiting() -> bool {
    KEYS.with(|k| !k.borrow().is_empty())
}

/// `INKEY$`: the next key pressed, or "" (it doesn't wait).
pub fn inkey() -> Value {
    Value::String(KEYS.with(|k| k.borrow_mut().pop_front()).unwrap_or_default())
}

/// What INKEY$ returns for a key (`vk` its virtual key code, `text` what it
/// types): the character; Enter CHR$(13), Escape CHR$(27), Backspace
/// CHR$(8), Tab CHR$(9); the arrows, Home / End / Page Up / Page Down,
/// Insert / Delete and F1–F12 as RapidQ's extended keys: CHR$(27) + their
/// QBasic scan code (Up is CHR$(27) + "H"; RapidQ manual, chapter 6.5).
/// Shift, Ctrl, Alt, Caps / Num / Scroll Lock and the menu key count only
/// under `$OPTION INKEY$ TRAPALL` ([`set_inkey_trap_all`]).
pub fn inkey_of(vk: i64, text: &str) -> Option<String> {
    let scan = match vk {
        38 => 72,
        40 => 80,
        37 => 75,
        39 => 77,
        36 => 71,
        35 => 79,
        33 => 73,
        34 => 81,
        45 => 82,
        46 => 83,
        112..=121 => 59 + (vk - 112),
        122 => 133,
        123 => 134,
        16 | 17 | 18 | 20 | 93 | 144 | 145 if TRAP_ALL.with(Cell::get) => match vk {
            16 => 42,
            17 => 29,
            18 => 56,
            20 => 58,
            144 => 69,
            145 => 70,
            _ => 93,
        },
        _ => 0,
    };
    if scan != 0 {
        return Some(format!("\x1b{}", char::from(scan as u8)));
    }
    match vk {
        13 => Some("\r".into()),
        27 => Some("\x1b".into()),
        8 => Some("\x08".into()),
        9 => Some("\t".into()),
        _ => {
            let mut chars = text.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) if !c.is_control() || c == ' ' => Some(c.to_string()),
                _ => None,
            }
        }
    }
}

/// Keys read from a terminal (its bytes, escape sequences for the arrows
/// and the like) as INKEY$ returns them.
pub fn terminal_keys(bytes: &[u8]) -> Vec<String> {
    let mut keys = Vec::new();
    let text = String::from_utf8_lossy(bytes);
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' && matches!(chars.peek(), Some('[') | Some('O')) {
            chars.next();
            let mut seq = String::new();
            for p in chars.by_ref() {
                seq.push(p);
                if p.is_ascii_alphabetic() || p == '~' {
                    break;
                }
            }
            let vk = match seq.as_str() {
                "A" => 38,
                "B" => 40,
                "D" => 37,
                "C" => 39,
                "H" | "1~" | "7~" => 36,
                "F" | "4~" | "8~" => 35,
                "5~" => 33,
                "6~" => 34,
                "2~" => 45,
                "3~" => 46,
                "P" | "11~" => 112,
                "Q" | "12~" => 113,
                "R" | "13~" => 114,
                "S" | "14~" => 115,
                "15~" => 116,
                "17~" => 117,
                "18~" => 118,
                "19~" => 119,
                "20~" => 120,
                "21~" => 121,
                "23~" => 122,
                "24~" => 123,
                _ => 0,
            };
            if let Some(k) = inkey_of(vk, "") {
                keys.push(k);
            }
            continue;
        }
        let key = match c {
            '\n' => "\r".to_string(),
            '\x7f' => "\x08".to_string(),
            c => c.to_string(),
        };
        keys.push(key);
    }
    keys
}

#[cfg(test)]
mod inkey_tests {
    use super::*;

    #[test]
    fn keys_as_rapidq_names_them() {
        assert_eq!(inkey_of(65, "a").as_deref(), Some("a"));
        assert_eq!(inkey_of(38, "").as_deref(), Some("\x1bH"));
        assert_eq!(inkey_of(112, "").as_deref(), Some("\x1b;"));
        assert_eq!(inkey_of(13, "\r").as_deref(), Some("\r"));
        assert_eq!(inkey_of(16, ""), None);
        set_inkey_trap_all(true);
        assert_eq!(inkey_of(16, "").as_deref(), Some("\x1b*"));
        set_inkey_trap_all(false);
        assert_eq!(terminal_keys(b"x\x1b[A\x1b[D\n\x7f"), vec!["x", "\x1bH", "\x1bK", "\r", "\x08"]);
        push_key("q".into());
        assert_eq!(inkey().to_string_val(), "q");
        assert_eq!(inkey().to_string_val(), "");
    }
}
