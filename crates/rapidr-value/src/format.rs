//! `FORMAT$` and `STRF$` as the RapidQ manual describes them: RapidQ passes
//! them to its own `Format` and `FloatToStrF`, so these follow those rules
//! (US settings: `.` decimal point, `,` thousands, `$` currency). One
//! implementation for native builds, the interpreter and the web.

use crate::Value;

/// `FORMAT$(fmt, arg0, arg1, …)`: plain text is copied; each
/// `%[index:][-][width][.prec]type` takes the next argument (or argument
/// `index`, counted from 0). Types: `d` (integer; `.prec` = at least that
/// many digits, zero-padded), `e` `f` `g` `n` `m` (floating point), `s`
/// (string; `.prec` = at most that many characters), `x` (hex; `.prec` as for
/// `d`). `*` as the width takes it from the arguments; `%%` is a `%`.
///
/// What RC.EXE does with it (docs/rapidq-ground-truth.md, `rapidq_format`),
/// all kept here:
/// - the arguments' types count: `%s` shows a string and nothing for a
///   number, `%d` `%x` `%e` `%f` `%g` `%n` `%m` take a number and read a
///   string as 0;
/// - `%d` and `%x` take a number as a 32-bit integer (cut toward zero;
///   beyond 32 bits, an infinity or NaN: -2147483648, `80000000`);
/// - the digits of `%e` `%f` `%g` `%n` `%m` are RapidQ's own 18
///   ([`rapidq_digits`]), so `%.2f` of 2.675 is `2.67` and `%.9f` of
///   1234567890.1 `1234567890.099999840`; a negative number rounded to zero
///   keeps its sign (`-0.0`); an infinity and NaN are `INF` and `NAN`; a
///   whole part longer than 18 digits turns to `1E300`'s scientific form;
/// - `.prec` is ignored above 15 for `d` `x` and above 18 for the floats
///   (the default then: 2 decimals, 15 digits), and is at least 2 for `e`
///   `g`;
/// - a `%` ending the text, or ending in a `.`, `-`, `:` or an index is
///   dropped; a specifier RapidQ's `Format` refuses (`%+d`, `% d`, `%#x`,
///   `%ld`, `%i`, `%c`, `%.*f` …) stops the program: `Format '%+' invalid
///   or incompatible with argument.` (RapidQ's EConvertError; for some
///   letters it crashes with an access violation instead).
/// A specifier with no argument left is kept as written (RapidQ shows
/// whatever its stack holds there).
pub fn format(fmt: &str, args: &[Value]) -> String {
    try_format(fmt, args).unwrap_or_else(|e| crate::runtime_error(&e))
}

