//! RapidQ's console statements (manual appendix C: CLS, COLOR, LOCATE,
//! CSRLIN, POS, PEEK, POKE, PCOPY) as ANSI/VT escape sequences, the
//! portable way to drive a terminal (macOS, Linux, Windows 10+). The web
//! IDE's output panel understands the same sequences. Also tracks the
//! output cursor for CSRLIN/POS and PRINT zones; escape sequences don't
//! move it — and RapidQ's 80 × 25 console pages, which PEEK reads and
//! POKE / PCOPY write ([`peek`], [`poke`], [`pcopy`]).

use std::cell::Cell;

use crate::Value;

thread_local! {
    /// Output cursor, 1-based (row, column).
    static CURSOR: Cell<(usize, usize)> = const { Cell::new((1, 1)) };
}

/// Where the cursor ends up after printing `text` from `(row, col)`:
/// text advances it, `\n` starts a new row, `ESC[r;cH` moves it, and other
/// escape sequences (colors, clearing) leave it alone.
pub fn advance(mut row: usize, mut col: usize, text: &str) -> (usize, usize) {
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\n' => {
                row += 1;
                col = 1;
            }
            '\r' => col = 1,
            '\x1b' if chars.peek() == Some(&'[') => {
                chars.next();
                let mut params = String::new();
                for p in chars.by_ref() {
                    if ('\x40'..='\x7e').contains(&p) {
                        if p == 'H' || p == 'f' {
                            let mut it = params.split(';').map(|n| n.parse::<usize>().unwrap_or(1).max(1));
                            row = it.next().unwrap_or(1);
                            col = it.next().unwrap_or(1);
                        }
                        break;
                    }
                    params.push(p);
                }
            }
            _ => col += 1,
        }
    }
    (row, col)
}

