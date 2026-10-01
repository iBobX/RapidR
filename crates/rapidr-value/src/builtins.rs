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

pub fn rp_chr(n: &Value) -> Value {
    Value::String(String::from(char::from(n.to_i64() as u8)))
}

pub fn rp_asc(s: &Value) -> Value {
    crate::strings::asc(s)
}

pub fn rp_replace(s: &Value, old: &Value, new: &Value) -> Value {
    Value::String(s.to_string_val().replace(&old.to_string_val(), &new.to_string_val()))
}

pub fn rp_str(val: &Value) -> Value {
    Value::String(val.to_string_val())
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

pub fn rp_val(s: &Value) -> Value {
    let mut s = s.to_string_val().trim().to_string();
    let decimal = DECIMAL.with(|d| d.get());
    if decimal != '.' {
        s = s.replace('.', "\u{1}").replace(decimal, ".");
    }
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

pub fn rp_cbool(val: &Value) -> Value {
    crate::cbool(val)
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