/// [`format`], the refused specifier as an error (the bytecode hosts').
pub fn try_format(fmt: &str, args: &[Value]) -> Result<String, String> {
    let chars: Vec<char> = fmt.chars().collect();
    let refused = |start: usize, end: usize| -> String {
        let end = end.min(chars.len().saturating_sub(1));
        format!("Format '{}' invalid or incompatible with argument.", chars[start..=end].iter().collect::<String>())
    };
    let mut out = String::new();
    let mut next = 0usize;
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '%' {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        if chars.get(i + 1) == Some(&'%') {
            out.push('%');
            i += 2;
            continue;
        }
        let start = i;
        i += 1;
        let digits = |i: &mut usize| -> Option<usize> {
            let from = *i;
            while chars.get(*i).is_some_and(|c| c.is_ascii_digit()) {
                *i += 1;
            }
            (*i > from).then(|| chars[from..*i].iter().collect::<String>().parse().unwrap_or(usize::MAX))
        };
        // `%1:d`: an index is a number followed by `:`.
        let save = i;
        if let Some(n) = digits(&mut i) {
            if chars.get(i) == Some(&':') {
                next = n;
                i += 1;
            } else {
                i = save;
            }
        }
        let left = chars.get(i) == Some(&'-');
        if left {
            i += 1;
        }
        // The width: a number, or `*` (the next argument).
        let mut width = None;
        if chars.get(i) == Some(&'*') {
            i += 1;
            width = args.get(next).map(|v| number_of(v).trunc().max(0.0) as usize);
            next += 1;
        } else if let Some(w) = digits(&mut i) {
            width = Some(w);
        }
        let mut prec: Option<usize> = None;
        let mut star_prec = false;
        if chars.get(i) == Some(&'.') {
            i += 1;
            if chars.get(i) == Some(&'*') {
                i += 1;
                star_prec = true;
            } else {
                prec = Some(digits(&mut i).unwrap_or(0));
            }
        }
        let Some(&kind) = chars.get(i) else {
            // (the text ends inside a specifier: what is left is dropped)
            break;
        };
        // (`%-`, `%.` and the like: nothing follows the flag)
        let type_at = i;
        i += 1;
        if !matches!(kind.to_ascii_lowercase(), 'd' | 'x' | 'e' | 'f' | 'g' | 'n' | 'm' | 's') {
            return Err(refused(start, type_at));
        }
        if star_prec {
            return Err(refused(start, type_at));
        }
        let Some(arg) = args.get(next) else {
            out.extend(&chars[start..i]);
            continue;
        };
        next += 1;
        let float_prec = prec.filter(|p| *p <= 18);
        let int_prec = prec.filter(|p| *p <= 15);
        let text = match kind.to_ascii_lowercase() {
            's' => {
                let s = match arg {
                    Value::String(s) => s.as_str().to_string(),
                    _ => String::new(),
                };
                match prec {
                    Some(p) => s.chars().take(p).collect(),
                    None => s,
                }
            }
            'd' => min_digits(&int32_trunc(number_of(arg)).to_string(), int_prec),
            'x' => min_digits(&format!("{:X}", int32_trunc(number_of(arg)) as i32 as u32), int_prec),
            kind => {
                let v = number_of(arg);
                if v.is_nan() {
                    "NAN".to_string()
                } else if v.is_infinite() {
                    // (RapidQ writes no sign for an infinity, -1 / 0 too)
                    "INF".to_string()
                } else {
                    match kind {
                        'e' => exponent_from(Decimal::rapidq(v), float_prec.map_or(15, |p| p.max(2)), 3),
                        'f' => fixed_rq(v, float_prec.unwrap_or(2)),
                        'g' => general_rq(v, float_prec.map_or(15, |p| p.max(2)), 0),
                        'n' => {
                            let s = fixed_rq(v, float_prec.unwrap_or(2));
                            if s.contains('E') { s } else { thousands(&s) }
                        }
                        _ => money(v, float_prec.unwrap_or(2)),
                    }
                }
            }
        };
        let pad = width.unwrap_or(0).saturating_sub(text.chars().count());
        if left {
            out.push_str(&text);
            out.extend(std::iter::repeat_n(' ', pad));
        } else {
            out.extend(std::iter::repeat_n(' ', pad));
            out.push_str(&text);
        }
    }
    Ok(out)
}

/// What a number-taking specifier reads from an argument: a number as it
/// is, anything else (a string, nothing) 0 — RapidQ's `Format` is passed
/// the argument's own bits.
fn number_of(v: &Value) -> f64 {
    match v {
        Value::Integer(_) | Value::Double(_) | Value::Boolean(_) => v.to_f64(),
        _ => 0.0,
    }
}

/// A number as `%d` and `%x` make it a 32-bit integer: cut toward zero,
/// -2147483648 beyond 32 bits (an infinity and NaN too).
fn int32_trunc(v: f64) -> i64 {
    let t = v.trunc();
    if t.is_nan() || !(-2147483648.0..=2147483647.0).contains(&t) {
        i32::MIN as i64
    } else {
        t as i64
    }
}

/// `-42` with at least `prec` digits: `-00042`.
fn min_digits(s: &str, prec: Option<usize>) -> String {
    let (sign, digits) = s.strip_prefix('-').map_or(("", s), |d| ("-", d));
    let width = prec.unwrap_or(0);
    format!("{sign}{digits:0>width$}")
}