/// Records printed text (called by every runtime's PRINT).
pub fn track(text: &str) {
    CURSOR.with(|c| {
        let (row, col) = c.get();
        c.set(advance(row, col, text));
    });
    PAGES.with(|p| p.borrow_mut().feed(text));
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

/// `LOCATE [row][, col]`: move the cursor (1-based); an omitted value keeps
/// the current one.
pub fn locate(row: &Value, col: &Value) -> String {
    let (cur_row, cur_col) = CURSOR.with(|c| c.get());
    let pick = |v: &Value, current: usize| if matches!(v, Value::Null) { current } else { v.to_i64().max(1) as usize };
    format!("\x1b[{};{}H", pick(row, cur_row), pick(col, cur_col))
}

/// `CSRLIN`: the cursor's row (1-based).
pub fn csrlin() -> Value {
    Value::Integer(CURSOR.with(|c| c.get().0) as i64)
}

/// `POS(0)`: the cursor's column (1-based).
pub fn pos() -> Value {
    Value::Integer(CURSOR.with(|c| c.get().1) as i64)
}

// ---------------------------------------------------------------------------
// The console's pages (RapidQ manual, chapter 6.3 and appendix C: PEEK, POKE,
// PCOPY — QBasic's screen memory without DEF SEG)
// ---------------------------------------------------------------------------

/// RapidQ's console: 80 columns, 25 rows ("the screen size is restricted
/// to 80x25 in Rapid-Q"), page 0 the one shown and 7 off screen.
const COLS: usize = 80;
const ROWS: usize = 25;
const PAGE_COUNT: usize = 8;
/// A page's addresses: a character (even) and its attribute (odd) per cell.
const PAGE_BYTES: i64 = (COLS * ROWS * 2) as i64;
/// The attribute a console starts with and `COLOR` with no arguments sets:
/// light grey on black (background × 16 + foreground).
const DEFAULT_ATTR: u8 = 7;
/// QBasic colour number ↔ ANSI colour number (0-7): the same swap both
/// ways (blue ↔ red, cyan ↔ brown).
const QB_ANSI: [u8; 8] = [0, 4, 2, 6, 1, 5, 3, 7];

thread_local! {
    static PAGES: std::cell::RefCell<Pages> = std::cell::RefCell::new(Pages::new());
}

/// The pages' cells (character byte, attribute) and, for page 0, the
/// cursor and colours what's printed is written with — kept by reading the
/// program's output as a terminal would ([`Pages::feed`]), so PEEK reads
/// what the screen shows.
struct Pages {
    cells: Vec<Vec<(u8, u8)>>,
    row: usize,
    col: usize,
    /// A character went into the last column: the next one starts a new
    /// line first (a terminal's deferred wrap).
    wrap: bool,
    attr: u8,
}

impl Pages {
    fn new() -> Pages {
        Pages { cells: vec![vec![(b' ', DEFAULT_ATTR); COLS * ROWS]; PAGE_COUNT], row: 0, col: 0, wrap: false, attr: DEFAULT_ATTR }
    }

    fn new_line(&mut self) {
        self.wrap = false;
        self.col = 0;
        if self.row + 1 < ROWS {
            self.row += 1;
        } else {
            // (the screen scrolls a line)
            let attr = self.attr;
            let page = &mut self.cells[0];
            page.drain(..COLS);
            page.extend(std::iter::repeat_n((b' ', attr), COLS));
        }
    }

    fn put(&mut self, c: char) {
        if self.wrap {
            self.new_line();
        }
        let byte = u8::try_from(u32::from(c)).unwrap_or(b'?');
        self.cells[0][self.row * COLS + self.col] = (byte, self.attr);
        if self.col + 1 < COLS {
            self.col += 1;
        } else {
            self.wrap = true;
        }
    }

    /// The colours an `ESC[…m` sets (COLOR's codes, 0 the console's own).
    fn sgr(&mut self, params: &str) {
        let (mut fg, mut bg) = (self.attr & 15, self.attr >> 4);
        for code in params.split(';').map(|n| n.parse::<u8>().unwrap_or(0)) {
            match code {
                0 => (fg, bg) = (DEFAULT_ATTR & 15, DEFAULT_ATTR >> 4),
                30..=37 => fg = QB_ANSI[usize::from(code - 30)],
                90..=97 => fg = QB_ANSI[usize::from(code - 90)] + 8,
                39 => fg = DEFAULT_ATTR & 15,
                40..=47 => bg = QB_ANSI[usize::from(code - 40)],
                100..=107 => bg = QB_ANSI[usize::from(code - 100)] + 8,
                49 => bg = DEFAULT_ATTR >> 4,
                _ => {}
            }
        }
        self.attr = bg << 4 | fg;
    }

    /// Page 0 after `text` is printed: characters written at the cursor in
    /// the colours set, `\n` / `\r` / tab / backspace, and the escapes the
    /// console statements print (move, clear, clear to the line's end,
    /// colours).
    fn feed(&mut self, text: &str) {
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '\n' => self.new_line(),
                '\r' => {
                    self.col = 0;
                    self.wrap = false;
                }
                '\t' => {
                    self.col = ((self.col / 8 + 1) * 8).min(COLS - 1);
                    self.wrap = false;
                }
                '\x08' => {
                    self.col = self.col.saturating_sub(1);
                    self.wrap = false;
                }
                '\x07' => {}
                '\x1b' if chars.peek() == Some(&'[') => {
                    chars.next();
                    let mut params = String::new();
                    let mut end = None;
                    for p in chars.by_ref() {
                        if ('\x40'..='\x7e').contains(&p) {
                            end = Some(p);
                            break;
                        }
                        params.push(p);
                    }
                    match end {
                        Some('H' | 'f') => {
                            let mut it = params.split(';').map(|n| n.parse::<usize>().unwrap_or(1).max(1));
                            self.row = it.next().unwrap_or(1).min(ROWS) - 1;
                            self.col = it.next().unwrap_or(1).min(COLS) - 1;
                            self.wrap = false;
                        }
                        Some('J') if matches!(params.as_str(), "2" | "3") => {
                            let attr = self.attr;
                            self.cells[0].fill((b' ', attr));
                        }
                        Some('K') => {
                            let (at, attr) = (self.row * COLS, self.attr);
                            self.cells[0][at + self.col..at + COLS].fill((b' ', attr));
                        }
                        Some('m') => self.sgr(&params),
                        _ => {}
                    }
                }
                c if c.is_control() => {}
                c => self.put(c),
            }
        }
    }
}

