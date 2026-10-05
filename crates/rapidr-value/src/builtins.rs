//! BASIC builtins that are the same everywhere (strings, math, conversions):
//! one implementation for native builds, the interpreter and the web, so
//! they can't drift. The runtimes re-export these next to their own
//! platform builtins (console, files, time, sound).

use crate::{v_dbl, v_int, v_null, v_str, Value};

pub fn rp_len(val: &Value) -> Value {
    crate::strings::len(val)
}

pub fn rp_mid(s: &Value, start: &Value, length: &Value) -> Value {
    crate::strings::mid(s, start, length)
}

pub fn rp_left(s: &Value, n: &Value) -> Value {
    crate::strings::left(s, n)
}

pub fn rp_right(s: &Value, n: &Value) -> Value {
    crate::strings::right(s, n)
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
    crate::strings::instr(start, haystack, needle)
}

pub fn rp_space(n: &Value) -> Value {
    crate::strings::space(n)
}

pub fn rp_string_func(n: &Value, ch: &Value) -> Value {
    crate::strings::string_of(n, ch)
}

/// `CHR$(n)`: the character of code n's low byte — a real code rounded half
/// to even first, as RapidQ's (`CHR$(65.7)` is "B", `CHR$(66.5)` "B").
pub fn rp_chr(n: &Value) -> Value {
    let code = match n {
        Value::Double(d) => crate::format::int32(*d),
        other => other.to_i64(),
    };
    Value::String(String::from(char::from(code as u8)))
}

pub fn rp_asc(s: &Value) -> Value {
    crate::strings::asc(s)
}

/// `REPLACE$(source, replacement, index)`: RapidQ's — `replacement` written
/// over `source` from character `index` (1 the first): `REPLACE$("Hello",
/// "J", 1)` is "Jello". As RC.EXE shows, it is the text before `index`, the
/// replacement, then what follows the replaced characters: past the end it
/// is appended (`REPLACE$("abc", "Z", 9)` is "abcZ"), at 0 or before it goes
/// in front (`"Zabc"`). (Find and replace is REPLACESUBSTR$.)
pub fn rp_replace(s: &Value, replacement: &Value, index: &Value) -> Value {
    let chars: Vec<char> = s.to_string_val().chars().collect();
    let rep = replacement.to_string_val();
    let index = index.to_i64();
    // Delphi's Copy(s, 1, index - 1) + r + Copy(s, index + Length(r), MaxInt).
    let before = (index - 1).clamp(0, chars.len() as i64) as usize;
    let after = (index + rep.chars().count() as i64).max(1);
    let after = ((after - 1) as usize).min(chars.len());
    let mut out: String = chars[..before].iter().collect();
    out.push_str(&rep);
    out.extend(&chars[after..]);
    Value::String(out)
}

/// `STR$(n)`: RapidQ's 9 significant digits (`crate::format::str_number`).
pub fn rp_str(val: &Value) -> Value {
    match val {
        Value::Integer(_) | Value::Double(_) | Value::Boolean(_) => Value::String(crate::format::str_number(val.to_f64())),
        _ => Value::String(val.to_string_val()),
    }
}

/// `PRINT TAB(n)`: spaces to column n of the console (1 is the first);
/// past it already, column n of the next line (as QBasic's TAB).
pub fn rp_tab(n: &Value) -> Value {
    let n = n.to_i64().clamp(1, 32_767);
    let col = crate::console::pos().to_i64().max(1);
    if col <= n {
        v_str(&" ".repeat((n - col) as usize))
    } else {
        v_str(&format!("\n{}", " ".repeat((n - 1) as usize)))
    }
}