/// `v` as decimal digits: value = 0.d1d2d3… × 10^exp (the shortest digits
/// that read back as `v`, as Rust prints them).
struct Decimal {
    neg: bool,
    digits: Vec<u8>,
    exp: i32,
}

impl Decimal {
    fn of(v: f64) -> Decimal {
        let s = format!("{:e}", v.abs());
        let (mantissa, exp) = s.split_once('e').unwrap_or((&s, "0"));
        let digits: Vec<u8> = mantissa.bytes().filter(u8::is_ascii_digit).map(|b| b - b'0').collect();
        let exp = exp.parse::<i32>().unwrap_or(0) + 1;
        let mut d = Decimal { neg: v < 0.0, digits, exp };
        d.trim();
        d
    }

    /// `v` as RapidQ's `FloatToDecimal` gives it ([`rapidq_digits`]: 18
    /// digits), the sign kept: what `FORMAT$` rounds.
    fn rapidq(v: f64) -> Decimal {
        if v == 0.0 || !v.is_finite() {
            return Decimal::of(v);
        }
        let mut d = rapidq_digits(v.abs());
        d.neg = v < 0.0 && !d.digits.is_empty();
        d
    }

    fn trim(&mut self) {
        while self.digits.last() == Some(&0) {
            self.digits.pop();
        }
        if self.digits.is_empty() {
            self.exp = 0;
            self.neg = false;
        }
    }

    /// Keeps `n` digits, rounding half away from zero (as RapidQ's runtime does).
    fn round_to(&mut self, n: i64) {
        if n < 0 {
            self.digits.clear();
            self.trim();
            return;
        }
        let n = n as usize;
        if self.digits.len() <= n {
            return;
        }
        let up = self.digits[n] >= 5;
        self.digits.truncate(n);
        if up {
            let mut i = n;
            loop {
                if i == 0 {
                    self.digits.insert(0, 1);
                    self.exp += 1;
                    break;
                }
                i -= 1;
                if self.digits[i] == 9 {
                    self.digits[i] = 0;
                } else {
                    self.digits[i] += 1;
                    break;
                }
            }
        }
        self.trim();
    }

    fn digit(&self, i: i64) -> char {
        if i < 0 {
            '0'
        } else {
            (b'0' + self.digits.get(i as usize).copied().unwrap_or(0)) as char
        }
    }

    fn sign(&self) -> &'static str {
        if self.neg { "-" } else { "" }
    }
}

/// `-ddd.ddd` with `decimals` digits after the point.
fn fixed(v: f64, decimals: usize) -> String {
    fixed_from(Decimal::of(v), decimals, false)
}

/// [`fixed`] on RapidQ's own digits, as `FORMAT$`'s `%f` shows it: a
/// negative number that rounds to zero keeps its minus (`-0.0`), and a
/// whole part of more than 18 digits is shown in scientific form
/// (`9.99999999999999872E299`).
fn fixed_rq(v: f64, decimals: usize) -> String {
    let d = Decimal::rapidq(v);
    if d.exp > 18 {
        return general_rq(v, 18, 0);
    }
    fixed_from(d, decimals, true)
}

fn fixed_from(mut d: Decimal, decimals: usize, keep_sign: bool) -> String {
    // (a negative number rounded to zero keeps its minus while the rounding
    // reaches into its digits: `%.1f` of -0.04 is `-0.0`, `%.0f` of -0.04 `0`)
    let reach = d.exp as i64 + decimals as i64;
    let neg = d.neg && reach >= 0;
    d.round_to(reach);
    let mut s = String::from(if keep_sign && neg { "-" } else { d.sign() });
    if d.exp <= 0 {
        s.push('0');
    } else {
        s.extend((0..d.exp as i64).map(|i| d.digit(i)));
    }
    if decimals > 0 {
        s.push('.');
        s.extend((0..decimals as i64).map(|i| d.digit(d.exp as i64 + i)));
    }
    s
}

