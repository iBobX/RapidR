//! BASIC builtin functions — implementations that back the generated Rust code.

use crate::value::{v_bool, v_dbl, v_int, v_null, v_str, Value};
use std::io::{self, Write};

#[cfg(feature = "audio")]
use rodio::Source;

// ---------------------------------------------------------------------------
// PRINT
// ---------------------------------------------------------------------------

/// BASIC `PRINT` — items are space-separated; optional trailing newline.
pub fn rp_print(items: &[Value], newline: bool) {
    let mut first = true;
    for item in items {
        if !first {
            print!(" ");
        }
        print!("{}", item.to_string_val());
        first = false;
    }
    if newline {
        println!();
    }
    let _ = io::stdout().flush();
}

// ---------------------------------------------------------------------------
// INPUT
// ---------------------------------------------------------------------------

pub fn rp_input(prompt: &Value) -> Value {
    let p = prompt.to_string_val();
    if !p.is_empty() {
        print!("{}", p);
        let _ = io::stdout().flush();
    }
    let mut buf = String::new();
    match io::stdin().read_line(&mut buf) {
        Ok(_) => Value::String(buf.trim_end_matches('\n').trim_end_matches('\r').to_string()),
        Err(_) => Value::String(String::new()),
    }
}

// ---------------------------------------------------------------------------
// String functions
// ---------------------------------------------------------------------------

pub fn rp_len(val: &Value) -> Value {
    match val {
        Value::String(s) => v_int(s.len() as i64),
        _ => v_int(val.to_string_val().len() as i64),
    }
}

pub fn rp_mid(s: &Value, start: &Value, length: &Value) -> Value {
    let s = s.to_string_val();
    let start = (start.to_i64() - 1).max(0) as usize; // BASIC is 1-indexed
    let length = length.to_i64().max(0) as usize;
    if start >= s.len() {
        return v_str("");
    }
    let end = (start + length).min(s.len());
    Value::String(s[start..end].to_string())
}

pub fn rp_left(s: &Value, n: &Value) -> Value {
    rp_mid(s, &v_int(1), n)
}

pub fn rp_right(s: &Value, n: &Value) -> Value {
    let s = s.to_string_val();
    let n = n.to_i64().max(0) as usize;
    if n >= s.len() {
        Value::String(s)
    } else {
        Value::String(s[s.len() - n..].to_string())
    }
}

pub fn rp_ucase(s: &Value) -> Value {
    Value::String(s.to_string_val().to_uppercase())
}

pub fn rp_lcase(s: &Value) -> Value {
    Value::String(s.to_string_val().to_lowercase())
}

pub fn rp_ltrim(s: &Value) -> Value {
    Value::String(s.to_string_val().trim_start().to_string())
}

pub fn rp_rtrim(s: &Value) -> Value {
    Value::String(s.to_string_val().trim_end().to_string())
}

pub fn rp_trim(s: &Value) -> Value {
    Value::String(s.to_string_val().trim().to_string())
}

pub fn rp_instr(start: &Value, haystack: &Value, needle: &Value) -> Value {
    let h = haystack.to_string_val();
    let n = needle.to_string_val();
    let from = (start.to_i64() - 1).max(0) as usize;
    match h[from..].find(&n) {
        Some(pos) => v_int((pos + from + 1) as i64), // 1-indexed result
        None => v_int(0),
    }
}

pub fn rp_space(n: &Value) -> Value {
    Value::String(" ".repeat(n.to_i64().max(0) as usize))
}

pub fn rp_string_func(n: &Value, ch: &Value) -> Value {
    let c = ch.to_string_val();
    let ch = c.chars().next().unwrap_or(' ');
    Value::String(std::iter::repeat(ch).take(n.to_i64().max(0) as usize).collect())
}

pub fn rp_chr(n: &Value) -> Value {
    Value::String(String::from(char::from(n.to_i64() as u8)))
}

pub fn rp_asc(s: &Value) -> Value {
    let s = s.to_string_val();
    v_int(s.bytes().next().unwrap_or(0) as i64)
}

pub fn rp_replace(s: &Value, old: &Value, new: &Value) -> Value {
    Value::String(s.to_string_val().replace(&old.to_string_val(), &new.to_string_val()))
}