/// `QUICKSORT(A(first), A(last), ASCEND|DESCEND)` (rapidr_ast::quicksort):
/// sorts A's elements from `first` to `last` in place — numbers by value,
/// strings by their characters; `start_end` holds both elements' indices.
pub fn rp_quicksort(array: &Value, descend: &Value, start_end: &[i64]) -> Value {
    let Value::Array(a) = array else { return v_null() };
    let mut a = a.borrow_mut();
    let n = a.bounds.len();
    if start_end.len() != 2 * n {
        return v_null();
    }
    let (Ok(mut from), Ok(mut to)) = (a.offset(&start_end[..n]), a.offset(&start_end[n..])) else { return v_null() };
    if from > to {
        std::mem::swap(&mut from, &mut to);
    }
    let numeric = |v: &Value| matches!(v, Value::Integer(_) | Value::Double(_));
    let slice = &mut a.data[from..=to];
    slice.sort_by(|x, y| {
        if numeric(x) && numeric(y) {
            x.to_f64().partial_cmp(&y.to_f64()).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            x.to_string_val().cmp(&y.to_string_val())
        }
    });
    if descend.to_i64() != 0 {
        slice.reverse();
    }
    v_null()
}

thread_local! {
    /// `$OPTION DECIMAL ","`: the character VAL takes as the decimal point.
    static DECIMAL: std::cell::Cell<char> = const { std::cell::Cell::new('.') };
}

/// `$OPTION DECIMAL ","` / `$OPTION DECIMAL 44` (the parser calls
/// `__decimal`): VAL's decimal character from then on (RapidQ manual,
/// chapter 3).
pub fn rp_set_decimal(c: &Value) -> Value {
    let ch = match c {
        Value::String(_) => c.to_string_val().chars().next(),
        _ => char::from_u32(c.to_i64().clamp(0, 0x10FFFF) as u32),
    };
    if let Some(ch) = ch.filter(|ch| *ch != '\0') {
        DECIMAL.with(|d| d.set(ch));
    }
    v_null()
}

/// QFORM's border icons after `AddBorderIcons` / `DelBorderIcons` (RapidQ's
/// biSystemMenu 0, biMinimize 1, biMaximize 2, biHelp 3), as a bit set:
/// `current` (a form starts with the first three), the icons named, adding
/// or removing them.
pub fn border_icons(current: &Value, icons: &[Value], add: bool) -> Value {
    let mut bits = match current {
        Value::Null => 0b0111,
        v => v.to_i64(),
    };
    for i in icons {
        let b = 1i64 << i.to_i64().clamp(0, 3);
        if add {
            bits |= b;
        } else {
            bits &= !b;
        }
    }
    Value::Integer(bits)
}

/// `LPRINT …` (the parser's `__lprint(newline, item, zone, …)`).
pub fn rp_lprint(args: &[Value]) -> Value {
    crate::lprint::lprint(args)
}

/// `LFLUSH`: the LPRINTed text to the printer.
pub fn rp_lflush() -> Value {
    let _ = crate::lprint::flush();
    v_null()
}

/// `$OPTION INKEY$ TRAPALL` (1) / `DEFAULT` (0) (the parser calls
/// `__inkey_trapall`).
pub fn rp_inkey_trap_all(on: &Value) -> Value {
    crate::console::set_inkey_trap_all(on.to_i64() != 0);
    v_null()
}

/// The `ENVIRON "name=text"` statement (rapidr_ast::library lowers it to
/// `__environ_set`): the program's environment (crate::environ).
pub fn rp_environ_set(spec: &Value) -> Value {
    crate::environ::set(&spec.to_string_val());
    Value::Null
}

/// `ENVIRON$(name)` (crate::environ).
pub fn rp_environ_get(name: &Value) -> Value {
    Value::String(crate::environ::get(&name.to_string_val()))
}

