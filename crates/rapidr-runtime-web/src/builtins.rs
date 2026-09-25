//! BASIC builtin functions — web/WASM implementations.
//!
//! Pure-Rust functions (math, string) are identical to the desktop runtime.
//! Platform-specific functions (PRINT, INPUT, SHELL, SLEEP, BEEP, SOUND, etc.)
//! are replaced with web-compatible equivalents using web-sys / js-sys.

use crate::value::{v_bool, v_dbl, v_int, v_null, v_str, Value};
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// PRINT — outputs to the browser console and to a #rr-console element if present
// ---------------------------------------------------------------------------

pub fn rp_print(items: &[Value], newline: bool) {
    let mut parts = Vec::new();
    for item in items {
        parts.push(item.to_string_val());
    }
    let text = parts.join(" ");
    let msg = if newline {
        format!("{}\n", text)
    } else {
        text
    };
    track_print_column(&msg);

    // console.log is line-oriented: log complete lines, keep a partial one
    // (`PRINT "a";`) buffered until its newline, or until the current
    // synchronous code finishes (see schedule_output_flush).
    let complete: Vec<String> = LINE_BUF.with(|b| {
        let mut b = b.borrow_mut();
        b.push_str(&msg);
        let mut lines = Vec::new();
        while let Some(i) = b.find('\n') {
            let line: String = b.drain(..=i).collect();
            lines.push(line[..line.len() - 1].to_string());
        }
        lines
    });
    for line in complete {
        web_sys::console::log_1(&JsValue::from_str(&line));
    }
    if LINE_BUF.with(|b| !b.borrow().is_empty()) {
        schedule_output_flush();
    }

    // Also append to #rr-console element if it exists
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(el) = document.get_element_by_id("rr-console") {
                // Append as text nodes (never markup) so printed data can't
                // inject HTML; spaces/newlines render as before.
                for (i, line) in msg.split('\n').enumerate() {
                    if i > 0 {
                        if let Ok(br) = document.create_element("br") {
                            let _ = el.append_child(&br);
                        }
                    }
                    if !line.is_empty() {
                        let text = document.create_text_node(&line.replace(' ', "\u{00A0}"));
                        let _ = el.append_child(&text);
                    }
                }
            }
        }
    }
}

/// Width of a PRINT zone (`PRINT a, b`), as in QBasic and VB.
pub const PRINT_ZONE_WIDTH: usize = 14;

thread_local! {
    /// Output cursor column (chars since the last newline), for print zones.
    static PRINT_COL: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// Text printed since the last newline, not yet sent to console.log.
    static LINE_BUF: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
    static FLUSH_SCHEDULED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn track_print_column(text: &str) {
    PRINT_COL.with(|c| {
        c.set(match text.rfind('\n') {
            Some(i) => text[i + 1..].chars().count(),
            None => c.get() + text.chars().count(),
        })
    });
}

/// `,` in PRINT: pad with spaces to the next print zone.
pub fn rp_print_zone() {
    let pad = PRINT_ZONE_WIDTH - PRINT_COL.with(|c| c.get()) % PRINT_ZONE_WIDTH;
    rp_print(&[Value::String(" ".repeat(pad))], false);
}

/// Send any buffered partial line to console.log.
pub fn rp_flush_output() {
    let pending = LINE_BUF.with(|b| std::mem::take(&mut *b.borrow_mut()));
    if !pending.is_empty() {
        web_sys::console::log_1(&JsValue::from_str(&pending));
    }
}

/// Flush a partial line once the current synchronous work (the program's
/// main, or an event handler) has finished.
fn schedule_output_flush() {
    if FLUSH_SCHEDULED.with(|f| f.replace(true)) {
        return;
    }
    let flush = Closure::once_into_js(|| {
        FLUSH_SCHEDULED.with(|f| f.set(false));
        rp_flush_output();
    });
    if let Some(window) = web_sys::window() {
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(flush.unchecked_ref(), 0);
    }
}

// ---------------------------------------------------------------------------
// INPUT — uses window.prompt() on web
// ---------------------------------------------------------------------------

pub fn rp_input(prompt: &Value) -> Value {
    let p = prompt.to_string_val();
    if let Some(window) = web_sys::window() {
        match window.prompt_with_message(&p) {
            Ok(Some(s)) => Value::String(s),
            _ => Value::String(String::new()),
        }
    } else {
        Value::String(String::new())
    }
}

// ---------------------------------------------------------------------------
// String functions — identical to desktop (pure Rust)
// ---------------------------------------------------------------------------

pub fn rp_len(val: &Value) -> Value {
    rapidr_value::strings::len(val)
}

pub fn rp_mid(s: &Value, start: &Value, length: &Value) -> Value {
    rapidr_value::strings::mid(s, start, length)
}

pub fn rp_left(s: &Value, n: &Value) -> Value {
    rapidr_value::strings::left(s, n)
}

pub fn rp_right(s: &Value, n: &Value) -> Value {
    rapidr_value::strings::right(s, n)
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
    rapidr_value::strings::instr(start, haystack, needle)
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
    rapidr_value::strings::asc(s)
}

pub fn rp_replace(s: &Value, old: &Value, new: &Value) -> Value {
    Value::String(s.to_string_val().replace(&old.to_string_val(), &new.to_string_val()))
}

// ---------------------------------------------------------------------------
// Numeric / conversion functions — identical to desktop (pure Rust)
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
    let s = val.to_string_val();
    if s.contains(',') {
        let result: String = s
            .split(',')
            .map(|v| v.trim().parse::<f64>().unwrap_or(0.0).sin().to_string())
            .collect::<Vec<_>>()
            .join(",");
        v_str(&result)
    } else {
        v_dbl(val.to_f64().sin())
    }
}