// ---------------------------------------------------------------------------
// Numeric / conversion functions
// ---------------------------------------------------------------------------

pub fn rp_str(val: &Value) -> Value {
    Value::String(val.to_string_val())
}

pub fn rp_val(s: &Value) -> Value {
    let s = s.to_string_val().trim().to_string();
    if let Ok(n) = s.parse::<i64>() {
        v_int(n)
    } else if let Ok(n) = s.parse::<f64>() {
        v_dbl(n)
    } else {
        v_int(0)
    }
}

pub fn rp_int(val: &Value) -> Value {
    v_int(val.to_f64().floor() as i64)
}

pub fn rp_abs(val: &Value) -> Value {
    match val {
        Value::Integer(n) => v_int(n.abs()),
        _ => v_dbl(val.to_f64().abs()),
    }
}

pub fn rp_sgn(val: &Value) -> Value {
    let n = val.to_f64();
    v_int(if n > 0.0 { 1 } else if n < 0.0 { -1 } else { 0 })
}

pub fn rp_sqr(val: &Value) -> Value {
    v_dbl(val.to_f64().sqrt())
}

pub fn rp_sin(val: &Value) -> Value {
    v_dbl(val.to_f64().sin())
}

pub fn rp_cos(val: &Value) -> Value {
    v_dbl(val.to_f64().cos())
}

pub fn rp_tan(val: &Value) -> Value {
    v_dbl(val.to_f64().tan())
}

pub fn rp_atn(val: &Value) -> Value {
    v_dbl(val.to_f64().atan())
}

pub fn rp_acos(val: &Value) -> Value {
    v_dbl(val.to_f64().acos())
}

pub fn rp_asin(val: &Value) -> Value {
    v_dbl(val.to_f64().asin())
}

pub fn rp_log(val: &Value) -> Value {
    v_dbl(val.to_f64().ln())
}

pub fn rp_exp(val: &Value) -> Value {
    v_dbl(val.to_f64().exp())
}

pub fn rp_ceil(val: &Value) -> Value {
    v_dbl(val.to_f64().ceil())
}

pub fn rp_floor(val: &Value) -> Value {
    v_dbl(val.to_f64().floor())
}

pub fn rp_round(val: &Value) -> Value {
    v_dbl(val.to_f64().round())
}

pub fn rp_hex(val: &Value) -> Value {
    Value::String(format!("{:X}", val.to_i64()))
}

pub fn rp_oct(val: &Value) -> Value {
    Value::String(format!("{:o}", val.to_i64()))
}

pub fn rp_bin(val: &Value) -> Value {
    Value::String(format!("{:b}", val.to_i64()))
}

pub fn rp_rnd(upper: &Value) -> Value {
    use rand::Rng;
    let mut rng = rand::rng();
    let u = upper.to_i64();
    if u > 0 {
        v_int(rng.random_range(0..u))
    } else {
        v_dbl(rng.random::<f64>())
    }
}

pub fn rp_randomize(seed: &Value) {
    // Seed is acknowledged but rand crate uses thread_rng automatically.
    // Full deterministic seeding would require a custom RNG wrapper.
    let _ = seed;
}

// ---------------------------------------------------------------------------
// Additional math / conversion (Phase 3d)
// ---------------------------------------------------------------------------

/// FIX — truncate toward zero (unlike INT which floors)
pub fn rp_fix(val: &Value) -> Value {
    let n = val.to_f64();
    v_int(n as i64)  // Rust truncates toward zero
}

/// FRAC — fractional part
pub fn rp_frac(val: &Value) -> Value {
    let n = val.to_f64();
    v_dbl(n - (n as i64) as f64)
}

/// CINT — round to nearest integer
pub fn rp_cint(val: &Value) -> Value {
    v_int(val.to_f64().round() as i64)
}

/// CLNG — round to nearest long integer (same as CINT in Rust)
pub fn rp_clng(val: &Value) -> Value {
    v_int(val.to_f64().round() as i64)
}

/// CDBL — convert to double
pub fn rp_cdbl(val: &Value) -> Value {
    v_dbl(val.to_f64())
}

/// CSNG — convert to single (still stored as f64)
pub fn rp_csng(val: &Value) -> Value {
    v_dbl(val.to_f64())
}