/// An attribute as the escape that shows it (foreground, background).
fn attr_escape(attr: u8) -> String {
    let (fg, bg) = (attr & 15, attr >> 4);
    let ansi = |n: u8| QB_ANSI[usize::from(n % 8)];
    format!("\x1b[{};{}m", if fg >= 8 { 90 } else { 30 } + ansi(fg), if bg >= 8 { 100 } else { 40 } + ansi(bg))
}

/// A cell's character as it's printed.
fn cell_char(byte: u8) -> char {
    if byte < 32 || byte == 127 { ' ' } else { char::from(byte) }
}

/// `[page,] address` (and the rest) from PEEK's / POKE's arguments: the
/// page is the first of one more than `plain` arguments.
fn page_and_rest(args: &[Value], plain: usize) -> (i64, &[Value]) {
    if args.len() > plain { (args[0].to_i64(), &args[1..]) } else { (0, args) }
}

/// What redraws `cells` (row, column 0-based: (character, attribute)) on
/// the screen, the cursor and colours put back after.
fn redraw(cells: impl IntoIterator<Item = (usize, usize, u8, u8)>) -> String {
    let (row, col, attr) = PAGES.with(|p| {
        let p = p.borrow();
        (p.row, p.col, p.attr)
    });
    let mut out = String::new();
    let mut at = None;
    let mut shown = None;
    for (r, c, ch, a) in cells {
        if at != Some((r, c)) {
            out.push_str(&format!("\x1b[{};{}H", r + 1, c + 1));
        }
        if shown != Some(a) {
            out.push_str(&attr_escape(a));
            shown = Some(a);
        }
        out.push(cell_char(ch));
        at = Some((r, c + 1));
    }
    if out.is_empty() {
        return out;
    }
    // (the colours COLOR set, the cursor where it was)
    out.push_str(&attr_escape(attr));
    out.push_str(&format!("\x1b[{};{}H", row + 1, col + 1));
    out
}

/// `POKE [page,] address, byte`: the character (an even address) or the
/// attribute (odd: background × 16 + foreground) of a console cell —
/// `(row - 1) * 160 + (column - 1) * 2`, 0 to 3999 — on page 0 (shown at
/// once) or an off-screen page 1 to 7. What to print: page 0's change.
pub fn poke(args: &[Value]) -> String {
    let (page, rest) = page_and_rest(args, 2);
    let (address, byte) = (rest.first().map_or(0, Value::to_i64), rest.get(1).map_or(0, Value::to_i64) as u8);
    if !(0..PAGE_COUNT as i64).contains(&page) || !(0..PAGE_BYTES).contains(&address) {
        return String::new();
    }
    let cell = (address / 2) as usize;
    let (ch, attr) = PAGES.with(|p| {
        let mut p = p.borrow_mut();
        let c = &mut p.cells[page as usize][cell];
        if address % 2 == 0 {
            c.0 = byte;
        } else {
            c.1 = byte;
        }
        *c
    });
    if page != 0 {
        return String::new();
    }
    redraw([(cell / COLS, cell % COLS, ch, attr)])
}

/// `PEEK([page,] address)`: a console cell's character (even address) or
/// attribute (odd) on a page (0, the screen, unless given); 0 outside the
/// page.
pub fn peek(args: &[Value]) -> Value {
    let (page, rest) = page_and_rest(args, 1);
    let address = rest.first().map_or(0, Value::to_i64);
    if !(0..PAGE_COUNT as i64).contains(&page) || !(0..PAGE_BYTES).contains(&address) {
        return Value::Integer(0);
    }
    let (ch, attr) = PAGES.with(|p| p.borrow().cells[page as usize][(address / 2) as usize]);
    Value::Integer(i64::from(if address % 2 == 0 { ch } else { attr }))
}