pub fn rp_cos(val: &Value) -> Value {
    let s = val.to_string_val();
    if s.contains(',') {
        let result: String = s
            .split(',')
            .map(|v| v.trim().parse::<f64>().unwrap_or(0.0).cos().to_string())
            .collect::<Vec<_>>()
            .join(",");
        v_str(&result)
    } else {
        v_dbl(val.to_f64().cos())
    }
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
    let u = upper.to_i64();
    let random: f64 = js_sys::Math::random();
    if u > 0 {
        v_int((random * u as f64) as i64)
    } else {
        v_dbl(random)
    }
}

pub fn rp_randomize(_seed: &Value) {
    // js_sys::Math::random() is auto-seeded; no-op
}

pub fn rp_randint(low: &Value, high: &Value, size: &Value) -> Value {
    let lo = low.to_i64();
    let hi = high.to_i64();
    let n = size.to_i64().max(0) as usize;
    let range = (hi - lo + 1) as f64;
    let vals: Vec<String> = (0..n)
        .map(|_| {
            let r = js_sys::Math::random();
            (lo + (r * range) as i64).to_string()
        })
        .collect();
    v_str(&vals.join(","))
}

// ---------------------------------------------------------------------------
// Additional math / conversion (Phase 3d) — pure Rust, identical to desktop
// ---------------------------------------------------------------------------

pub fn rp_fix(val: &Value) -> Value {
    let n = val.to_f64();
    v_int(n as i64)
}

pub fn rp_frac(val: &Value) -> Value {
    let n = val.to_f64();
    v_dbl(n - (n as i64) as f64)
}

pub fn rp_cint(val: &Value) -> Value {
    v_int(val.to_f64().round() as i64)
}

pub fn rp_clng(val: &Value) -> Value {
    v_int(val.to_f64().round() as i64)
}

pub fn rp_cdbl(val: &Value) -> Value {
    v_dbl(val.to_f64())
}

pub fn rp_csng(val: &Value) -> Value {
    v_dbl(val.to_f64())
}

pub fn rp_iif(condition: &Value, true_val: &Value, false_val: &Value) -> Value {
    if condition.to_bool() {
        true_val.clone()
    } else {
        false_val.clone()
    }
}

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

pub fn rp_rgb(r: &Value, g: &Value, b: &Value) -> Value {
    let r = r.to_i64() & 0xFF;
    let g = g.to_i64() & 0xFF;
    let b = b.to_i64() & 0xFF;
    v_int((b << 16) | (g << 8) | r)
}

// ---------------------------------------------------------------------------
// Date / Time — use js_sys::Date on WASM
// ---------------------------------------------------------------------------

pub fn rp_date() -> Value {
    let d = js_sys::Date::new_0();
    let m = d.get_month() + 1; // JS months are 0-based
    let day = d.get_date();
    let y = d.get_full_year();
    Value::String(format!("{:02}-{:02}-{:04}", m, day, y))
}