/// `-d.dddE+ddd`: `digits` significant digits, an exponent of at least
/// `exp_digits` digits.
fn exponent(v: f64, digits: usize, exp_digits: usize) -> String {
    exponent_from(Decimal::of(v), digits, exp_digits)
}

fn exponent_from(mut d: Decimal, digits: usize, exp_digits: usize) -> String {
    let digits = digits.max(1);
    d.round_to(digits as i64);
    let e = if d.digits.is_empty() { 0 } else { d.exp - 1 };
    let mut s = format!("{}{}", d.sign(), d.digit(0));
    if digits > 1 {
        s.push('.');
        s.extend((1..digits as i64).map(|i| d.digit(i)));
    }
    let (esign, e) = if e < 0 { ('-', -e) } else { ('+', e) };
    format!("{s}E{esign}{e:0>exp_digits$}")
}

/// The shortest of fixed and scientific with `precision` significant
/// digits, trailing zeros removed.
fn general(v: f64, precision: usize, exp_digits: usize) -> String {
    let precision = precision.clamp(1, 18);
    let mut d = Decimal::of(v);
    d.round_to(precision as i64);
    if d.digits.is_empty() {
        return "0".into();
    }
    let rounded: f64 = format!("{}0.{}e{}", d.sign(), d.digits.iter().map(|b| (b'0' + b) as char).collect::<String>(), d.exp).parse().unwrap_or(v);
    // (RapidQ's FloatToText: scientific when the exponent is above the
    // precision or below -3 — 0.0001 stays, 0.00001 is `1E-5`, as RC.EXE's
    // STR$ shows)
    if d.exp < -3 || d.exp > precision as i32 {
        // ffGeneral writes no `+` in the exponent (`1E20`, `1E-6`).
        let s = exponent(rounded, precision, exp_digits);
        let (mantissa, exp) = s.split_once('E').unwrap_or((&s, ""));
        return format!("{}E{}", trim_zeros(mantissa), exp.trim_start_matches('+'));
    }
    let decimals = (precision as i64 - d.exp as i64).max(0) as usize;
    trim_zeros(&fixed(rounded, decimals))
}

/// [`general`] on RapidQ's own digits (`FORMAT$`'s `%g`): `%.18g` of 2.675
/// is `2.67499999999999968`, `%.3g` of it `2.67`.
fn general_rq(v: f64, precision: usize, exp_digits: usize) -> String {
    let precision = precision.clamp(1, 18);
    let mut d = Decimal::rapidq(v);
    d.round_to(precision as i64);
    if d.digits.is_empty() {
        return "0".into();
    }
    if d.exp < -3 || d.exp > precision as i32 {
        let s = exponent_from(d, precision, exp_digits);
        let (mantissa, exp) = s.split_once('E').unwrap_or((&s, ""));
        return format!("{}E{}", trim_zeros(mantissa), exp.trim_start_matches('+'));
    }
    let decimals = (precision as i64 - d.exp as i64).max(0) as usize;
    trim_zeros(&fixed_from(d, decimals, false))
}

fn trim_zeros(s: &str) -> String {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s.to_string()
    }
}