/// `VAL(s)` as RapidQ reads a number (RC.EXE): spaces anywhere are
/// skipped (`VAL("12 34")` is 1234, `VAL("- 5")` -5), then the longest
/// number at the start counts — a sign, digits, a decimal point (`$OPTION
/// DECIMAL`'s) and digits, an exponent only with its digits — and the rest
/// is ignored (`"12abc"` → 12, `"1.2.3"` → 1.2, `"1.5e"` → 1.5, `"1d2"` →
/// 1); no number, 0 (`"&H10"`, `"$5"`). (RapidQ stops with an error on a
/// sign without digits, `VAL("--1")`; RapidR gives 0.)
pub fn rp_val(s: &Value) -> Value {
    let decimal = DECIMAL.with(|d| d.get());
    let t: Vec<char> = s.to_string_val().chars().filter(|c| *c != ' ').collect();
    let mut i = 0;
    let mut text = String::new();
    if matches!(t.first(), Some('+' | '-')) {
        text.push(t[0]);
        i = 1;
    }
    let digits = |i: &mut usize, text: &mut String| {
        let from = *i;
        while t.get(*i).is_some_and(|c| c.is_ascii_digit()) {
            text.push(t[*i]);
            *i += 1;
        }
        *i > from
    };
    let mut any = digits(&mut i, &mut text);
    if t.get(i) == Some(&decimal) {
        text.push('.');
        i += 1;
        any |= digits(&mut i, &mut text);
    }
    if !any {
        return v_int(0);
    }
    if matches!(t.get(i), Some('e' | 'E')) {
        let mut j = i + 1;
        let mut exp = String::from("e");
        if matches!(t.get(j), Some('+' | '-')) {
            exp.push(t[j]);
            j += 1;
        }
        if digits(&mut j, &mut exp) {
            text.push_str(&exp);
        }
    }
    match text.parse::<i64>() {
        Ok(n) => v_int(n),
        Err(_) => v_dbl(text.parse::<f64>().unwrap_or(0.0)),
    }
}

/// A whole float as a number: an integer when it is one exactly, else (huge,
/// NaN, infinite) the float itself.
fn whole(t: f64) -> Value {
    if t.is_finite() && t.abs() < 9.0e15 {
        v_int(t as i64)
    } else {
        v_dbl(t)
    }
}

/// `INT(x)`: RapidQ truncates toward zero, whatever its manual says
/// ("largest integer less than or equal"): RC.EXE gives `INT(-2.5)` = -2,
/// `INT(-0.5)` = 0 — the same as FIX. The result is a float's whole number
/// (`INT(1E10) / 1E10` is 1), not a 32-bit integer.
pub fn rp_int(val: &Value) -> Value {
    match val {
        Value::Integer(n) => v_int(*n),
        _ => whole(val.to_f64().trunc()),
    }
}

/// A float as RapidQ's ROUND, CINT, CLNG, CEIL and FLOOR return it: a
/// 32-bit integer (RC.EXE: `ROUND(1E10) / 1E10` is -0.214748365 — beyond 32
/// bits, -2147483648).
fn int32_value(f: f64) -> Value {
    v_int(crate::numeric::trunc_to_int(f))
}