pub fn rp_time() -> Value {
    let d = js_sys::Date::new_0();
    let h = d.get_hours();
    let m = d.get_minutes();
    let s = d.get_seconds();
    Value::String(format!("{:02}:{:02}:{:02}", h, m, s))
}

// ---------------------------------------------------------------------------
// Array helpers — identical to desktop
// ---------------------------------------------------------------------------

pub fn rp_lbound(_arr: &[Value]) -> Value {
    v_int(0)
}

pub fn rp_ubound(arr: &[Value]) -> Value {
    v_int(if arr.is_empty() {
        0
    } else {
        (arr.len() - 1) as i64
    })
}

pub fn rp_vartype(val: &Value) -> Value {
    v_int(match val {
        Value::Integer(_) => 2,
        Value::Double(_) => 5,
        Value::String(_) => 8,
        Value::Boolean(_) => 11,
        Value::Null => 0,
        Value::Array(_) => 8204, // vbArray + vbVariant, as in VB
    })
}

pub fn rp_sizeof(val: &Value) -> Value {
    v_int(match val {
        Value::Integer(_) => 8,
        Value::Double(_) => 8,
        Value::String(s) => s.len() as i64,
        Value::Boolean(_) => 1,
        Value::Null => 0,
        Value::Array(a) => 8 * a.borrow().data.len() as i64,
    })
}

pub fn rp_varptr(_val: &Value) -> Value {
    v_int(0)
}

pub fn rp_varptr_str(_val: &Value) -> Value {
    v_str("0x0000000000000000") // no real pointers in WASM
}

pub fn rp_isnumeric(val: &Value) -> Value {
    match val {
        Value::Integer(_) | Value::Double(_) => v_int(-1),
        Value::String(s) => v_int(if s.parse::<f64>().is_ok() { -1 } else { 0 }),
        _ => v_int(0),
    }
}

// ---------------------------------------------------------------------------
// Timer — uses performance.now() on web
// ---------------------------------------------------------------------------

pub fn rp_timer() -> Value {
    if let Some(window) = web_sys::window() {
        if let Ok(perf) = window.performance().ok_or(()) {
            return v_dbl(perf.now() / 1000.0);
        }
    }
    v_dbl(0.0)
}

// ---------------------------------------------------------------------------
// Sleep — NOT SUPPORTED on web (WASM is single-threaded)
// ---------------------------------------------------------------------------

pub fn rp_sleep(_ms: &Value) {
    web_sys::console::warn_1(&JsValue::from_str(
        "[WARN] SLEEP is not supported in WASM — use timers instead",
    ));
}

// ---------------------------------------------------------------------------
// System / Shell — NOT SUPPORTED on web
// ---------------------------------------------------------------------------

pub fn rp_command() -> Value {
    // Return URL query string as "command line"
    if let Some(window) = web_sys::window() {
        if let Ok(loc) = window.location().search() {
            if loc.len() > 1 {
                return Value::String(loc[1..].to_string());
            }
        }
    }
    Value::String(String::new())
}

pub fn rp_environ(_name: &Value) -> Value {
    v_str("") // not available on web
}

pub fn rp_doevents() {
    // no-op on web — browser handles event loop
}

pub fn rp_end() {
    web_sys::console::log_1(&JsValue::from_str("[RapidR] Program ended."));
}

pub fn rp_showmessage(msg: &Value) {
    if let Some(window) = web_sys::window() {
        let _ = window.alert_with_message(&msg.to_string_val());
    }
}

pub fn rp_msgbox(msg: &Value) -> Value {
    rp_showmessage(msg);
    v_int(0)
}

pub fn rp_shell(_command: &Value) -> Value {
    web_sys::console::warn_1(&JsValue::from_str(
        "[WARN] SHELL is not supported in WASM",
    ));
    v_int(-1)
}

pub fn rp_shellwait(_command: &Value) -> Value {
    web_sys::console::warn_1(&JsValue::from_str(
        "[WARN] SHELLWAIT is not supported in WASM",
    ));
    v_int(-1)
}

pub fn rp_beep() {
    play_tone(800.0, 200.0);
}

pub fn rp_sound(freq: &Value, duration: &Value) {
    play_tone(freq.to_f64(), duration.to_f64());
}