/// `1234567.5` → `1,234,567.5`.
fn thousands(s: &str) -> String {
    let (sign, rest) = s.strip_prefix('-').map_or(("", s), |r| ("-", r));
    let (int, frac) = rest.split_once('.').map_or((rest, None), |(a, b)| (a, Some(b)));
    let mut grouped = String::new();
    for (n, c) in int.chars().enumerate() {
        if n > 0 && (int.len() - n) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    match frac {
        Some(f) => format!("{sign}{grouped}.{f}"),
        None => format!("{sign}{grouped}"),
    }
}

/// `$1,234.50`, negative `($1,234.50)` (US currency settings; RC.EXE: a
/// negative number is in parentheses as `%f` shows it with a minus, so
/// `($0.00)` for -0.004 and `$0.00` for -0.0000001).
fn money(v: f64, decimals: usize) -> String {
    let f = fixed_rq(v, decimals);
    if f.contains('E') {
        return f;
    }
    match f.strip_prefix('-') {
        Some(rest) => format!("(${})", thousands(rest)),
        None => format!("${}", thousands(&f)),
    }
}

/// A whole number as RapidQ makes it a 32-bit integer (the x87's `FISTP`):
/// rounded to the nearest (half to even); out of the 32-bit range, NaN or
/// infinite, the "integer indefinite" -2147483648 (RC.EXE:
/// `PRINT 2147483648` shows -2147483648, docs/rapidq-ground-truth.md).
#[inline]
pub fn int32(v: f64) -> i64 {
    let r = v.round_ties_even();
    if r.is_nan() || !(-2147483648.0..=2147483647.0).contains(&r) {
        i32::MIN as i64
    } else {
        r as i64
    }
}

/// An integer RapidQ holds (32 bits): out of range, -2147483648.
#[inline]
pub fn int32_of(n: i64) -> i64 {
    if n < i32::MIN as i64 || n > i32::MAX as i64 {
        i32::MIN as i64
    } else {
        n
    }
}

/// A number as RapidQ's `PRINT` shows it (RC.EXE is the evidence,
/// docs/rapidq-ground-truth.md): a whole number as a 32-bit integer
/// ([`int32`]: `PRINT 1E10` shows -2147483648, so do NaN and the
/// infinities); any other with 9 decimals (`3.500000000`, `0.333333333`,
/// `-0.000000000` for a tiny negative one). The digits are RapidQ's
/// `FloatToDecimal` ones as its runtime gets them ([`rapidq_digits`]):
/// 18 of them, from double-precision arithmetic, so `123456789.1` shows
/// `123456789.100000000` and `1234567890.1` `1234567890.099999840`, as
/// RC.EXE's programs do (a few values ≥ 1E7 still differ in their last
/// digits: 123456789.125 shows `123456789.125000013` there, `…124999984`
/// here).
pub fn print_double(v: f64) -> String {
    if v.is_nan() || v.is_infinite() || v == v.trunc() {
        return int32(v).to_string();
    }
    let mut d = rapidq_digits(v.abs());
    d.round_to(d.exp as i64 + 9);
    let mut out = String::from(if v < 0.0 { "-" } else { "" });
    if d.exp <= 0 {
        out.push('0');
    } else {
        out.extend((0..d.exp as i64).map(|i| d.digit(i)));
    }
    out.push('.');
    out.extend((0..9).map(|i| d.digit(d.exp as i64 + i)));
    out
}

/// The 18 digits RapidQ's `FloatToDecimal` takes from `x` (> 0), with the
/// FPU in double precision as RapidQ's runtime has it: the decimal exponent
/// estimated from the binary one (`(e2 * 19728) >> 16 + 1`), `x` scaled by
/// 10^(18 - E) (the product rounded to a double), rounded to an integer, a
/// tenth of it when it came out 19 digits long, then the 18 BCD digits
/// (`FBSTP`). Reproduces RC.EXE-built programs' digits beyond the 16th
/// (`1234567890.1` prints `…099999840`, `1.0000000015` `1.000000001`) —
/// most of them.
fn rapidq_digits(x: f64) -> Decimal {
    let e2 = ((x.to_bits() >> 52) & 0x7ff) as i64 - 1023;
    let mut e = ((e2 * 19728) >> 16) + 1;
    let k = 18 - e;
    // (10^k is exact up to 10^22; beyond, only digits far past the 9
    // decimals shown change)
    let scale = |y: f64, k: i64| if k >= 0 { y * 10f64.powi(k.min(22) as i32) * 10f64.powi((k - 22).max(0) as i32) } else { y / 10f64.powi((-k) as i32) };
    let mut y = scale(x, k).round_ties_even();
    if y >= 1e18 {
        y = (y / 10.0).round_ties_even();
        e += 1;
    }
    let n = if y.is_finite() && y >= 0.0 { y as u64 } else { 0 };
    let digits: Vec<u8> = format!("{n:018}").bytes().map(|b| b - b'0').collect();
    let mut d = Decimal { neg: false, digits, exp: e as i32 };
    // (a scaled value below 10^17 — an underestimated exponent — has
    // leading zeros: the number's own exponent is lower)
    while d.digits.first() == Some(&0) && !d.digits.is_empty() {
        d.digits.remove(0);
        d.exp -= 1;
    }
    d.trim();
    d
}

/// A value as `PRINT` shows it: numbers as [`print_double`] (an integer
/// beyond 32 bits as RapidQ's -2147483648), anything else as its text.
pub fn print_text(v: &Value) -> String {
    match v {
        Value::Integer(n) => int32_of(*n).to_string(),
        Value::Double(d) => print_double(*d),
        _ => v.to_string_val(),
    }
}

/// `STR$` of a number as RapidQ's RC.EXE shows it: its
/// `FloatToStrF(v, ffGeneral, 9, 0)` — 9 significant digits, the shorter
/// of fixed and scientific (`0.333333333`, `2.5`, `1.23456789E9` — even
/// for an INTEGER, `1E-5`, `INF`), no leading space.
pub fn str_number(v: f64) -> String {
    if v.is_nan() {
        return "NAN".into();
    }
    // (RC.EXE: STR$ of -1 / 0 is INF too, the sign dropped — as FORMAT$'s)
    if v.is_infinite() {
        return "INF".into();
    }
    general(v, 9, 0)
}

/// A number as RapidR's own text conversions show it: RapidQ's `FloatToStr`
/// (15 significant digits, the shorter of fixed and scientific), so
/// `0.1 + 0.2` is `0.3` and `1E20` stays short.
pub fn float_to_str(v: f64) -> String {
    if v.is_nan() {
        return "NAN".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "INF".into() } else { "-INF".into() };
    }
    // Whole numbers (the common case) as integers.
    if v == v.trunc() && v.abs() < 1e15 {
        return (v as i64).to_string();
    }
    general(v, 15, 0)
}