/// IIF — inline if (both branches are pre-evaluated, matching BASIC semantics)
pub fn rp_iif(condition: &Value, true_val: &Value, false_val: &Value) -> Value {
    if condition.to_bool() {
        true_val.clone()
    } else {
        false_val.clone()
    }
}

/// HEXTODEC — convert hex string to decimal integer
pub fn rp_hextodec(val: &Value) -> Value {
    let mut s = val.to_string_val().trim().to_uppercase();
    if s.starts_with("&H") {
        s = s[2..].to_string();
    } else if s.starts_with("0X") {
        s = s[2..].to_string();
    }
    match i64::from_str_radix(&s, 16) {
        Ok(n) => v_int(n),
        Err(_) => v_int(0),
    }
}

/// CONVBASE$ — convert number string between bases
pub fn rp_convbase(num_str: &Value, from_base: &Value, to_base: &Value) -> Value {
    let s = num_str.to_string_val();
    let from = from_base.to_i64() as u32;
    let to = to_base.to_i64() as u32;
    if !(2..=36).contains(&from) || !(2..=36).contains(&to) {
        return v_str("");
    }
    let decimal = match i64::from_str_radix(s.trim(), from) {
        Ok(n) => n,
        Err(_) => return v_str(""),
    };
    match to {
        10 => Value::String(decimal.to_string()),
        16 => Value::String(format!("{:X}", decimal)),
        8 => Value::String(format!("{:o}", decimal)),
        2 => Value::String(format!("{:b}", decimal)),
        _ => {
            // General base conversion
            if decimal == 0 {
                return v_str("0");
            }
            let negative = decimal < 0;
            let mut n = decimal.unsigned_abs();
            let digits = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
            let mut result = Vec::new();
            while n > 0 {
                result.push(digits[(n % to as u64) as usize]);
                n /= to as u64;
            }
            if negative {
                result.push(b'-');
            }
            result.reverse();
            Value::String(String::from_utf8_lossy(&result).into_owned())
        }
    }
}

/// RGB — create a BGR color integer
pub fn rp_rgb(r: &Value, g: &Value, b: &Value) -> Value {
    let r = r.to_i64() & 0xFF;
    let g = g.to_i64() & 0xFF;
    let b = b.to_i64() & 0xFF;
    v_int((b << 16) | (g << 8) | r)
}

/// DATE$ — current date as MM-DD-YYYY
pub fn rp_date() -> Value {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Simple date calculation — days since epoch
    let days = now / 86400;
    let (y, m, d) = days_to_ymd(days as i64 + 719468); // days from year 0 to unix epoch
    Value::String(format!("{:02}-{:02}-{:04}", m, d, y))
}

/// TIME$ — current time as HH:MM:SS
pub fn rp_time() -> Value {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let time_of_day = now % 86400;
    let h = time_of_day / 3600;
    let m = (time_of_day % 3600) / 60;
    let s = time_of_day % 60;
    Value::String(format!("{:02}:{:02}:{:02}", h, m, s))
}