thread_local! {
    // Browsers cap the number of live AudioContexts, so share one.
    static AUDIO_CTX: std::cell::RefCell<Option<web_sys::AudioContext>> =
        const { std::cell::RefCell::new(None) };
}

/// Plays a sine tone via Web Audio. Failures (no audio support, autoplay
/// policy) are silently ignored, matching desktop BEEP semantics.
fn play_tone(freq_hz: f64, dur_ms: f64) {
    let ctx = AUDIO_CTX.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            *slot = web_sys::AudioContext::new().ok();
        }
        slot.clone()
    });
    let Some(ctx) = ctx else { return };
    let Ok(osc) = ctx.create_oscillator() else { return };
    osc.frequency().set_value(freq_hz as f32);
    if osc.connect_with_audio_node(&ctx.destination()).is_err() {
        return;
    }
    let now = ctx.current_time();
    let _ = osc.start_with_when(now);
    let _ = osc.stop_with_when(now + dur_ms.max(0.0) / 1000.0);
}

pub fn rp_playsound(filename: &Value) -> Value {
    let src = filename.to_string_val();
    if let Ok(audio) = web_sys::HtmlAudioElement::new_with_src(&src) {
        let _ = audio.play();
    }
    v_null()
}

// ---------------------------------------------------------------------------
// File system stubs — NOT SUPPORTED on web
// ---------------------------------------------------------------------------

pub fn rp_direxists(_path: &Value) -> Value {
    v_int(0) // directories don't exist on web
}

pub fn rp_fileexists(_path: &Value) -> Value {
    v_int(0) // files don't exist on web
}

pub fn rp_default_for_type(type_name: &str) -> Value {
    match type_name.to_uppercase().as_str() {
        "INTEGER" | "BYTE" | "WORD" | "DWORD" | "LONG" | "INT64" => v_int(0),
        "DOUBLE" | "SINGLE" | "CURRENCY" => v_dbl(0.0),
        "STRING" => v_str(""),
        _ => v_null(),
    }
}

// ---------------------------------------------------------------------------
// Additional string functions — identical to desktop (pure Rust)
// ---------------------------------------------------------------------------

pub fn rp_insert(s: &Value, pos: &Value, substr: &Value) -> Value {
    rapidr_value::strings::insert(s, pos, substr)
}

pub fn rp_delete(s: &Value, start: &Value, count: &Value) -> Value {
    rapidr_value::strings::delete(s, start, count)
}

pub fn rp_reverse(s: &Value) -> Value {
    Value::String(s.to_string_val().chars().rev().collect())
}

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

pub fn rp_tally(s: &Value, substr: &Value) -> Value {
    let s = s.to_string_val();
    let sub = substr.to_string_val();
    if sub.is_empty() {
        return v_int(0);
    }
    v_int(s.matches(&sub).count() as i64)
}

/// REPLACESUBSTR$ — replace every occurrence of a substring
pub fn rp_replacesubstr(s: &Value, find: &Value, replacement: &Value) -> Value {
    rapidr_value::strings::replace_all(s, find, replacement)
}

pub fn rp_rinstr(s: &Value, substr: &Value) -> Value {
    rapidr_value::strings::rinstr(s, substr)
}

pub fn rp_format(fmt_str: &Value, val: &Value) -> Value {
    let fmt = fmt_str.to_string_val();
    let n = val.to_f64();
    if let Some(dot_pos) = fmt.find('.') {
        let decimals = fmt.len() - dot_pos - 1;
        Value::String(format!("{:.prec$}", n, prec = decimals))
    } else {
        Value::String(format!("{}", n as i64))
    }
}

pub fn rp_strf(val: &Value) -> Value {
    Value::String(val.to_string_val())
}

// ---------------------------------------------------------------------------
// Constants — identical to desktop
// ---------------------------------------------------------------------------

pub fn rp_const_true() -> Value {
    v_bool(true)
}
pub fn rp_const_false() -> Value {
    v_bool(false)
}

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

pub const MB_OK: i64 = 0;
pub const MB_OKCANCEL: i64 = 1;
pub const MB_YESNOCANCEL: i64 = 3;
pub const MB_YESNO: i64 = 4;
pub const IDOK: i64 = 1;
pub const IDCANCEL: i64 = 2;
pub const IDYES: i64 = 6;
pub const IDNO: i64 = 7;