/// `STRF$(value, format, precision, digits)`: 0 = ffGeneral (shortest,
/// `digits` = least exponent digits), 1 = ffExponent, 2 = ffFixed
/// (`digits` after the point), 3 = ffNumber (ffFixed with thousands
/// separators). `precision` is the number of significant digits; ffFixed
/// and ffNumber switch to scientific when the integer part has more digits.
pub fn strf(v: f64, format: i64, precision: i64, digits: i64) -> String {
    let precision = precision.clamp(1, 18) as usize;
    match format {
        1 => exponent(v, precision, digits.clamp(0, 4) as usize),
        2 | 3 => {
            let mut d = Decimal::of(v);
            if d.exp > precision as i32 {
                return exponent(v, precision, 3);
            }
            // Rounded to `precision` significant digits first, then shown
            // with `digits` decimals.
            d.round_to(precision as i64);
            let rounded: f64 = format!("{}0.{}e{}", d.sign(), d.digits.iter().map(|b| (b'0' + b) as char).collect::<String>(), d.exp).parse().unwrap_or(0.0);
            let s = fixed(rounded, digits.clamp(0, 18) as usize);
            if format == 3 { thousands(&s) } else { s }
        }
        _ => general(v, precision, digits.clamp(0, 4) as usize),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{v_dbl, v_int, v_str};

    #[test]
    fn manual_examples() {
        assert_eq!(format("Location: %s %4d %-10.4g|", &[v_str("Any"), v_int(1234), v_dbl(55.39)]), "Location: Any 1234 55.39     |");
        assert_eq!(format("%d %d %0:d %d", &[v_int(10), v_int(20)]), "10 20 10 20");
        assert_eq!(format("%.5d|%5d|%-5d|%x|%.4x", &[v_int(42), v_int(42), v_int(42), v_int(255), v_int(255)]), "00042|   42|42   |FF|00FF");
        assert_eq!(format("%f %.1f %n %m %m", &[v_dbl(3.14159), v_dbl(2.25), v_dbl(1234567.891), v_dbl(1234.5), v_dbl(-2.0)]), "3.14 2.3 1,234,567.89 $1,234.50 ($2.00)");
        assert_eq!(format("%e|%.3e", &[v_dbl(1234.5), v_dbl(-0.00012)]), "1.23450000000000E+003|-1.20E-004");
        assert_eq!(format("%g|%g|%g", &[v_dbl(0.5), v_dbl(1e20), v_dbl(0.000001)]), "0.5|1E20|1E-6");
        assert_eq!(format("%.3s|%*d|100%%|%d", &[v_str("abcdef"), v_int(4), v_int(7)]), "abc|   7|100%|%d");
    }

    #[test]
    fn strf_formats() {
        assert_eq!(strf(99.9934, 0, 4, 4), "99.99");
        assert_eq!(strf(12345678.0, 3, 8, 0), "12,345,678");
        assert_eq!(strf(3.14159, 2, 6, 2), "3.14");
        assert_eq!(strf(3.14159, 2, 2, 4), "3.1000");
        assert_eq!(strf(1234.5, 1, 4, 2), "1.235E+03");
        assert_eq!(strf(123456.0, 2, 3, 2), "1.23E+005");
        assert_eq!(strf(2.5, 2, 15, 0), "3");
        assert_eq!(strf(-0.004, 2, 15, 2), "0.00");
        assert_eq!(strf(0.1 + 0.2, 0, 15, 0), "0.3");
        assert_eq!(strf(1234567.0, 0, 3, 2), "1.23E06");
    }

    #[test]
    fn numbers_as_text() {
        for (v, s) in [(0.1 + 0.2, "0.3"), (1.0 / 3.0, "0.333333333333333"), (1024.0, "1024"), (-2.5, "-2.5"), (1e20, "1E20"), (1e15, "1E15"), (123456789012345.0, "123456789012345"), (0.0001, "0.0001"), (0.00001, "1E-5"), (0.000001, "1E-6"), (-0.0, "0"), (f64::NAN, "NAN"), (2.0 / 3.0, "0.666666666666667")] {
            assert_eq!(float_to_str(v), s, "{v}");
        }
    }

    // What RapidQ's RC.EXE-built programs print (docs/rapidq-ground-truth.md).
    #[test]
    fn print_as_rapidq() {
        for (v, s) in [
            (1.0 / 3.0, "0.333333333"),
            (2.0 / 3.0, "0.666666667"),
            (2.5, "2.500000000"),
            (0.1, "0.100000000"),
            (-0.5, "-0.500000000"),
            (3.0, "3"),
            (1024.0, "1024"),
            (1.0 / 7.0 * 1e6, "142857.142857143"),
            (0.1f32 as f64, "0.100000001"),
            (123456.7890625, "123456.789062500"),
            (123456789.1, "123456789.100000000"),
            (1e9 + 0.5, "1000000000.500000000"),
            (2147483647.5, "2147483647.500000000"),
            // (digits beyond the 16th as RapidQ's runtime makes them)
            (1234567890.1, "1234567890.099999840"),
            (1234567890123.1, "1234567890123.100160000"),
            (99999999999.5, "99999999999.500006400"),
            (1.0000000015, "1.000000001"),
            (9876543.987654321, "9876543.987654321"),
            (9999999.99999999, "9999999.999999991"),
            (1234567.123456789, "1234567.123456789"),
            (123456.1234567895, "123456.123456790"),
            (99999.9999999999, "100000.000000000"),
            (2.675, "2.675000000"),
            (-9.84147, "-9.841470000"),
            (0.000001, "0.000001000"),
            (1e-20, "0.000000000"),
            (-1e-20, "-0.000000000"),
            (0.0000000005, "0.000000001"),
            (0.9999999999, "1.000000000"),
            (-0.9999999996, "-1.000000000"),
            (2147483647.0, "2147483647"),
            (2147483648.0, "-2147483648"),
            (1e20, "-2147483648"),
            (-3e9, "-2147483648"),
            (f64::INFINITY, "-2147483648"),
            (f64::NAN, "-2147483648"),
            (-0.0, "0"),
        ] {
            assert_eq!(print_double(v), s, "{v}");
        }
        assert_eq!(print_text(&Value::Integer(4294967296)), "-2147483648");
        assert_eq!(print_text(&Value::Integer(-7)), "-7");
    }

    #[test]
    fn str_as_rapidq() {
        for (v, s) in [
            (1.0 / 3.0, "0.333333333"),
            (5.0, "5"),
            (-5.0, "-5"),
            (2.5, "2.5"),
            (1e20, "1E20"),
            (0.1, "0.1"),
            (0.1f32 as f64, "0.100000001"),
            (25.0, "25"),
            (123456.789, "123456.789"),
            (1.0 / 7.0 * 1e6, "142857.143"),
            (12345678.5, "12345678.5"),
            (1234567890.5, "1.23456789E9"),
            (1234567890.0, "1.23456789E9"),
            (0.00001, "1E-5"),
            (0.000001234, "1.234E-6"),
            (1e-20, "1E-20"),
            (-1e20, "-1E20"),
            (3e9, "3E9"),
            (2147483648.0, "2.14748365E9"),
            (1.5e9, "1.5E9"),
            (2f64.sqrt(), "1.41421356"),
            (100.0 / 3.0, "33.3333333"),
            (123456789.123, "123456789"),
            (f64::INFINITY, "INF"),
            (f64::NEG_INFINITY, "INF"),
        ] {
            assert_eq!(str_number(v), s, "{v}");
        }
    }

    // What RC.EXE's FORMAT$ gives (tests/conformance/cases/rapidq_format.bas
    // is the whole table, in RapidQ's own output).
    #[test]
    fn format_as_rapidq() {
        let f = |fmt: &str, args: &[Value]| try_format(fmt, args).unwrap();
        // the digits are RapidQ's 18: 2.675 is a hair under
        assert_eq!(f("%.2f|%.2f|%.9f", &[v_dbl(2.675), v_dbl(1.005), v_dbl(1234567890.1)]), "2.67|1.00|1234567890.099999840");
        assert_eq!(f("%.1f|%.0f|%.0f|%.2m|%.2m", &[v_dbl(-0.04), v_dbl(-0.4), v_dbl(-0.04), v_dbl(-0.004), v_dbl(-0.0000001)]), "-0.0|-0|0|($0.00)|$0.00");
        // 32-bit integers, cut toward zero; text where a number goes is 0
        assert_eq!(f("%d|%d|%x|%x|%d", &[v_dbl(3.7), v_dbl(2147483648.0), v_dbl(4294967295.0), v_int(-1), v_str("12")]), "3|-2147483648|80000000|FFFFFFFF|0");
        // a number where text goes is nothing; a precision beyond the caps is ignored
        assert_eq!(f("[%s]|%.16d|%.19f|%.0e", &[v_int(5), v_int(5), v_dbl(0.5), v_dbl(1234.5)]), "[]|5|0.50|1.2E+003");
        assert_eq!(f("%f|%e|%d|abc%", &[v_dbl(f64::INFINITY), v_dbl(f64::NAN), v_dbl(f64::INFINITY)]), "INF|NAN|-2147483648|abc");
        // a specifier Format refuses
        assert_eq!(try_format("%+d", &[v_int(1)]).unwrap_err(), "Format '%+' invalid or incompatible with argument.");
        assert_eq!(try_format("%d %ld", &[v_int(1), v_int(2)]).unwrap_err(), "Format '%l' invalid or incompatible with argument.");
        assert_eq!(try_format("%.*f", &[v_int(1), v_dbl(2.0)]).unwrap_err(), "Format '%.*f' invalid or incompatible with argument.");
    }
}