/// `PCOPY source, dest`: a page copied onto another (0 the screen, 1 to 7
/// off screen). What to print: the screen redrawn when it's the
/// destination.
pub fn pcopy(source: &Value, dest: &Value) -> String {
    let (from, to) = (source.to_i64(), dest.to_i64());
    let pages = 0..PAGE_COUNT as i64;
    if !pages.contains(&from) || !pages.contains(&to) || from == to {
        return String::new();
    }
    let cells = PAGES.with(|p| {
        let mut p = p.borrow_mut();
        let copy = p.cells[from as usize].clone();
        p.cells[to as usize] = copy.clone();
        copy
    });
    if to != 0 {
        return String::new();
    }
    redraw(cells.into_iter().enumerate().map(|(i, (ch, a))| (i / COLS, i % COLS, ch, a)))
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
    fn pages_peek_poke_pcopy() {
        let int = Value::Integer;
        track(&color(&Value::Null, &Value::Null));
        track(&cls());
        // (what's printed is on page 0, in the colours COLOR set)
        track(&color(&int(14), &int(1)));
        track("Hi\n");
        assert_eq!((peek(&[int(0)]), peek(&[int(1)]), peek(&[int(2)])), (int(i64::from(b'H')), int(0x1E), int(i64::from(b'i'))));
        assert_eq!((peek(&[int(160)]), peek(&[int(161)])), (int(32), int(7)));
        // (POKE: a character, then its attribute, drawn where they go and
        // the cursor and colours put back)
        let drawn = poke(&[int(2000), int(i64::from(b'A'))]);
        assert_eq!(drawn, "\x1b[13;41H\x1b[37;40mA\x1b[93;44m\x1b[2;1H");
        track(&drawn);
        assert_eq!((csrlin(), pos()), (int(2), int(1)));
        track(&poke(&[int(2001), int(95)]));
        assert_eq!((peek(&[int(2000)]), peek(&[int(2001)])), (int(65), int(95)));
        // (3DBOX's: the last cell's attribute — no scrolling)
        track(&poke(&[int(3999), int(48)]));
        assert_eq!((peek(&[int(3999)]), peek(&[int(0)])), (int(48), int(i64::from(b'H'))));
        // (off-screen pages: nothing printed; PCOPY to page 0 redraws it)
        assert_eq!(poke(&[int(1), int(0), int(i64::from(b'Z'))]), "");
        assert_eq!((peek(&[int(1), int(0)]), peek(&[int(0)])), (int(i64::from(b'Z')), int(i64::from(b'H'))));
        assert_eq!(pcopy(&int(0), &int(2)), "");
        let screen = pcopy(&int(1), &int(0));
        assert!(screen.starts_with("\x1b[1;1H\x1b[37;40mZ"), "{screen:?}");
        track(&screen);
        assert_eq!((peek(&[int(0)]), peek(&[int(2), int(0)]), peek(&[int(2001)])), (int(i64::from(b'Z')), int(i64::from(b'H')), int(7)));
        // (out of range: nothing)
        assert_eq!((poke(&[int(4000), int(1)]), peek(&[int(-1)]), pcopy(&int(0), &int(8))), (String::new(), int(0), String::new()));
        // (a full screen scrolls; LOCATE past the screen stays on it)
        track(&cls());
        for i in 0..26 {
            track(&format!("{i}\n"));
        }
        assert_eq!((peek(&[int(0)]), peek(&[int(2)]), peek(&[int(23 * 160)]), peek(&[int(23 * 160 + 2)])), (int(i64::from(b'2')), int(i64::from(b' ')), int(i64::from(b'2')), int(i64::from(b'5'))));
        track("\x1b[40;90HX");
        assert_eq!(peek(&[int(3998)]), int(i64::from(b'X')));
        track(&color(&Value::Null, &Value::Null));
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
