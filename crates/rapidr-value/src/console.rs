//! RapidQ's console statements (manual appendix C: CLS, COLOR, LOCATE,
//! CSRLIN, POS) as ANSI/VT escape sequences, the portable way to drive a
//! terminal (macOS, Linux, Windows 10+). The web IDE's output panel
//! understands the same sequences. Also tracks the output cursor for
//! CSRLIN/POS and PRINT zones; escape sequences don't move it.

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
}