/// Civil date from day count (algorithm from Howard Hinnant).
fn days_to_ymd(z: i64) -> (i64, u32, u32) {
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// LBOUND — lower bound of array (always 0 in Rust)
pub fn rp_lbound(_arr: &[Value]) -> Value {
    v_int(0)
}

/// UBOUND — upper bound of array
pub fn rp_ubound(arr: &[Value]) -> Value {
    v_int(if arr.is_empty() { 0 } else { (arr.len() - 1) as i64 })
}

/// VARTYPE — return type code for a value
pub fn rp_vartype(val: &Value) -> Value {
    v_int(match val {
        Value::Integer(_) => 2,
        Value::Double(_) => 5,
        Value::String(_) => 8,
        Value::Boolean(_) => 11,
        Value::Null => 0,
    })
}

/// SIZEOF — return approximate size in bytes
pub fn rp_sizeof(val: &Value) -> Value {
    v_int(match val {
        Value::Integer(_) => 8,
        Value::Double(_) => 8,
        Value::String(s) => s.len() as i64,
        Value::Boolean(_) => 1,
        Value::Null => 0,
    })
}

/// VARPTR — return a dummy pointer value (no real pointers in safe Rust)
pub fn rp_varptr(_val: &Value) -> Value {
    v_int(0)
}

/// VARPTR$ — return a string representation of a variable's address
pub fn rp_varptr_str(val: &Value) -> Value {
    v_str(&format!("0x{:016x}", val as *const Value as usize))
}

// ---------------------------------------------------------------------------
// Type checking
// ---------------------------------------------------------------------------

pub fn rp_isnumeric(val: &Value) -> Value {
    match val {
        Value::Integer(_) | Value::Double(_) => v_int(-1),
        Value::String(s) => v_int(if s.parse::<f64>().is_ok() { -1 } else { 0 }),
        _ => v_int(0),
    }
}

// ---------------------------------------------------------------------------
// Misc
// ---------------------------------------------------------------------------

pub fn rp_timer() -> Value {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    v_dbl(dur.as_secs_f64())
}

pub fn rp_sleep(ms: &Value) {
    std::thread::sleep(std::time::Duration::from_millis(ms.to_i64().max(0) as u64));
}

pub fn rp_command() -> Value {
    Value::String(std::env::args().skip(1).collect::<Vec<_>>().join(" "))
}

pub fn rp_environ(name: &Value) -> Value {
    match std::env::var(name.to_string_val()) {
        Ok(v) => Value::String(v),
        Err(_) => v_str(""),
    }
}

pub fn rp_doevents() {
    // no-op in console mode
}

pub fn rp_end() {
    std::process::exit(0);
}

pub fn rp_showmessage(msg: &Value) {
    println!("[SHOWMESSAGE] {}", msg.to_string_val());
}

pub fn rp_msgbox(msg: &Value) -> Value {
    println!("[MSGBOX] {}", msg.to_string_val());
    v_int(0)
}

// Filesystem functions — now in file_io.rs, but keep these thin wrappers
// for backward compatibility with existing codegen output.
pub fn rp_direxists(path: &Value) -> Value {
    v_int(if std::path::Path::new(&path.to_string_val()).is_dir() { -1 } else { 0 })
}

pub fn rp_fileexists(path: &Value) -> Value {
    v_int(if std::path::Path::new(&path.to_string_val()).is_file() { -1 } else { 0 })
}

// Default value for types
pub fn rp_default_for_type(type_name: &str) -> Value {
    match type_name.to_uppercase().as_str() {
        "INTEGER" | "BYTE" | "WORD" | "DWORD" | "LONG" | "INT64" => v_int(0),
        "DOUBLE" | "SINGLE" | "CURRENCY" => v_dbl(0.0),
        "STRING" => v_str(""),
        _ => v_null(),
    }
}

// ---------------------------------------------------------------------------
// Additional string functions (Phase 3c)
// ---------------------------------------------------------------------------

/// INSERT$ — insert substring at 1-based position
pub fn rp_insert(s: &Value, pos: &Value, substr: &Value) -> Value {
    let mut s = s.to_string_val();
    let pos = (pos.to_i64() - 1).max(0) as usize;
    let pos = pos.min(s.len());
    s.insert_str(pos, &substr.to_string_val());
    Value::String(s)
}

/// DELETE$ — delete count characters starting at 1-based position
pub fn rp_delete(s: &Value, start: &Value, count: &Value) -> Value {
    let s = s.to_string_val();
    let start = (start.to_i64() - 1).max(0) as usize;
    let count = count.to_i64().max(0) as usize;
    if start >= s.len() {
        return Value::String(s);
    }
    let end = (start + count).min(s.len());
    let mut result = s[..start].to_string();
    result.push_str(&s[end..]);
    Value::String(result)
}

/// REVERSE$ — reverse a string
pub fn rp_reverse(s: &Value) -> Value {
    Value::String(s.to_string_val().chars().rev().collect())
}

/// FIELD$ — return the nth field (1-based) split by delimiter
pub fn rp_field(s: &Value, delim: &Value, n: &Value) -> Value {
    let s = s.to_string_val();
    let delim = delim.to_string_val();
    let n = n.to_i64();
    let parts: Vec<&str> = s.split(&delim).collect();
    if n < 1 || n > parts.len() as i64 {
        v_str("")
    } else {
        Value::String(parts[(n - 1) as usize].to_string())
    }
}

/// TALLY — count occurrences of substring in string
pub fn rp_tally(s: &Value, substr: &Value) -> Value {
    let s = s.to_string_val();
    let sub = substr.to_string_val();
    if sub.is_empty() {
        return v_int(0);
    }
    v_int(s.matches(&sub).count() as i64)
}

/// RINSTR — find last occurrence of substring (1-based, 0 if not found)
pub fn rp_rinstr(s: &Value, substr: &Value) -> Value {
    let s = s.to_string_val();
    let sub = substr.to_string_val();
    match s.rfind(&sub) {
        Some(pos) => v_int((pos + 1) as i64),
        None => v_int(0),
    }
}

/// FORMAT$ — basic number formatting
pub fn rp_format(fmt_str: &Value, val: &Value) -> Value {
    let fmt = fmt_str.to_string_val();
    let n = val.to_f64();
    // Count decimal places in format string
    if let Some(dot_pos) = fmt.find('.') {
        let decimals = fmt.len() - dot_pos - 1;
        Value::String(format!("{:.prec$}", n, prec = decimals))
    } else {
        Value::String(format!("{}", n as i64))
    }
}

/// STRF$ — convert number to string (alias for STR$)
pub fn rp_strf(val: &Value) -> Value {
    Value::String(val.to_string_val())
}

// ---------------------------------------------------------------------------
// System / Shell (Phase 3b partial — no crossterm yet)
// ---------------------------------------------------------------------------

/// SHELL — execute a command asynchronously
pub fn rp_shell(command: &Value) -> Value {
    let cmd = command.to_string_val();
    match std::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .status()
    {
        Ok(status) => v_int(status.code().unwrap_or(-1) as i64),
        Err(_) => v_int(-1),
    }
}

/// SHELLWAIT — execute a command and wait, return exit code
pub fn rp_shellwait(command: &Value) -> Value {
    let cmd = command.to_string_val();
    match std::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .status()
    {
        Ok(status) => v_int(status.code().unwrap_or(-1) as i64),
        Err(_) => v_int(-1),
    }
}

/// BEEP — play a short beep sound
pub fn rp_beep() {
    #[cfg(feature = "audio")]
    {
        use std::time::Duration;
        std::thread::spawn(|| {
            if let Ok((_stream, handle)) = rodio::OutputStream::try_default() {
                let source = rodio::source::SineWave::new(800.0)
                    .take_duration(Duration::from_millis(200));
                let _ = handle.play_raw(rodio::source::Source::convert_samples(source));
                std::thread::sleep(Duration::from_millis(220));
            }
        });
    }
    #[cfg(not(feature = "audio"))]
    {
        print!("\x07");
        let _ = io::stdout().flush();
    }
}

/// SOUND(freq, duration_ms) — play a tone at the given frequency for the given duration
pub fn rp_sound(freq: &Value, duration: &Value) {
    let freq_hz = freq.to_f64();
    let dur_ms = duration.to_i64() as u64;
    #[cfg(feature = "audio")]
    {
        use std::time::Duration;
        std::thread::spawn(move || {
            if let Ok((_stream, handle)) = rodio::OutputStream::try_default() {
                let source = rodio::source::SineWave::new(freq_hz as f32)
                    .take_duration(Duration::from_millis(dur_ms));
                let _ = handle.play_raw(rodio::source::Source::convert_samples(source));
                std::thread::sleep(Duration::from_millis(dur_ms + 20));
            }
        });
    }
    #[cfg(not(feature = "audio"))]
    {
        let _ = (freq_hz, dur_ms); // suppress unused warnings
    }
}

/// PLAYSOUND(filename) — play a WAV file
pub fn rp_playsound(filename: &Value) -> Value {
    let path = filename.to_string_val();
    #[cfg(feature = "audio")]
    {
        std::thread::spawn(move || {
            if let Ok((_stream, handle)) = rodio::OutputStream::try_default() {
                if let Ok(file) = std::fs::File::open(&path) {
                    let buf = std::io::BufReader::new(file);
                    if let Ok(source) = rodio::Decoder::new(buf) {
                        let sink = rodio::Sink::try_new(&handle).unwrap();
                        sink.append(source);
                        sink.sleep_until_end();
                    } else {
                        eprintln!("[ERROR] PlaySound: unsupported audio format: {}", path);
                    }
                } else {
                    eprintln!("[ERROR] PlaySound: file not found: {}", path);
                }
            }
        });
    }
    #[cfg(not(feature = "audio"))]
    {
        let _ = path;
        eprintln!("[WARN] PlaySound: audio feature not enabled");
    }
    v_null()
}

// ---------------------------------------------------------------------------
// Constants — exposed as functions for codegen convenience
// ---------------------------------------------------------------------------

pub fn rp_const_true() -> Value { v_bool(true) }
pub fn rp_const_false() -> Value { v_bool(false) }

// Color constants (BGR byte order matching BASIC convention)
pub const CL_BLACK: i64 = 0x000000;
pub const CL_MAROON: i64 = 0x000080;
pub const CL_GREEN: i64 = 0x008000;
pub const CL_OLIVE: i64 = 0x008080;
pub const CL_NAVY: i64 = 0x800000;
pub const CL_PURPLE: i64 = 0x800080;
pub const CL_TEAL: i64 = 0x808000;
pub const CL_GRAY: i64 = 0x808080;
pub const CL_SILVER: i64 = 0xC0C0C0;
pub const CL_RED: i64 = 0x0000FF;
pub const CL_LIME: i64 = 0x00FF00;
pub const CL_YELLOW: i64 = 0x00FFFF;
pub const CL_BLUE: i64 = 0xFF0000;
pub const CL_FUCHSIA: i64 = 0xFF00FF;
pub const CL_AQUA: i64 = 0xFFFF00;
pub const CL_WHITE: i64 = 0xFFFFFF;

// Virtual key constants
pub const VK_LEFT: i64 = 37;
pub const VK_UP: i64 = 38;
pub const VK_RIGHT: i64 = 39;
pub const VK_DOWN: i64 = 40;
pub const VK_RETURN: i64 = 13;
pub const VK_ESCAPE: i64 = 27;
pub const VK_SPACE: i64 = 32;
pub const VK_TAB: i64 = 9;
pub const VK_DELETE: i64 = 46;
pub const VK_BACK: i64 = 8;
pub const VK_F1: i64 = 112;
pub const VK_F2: i64 = 113;
pub const VK_F3: i64 = 114;
pub const VK_F4: i64 = 115;
pub const VK_F5: i64 = 116;
pub const VK_F6: i64 = 117;
pub const VK_F7: i64 = 118;
pub const VK_F8: i64 = 119;
pub const VK_F9: i64 = 120;
pub const VK_F10: i64 = 121;
pub const VK_F11: i64 = 122;
pub const VK_F12: i64 = 123;

// MessageBox constants
pub const MB_OK: i64 = 0;
pub const MB_OKCANCEL: i64 = 1;
pub const MB_YESNOCANCEL: i64 = 3;
pub const MB_YESNO: i64 = 4;
pub const IDOK: i64 = 1;
pub const IDCANCEL: i64 = 2;
pub const IDYES: i64 = 6;
pub const IDNO: i64 = 7;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_len() {
        assert_eq!(rp_len(&v_str("hello")), v_int(5));
    }

    #[test]
    fn test_mid() {
        assert_eq!(rp_mid(&v_str("Hello World"), &v_int(7), &v_int(5)), v_str("World"));
    }

    #[test]
    fn test_hex() {
        assert_eq!(rp_hex(&v_int(255)), v_str("FF"));
    }

    #[test]
    fn test_ucase_lcase() {
        assert_eq!(rp_ucase(&v_str("hello")), v_str("HELLO"));
        assert_eq!(rp_lcase(&v_str("HELLO")), v_str("hello"));
    }

    #[test]
    fn test_math() {
        assert_eq!(rp_abs(&v_int(-5)), v_int(5));
        assert_eq!(rp_sgn(&v_int(-3)), v_int(-1));
        assert_eq!(rp_ceil(&v_dbl(2.3)).to_f64(), 3.0);
    }

    #[test]
    fn test_direxists() {
        assert_eq!(rp_direxists(&v_str(".")), v_int(-1));
        assert_eq!(rp_direxists(&v_str("NONEXISTENT_DIR_12345")), v_int(0));
    }

    #[test]
    fn test_fix_frac() {
        assert_eq!(rp_fix(&v_dbl(-3.7)), v_int(-3));
        assert_eq!(rp_fix(&v_dbl(3.7)), v_int(3));
        let frac = rp_frac(&v_dbl(3.75)).to_f64();
        assert!((frac - 0.75).abs() < 1e-10);
    }

    #[test]
    fn test_cint_clng() {
        assert_eq!(rp_cint(&v_dbl(3.6)), v_int(4));
        assert_eq!(rp_cint(&v_dbl(3.4)), v_int(3));
        assert_eq!(rp_clng(&v_dbl(-2.7)), v_int(-3));
    }

    #[test]
    fn test_iif() {
        assert_eq!(rp_iif(&v_bool(true), &v_str("yes"), &v_str("no")), v_str("yes"));
        assert_eq!(rp_iif(&v_bool(false), &v_str("yes"), &v_str("no")), v_str("no"));
    }

    #[test]
    fn test_hextodec() {
        assert_eq!(rp_hextodec(&v_str("FF")), v_int(255));
        assert_eq!(rp_hextodec(&v_str("&HFF")), v_int(255));
        assert_eq!(rp_hextodec(&v_str("0xFF")), v_int(255));
    }

    #[test]
    fn test_convbase() {
        assert_eq!(rp_convbase(&v_str("255"), &v_int(10), &v_int(16)), v_str("FF"));
        assert_eq!(rp_convbase(&v_str("FF"), &v_int(16), &v_int(10)), v_str("255"));
        assert_eq!(rp_convbase(&v_str("10"), &v_int(10), &v_int(2)), v_str("1010"));
    }

    #[test]
    fn test_insert_delete() {
        assert_eq!(rp_insert(&v_str("Hello World"), &v_int(6), &v_str(" Beautiful")), v_str("Hello Beautiful World"));
        assert_eq!(rp_delete(&v_str("Hello World"), &v_int(6), &v_int(1)), v_str("HelloWorld"));
    }

    #[test]
    fn test_reverse() {
        assert_eq!(rp_reverse(&v_str("Hello")), v_str("olleH"));
    }

    #[test]
    fn test_field() {
        assert_eq!(rp_field(&v_str("one,two,three"), &v_str(","), &v_int(2)), v_str("two"));
        assert_eq!(rp_field(&v_str("one,two,three"), &v_str(","), &v_int(4)), v_str(""));
    }

    #[test]
    fn test_tally() {
        assert_eq!(rp_tally(&v_str("hello world hello"), &v_str("hello")), v_int(2));
    }

    #[test]
    fn test_rinstr() {
        assert_eq!(rp_rinstr(&v_str("hello world hello"), &v_str("hello")), v_int(13));
        assert_eq!(rp_rinstr(&v_str("hello"), &v_str("xyz")), v_int(0));
    }

    #[test]
    fn test_format() {
        assert_eq!(rp_format(&v_str("#.##"), &v_dbl(3.14159)), v_str("3.14"));
    }

    #[test]
    fn test_rnd() {
        let r = rp_rnd(&v_int(0));
        let f = r.to_f64();
        assert!(f >= 0.0 && f < 1.0);

        let r2 = rp_rnd(&v_int(10));
        let n = r2.to_i64();
        assert!(n >= 0 && n < 10);
    }

    #[test]
    fn test_rgb() {
        assert_eq!(rp_rgb(&v_int(255), &v_int(0), &v_int(0)), v_int(255)); // red
        assert_eq!(rp_rgb(&v_int(0), &v_int(255), &v_int(0)), v_int(0xFF00)); // green
    }

    #[test]
    fn test_vartype() {
        assert_eq!(rp_vartype(&v_int(0)), v_int(2));
        assert_eq!(rp_vartype(&v_dbl(0.0)), v_int(5));
        assert_eq!(rp_vartype(&v_str("")), v_int(8));
    }

    #[test]
    fn test_date_time() {
        let d = rp_date().to_string_val();
        assert_eq!(d.len(), 10); // MM-DD-YYYY
        let t = rp_time().to_string_val();
        assert_eq!(t.len(), 8); // HH:MM:SS
    }
}