/// `ROUND(x)` / `CINT(x)` / `CLNG(x)` as RapidQ computes them (RC.EXE):
/// `INT(x + 0.5)`, truncating — 2.5 → 3, 0.5 → 1, -2.5 → -2, -2.2 → -1,
/// -3.99 → -3 (the manual's half-to-even claim isn't what it does).
fn round_half_up(val: &Value) -> Value {
    match val {
        Value::Integer(n) => v_int(crate::format::int32_of(*n)),
        _ => int32_value(val.to_f64() + 0.5),
    }
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
        // Vectorized: apply sin to each element of comma-separated array
        let result: String = s.split(',')
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
        // Vectorized: apply cos to each element of comma-separated array
        let result: String = s.split(',')
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

/// `CEIL(x)`: a 32-bit integer in RapidQ (`int32_value`).
pub fn rp_ceil(val: &Value) -> Value {
    int32_value(val.to_f64().ceil())
}

/// `FLOOR(x)`: a 32-bit integer in RapidQ (`int32_value`).
pub fn rp_floor(val: &Value) -> Value {
    int32_value(val.to_f64().floor())
}

/// `ROUND(x)`: see `round_half_up`.
pub fn rp_round(val: &Value) -> Value {
    round_half_up(val)
}

/// The 32 bits HEX$ / BIN$ show: a real rounded half to even, a number
/// beyond 32 bits keeps its low 32 (RC.EXE: `HEX$(3000000000)` is
/// B2D05E00, `HEX$(-1)` FFFFFFFF, `HEX$(2.7)` 00000003).
fn low32(val: &Value) -> u32 {
    let f = match val {
        Value::Integer(n) => return *n as u32,
        Value::Boolean(b) => return if *b { u32::MAX } else { 0 },
        other => other.to_f64().round_ties_even(),
    };
    if f.is_finite() && f.abs() < 9.2e18 {
        f as i64 as u32
    } else {
        0
    }
}

/// `HEX$(n)`: RapidQ for Windows pads to 8 digits (`HEX$(255)` is
/// 000000FF — its manual says so, RC.EXE shows it).
pub fn rp_hex(val: &Value) -> Value {
    Value::String(format!("{:08X}", low32(val)))
}

pub fn rp_oct(val: &Value) -> Value {
    Value::String(format!("{:o}", val.to_i64()))
}

/// `BIN$(n)`: the 32 bits, without leading zeros (`BIN$(-1)` is 32 ones).
pub fn rp_bin(val: &Value) -> Value {
    Value::String(format!("{:b}", low32(val)))
}

/// `FIX(x)`: truncated toward zero, a float's whole number (as INT).
pub fn rp_fix(val: &Value) -> Value {
    rp_int(val)
}

/// FRAC — fractional part (`FRAC(-2.75)` is -0.75)
pub fn rp_frac(val: &Value) -> Value {
    let n = val.to_f64();
    v_dbl(n - n.trunc())
}

pub fn rp_cbool(val: &Value) -> Value {
    crate::cbool(val)
}

/// `CINT(x)`: as ROUND (`round_half_up`).
pub fn rp_cint(val: &Value) -> Value {
    round_half_up(val)
}

/// `CLNG(x)`: as ROUND (`round_half_up`).
pub fn rp_clng(val: &Value) -> Value {
    round_half_up(val)
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
    // A negative number into another base: its 32 bits (RC.EXE:
    // CONVBASE$("-10", 10, 16) is FFFFFFF6 — "32 bits negative", Lib's notes).
    let decimal = if decimal < 0 && to != 10 && decimal >= i32::MIN as i64 { decimal as u32 as i64 } else { decimal };
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
        Value::Array(_) => 8204, // vbArray + vbVariant, as in VB
        Value::Object(_) => 9,   // vbObject
    })
}

pub fn rp_isnumeric(val: &Value) -> Value {
    match val {
        Value::Integer(_) | Value::Double(_) => v_int(-1),
        Value::String(s) => v_int(if s.parse::<f64>().is_ok() { -1 } else { 0 }),
        _ => v_int(0),
    }
}

pub fn rp_doevents() {
    // no-op in console mode
}

pub fn rp_default_for_type(type_name: &str) -> Value {
    match type_name.to_uppercase().as_str() {
        "INTEGER" | "BYTE" | "WORD" | "DWORD" | "LONG" | "INT64" => v_int(0),
        "DOUBLE" | "SINGLE" | "CURRENCY" => v_dbl(0.0),
        "STRING" => v_str(""),
        _ => v_null(),
    }
}

/// `INSERT$(insert, source, index)`: `insert` placed before the
/// 1-based `index` of `source` (`INSERT$("hi", "Hello", 3)` = "Hehillo").
pub fn rp_insert(insert: &Value, source: &Value, index: &Value) -> Value {
    crate::strings::insert(source, index, insert)
}

/// DELETE$ — delete count characters starting at 1-based position
pub fn rp_delete(s: &Value, start: &Value, count: &Value) -> Value {
    crate::strings::delete(s, start, count)
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
/// REPLACESUBSTR$ — replace every occurrence of a substring
pub fn rp_replacesubstr(s: &Value, find: &Value, replacement: &Value) -> Value {
    crate::strings::replace_all(s, find, replacement)
}

pub fn rp_rinstr(s: &Value, substr: &Value) -> Value {
    crate::strings::rinstr(s, substr)
}

/// `FORMAT$(fmt, arg0, arg1, …)` (Pascal `Format`, see `crate::format`).
pub fn rp_format(fmt: &Value, args: &[Value]) -> Value {
    Value::String(crate::format::format(&fmt.to_string_val(), args))
}

/// `STRF$(value, format, precision, digits)` (`FloatToStrF`, see
/// `crate::format::strf`); a missing format is ffGeneral, precision 15.
pub fn rp_strf(v: &Value, format: &Value, precision: &Value, digits: &Value) -> Value {
    let precision = if matches!(precision, Value::Null) { 15 } else { precision.to_i64() };
    Value::String(crate::format::strf(v.to_f64(), format.to_i64(), precision, digits.to_i64()))
}

// ---------------------------------------------------------------------------
// RND / RANDOMIZE / RANDINT: one generator (SplitMix64) everywhere, so
// `RANDOMIZE 42` gives the same sequence in native builds, the interpreter
// and the browser.
// ---------------------------------------------------------------------------

thread_local! {
    /// Generator state; `None` until the first draw or RANDOMIZE.
    static RNG: std::cell::Cell<Option<u64>> = const { std::cell::Cell::new(None) };
    /// Where an unseeded generator (and a bare RANDOMIZE) gets its seed.
    static ENTROPY: std::cell::Cell<fn() -> u64> = const { std::cell::Cell::new(default_entropy) };
}

fn default_entropy() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0x9E37_79B9, |d| d.as_nanos() as u64)
    }
    #[cfg(target_arch = "wasm32")]
    {
        0x9E37_79B9_7F4A_7C15
    }
}

/// The runtime's source of a fresh seed (the web runtime uses `Math.random`,
/// wasm having no clock).
pub fn set_entropy(f: fn() -> u64) {
    ENTROPY.with(|e| e.set(f));
}

/// The next number in [0, 1) from the shared generator (RND and the
/// data-science components draw from it, so RANDOMIZE reaches both).
pub fn random_unit() -> f64 {
    RNG.with(|r| {
        let mut s = r.get().unwrap_or_else(|| ENTROPY.with(|e| e.get())());
        s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
        r.set(Some(s));
        let mut z = s;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64
    })
}

/// `RND` → a number in [0, 1); `RND(n)` → an integer 0 … n-1.
pub fn rp_rnd(upper: &Value) -> Value {
    let u = upper.to_i64();
    if u > 0 {
        v_int(((random_unit() * u as f64) as i64).min(u - 1))
    } else {
        v_dbl(random_unit())
    }
}

/// `RANDOMIZE seed` restarts the sequence for that seed; a bare
/// `RANDOMIZE` takes a fresh seed (RapidQ: `RANDOMIZE TIMER`).
pub fn rp_randomize(seed: &Value) {
    let s = match seed {
        Value::Null => ENTROPY.with(|e| e.get())(),
        v => v.to_f64().to_bits(),
    };
    RNG.with(|r| r.set(Some(s)));
}

/// `RANDINT(low, high, n)`: n integers in low … high, comma separated.
pub fn rp_randint(low: &Value, high: &Value, size: &Value) -> Value {
    let (lo, hi) = (low.to_i64(), high.to_i64());
    let span = (hi - lo + 1).max(1);
    let vals: Vec<String> = (0..size.to_i64().max(0)).map(|_| (lo + ((random_unit() * span as f64) as i64).min(span - 1)).to_string()).collect();
    v_str(&vals.join(","))
}

/// An integer in 0 … n-1 from the shared generator (0 when n is 0).
pub fn random_index(n: usize) -> usize {
    ((random_unit() * n as f64) as usize).min(n.saturating_sub(1))
}
