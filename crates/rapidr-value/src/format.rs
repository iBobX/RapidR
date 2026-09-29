//! `FORMAT$` and `STRF$` as the RapidQ manual describes them: RapidQ passes
//! them to Delphi's `Format` and `FloatToStrF`, so these follow those rules
//! (US settings: `.` decimal point, `,` thousands, `$` currency). One
//! implementation for native builds, the interpreter and the web.

use crate::Value;

/// `FORMAT$(fmt, arg0, arg1, …)`: plain text is copied; each
/// `%[index:][-][width][.prec]type` takes the next argument (or argument
/// `index`, counted from 0). Types: `d` `u` (integer; `.prec` = at least
/// that many digits, zero-padded), `e` `f` `g` `n` `m` (floating point),
/// `s` (string; `.prec` = at most that many characters), `x` (hex; `.prec`
/// as for `d`). `*` as width or precision takes it from the arguments;
/// `%%` is a `%`. A specifier with no argument left is kept as written.
pub fn format(fmt: &str, args: &[Value]) -> String {
    let chars: Vec<char> = fmt.chars().collect();
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
        // A number, or `*` (taken from the arguments).
        let number = |i: &mut usize, next: &mut usize| -> Option<usize> {
            if chars.get(*i) == Some(&'*') {
                *i += 1;
                let v = args.get(*next).map(|v| v.to_i64().max(0) as usize);
                *next += 1;
                return v;
            }
            let from = *i;
            while chars.get(*i).is_some_and(|c| c.is_ascii_digit()) {
                *i += 1;
            }
            (*i > from).then(|| chars[from..*i].iter().collect::<String>().parse().unwrap_or(0))
        };
        // `%1:d`: an index is a number followed by `:`.
        let save = i;
        if chars.get(i).is_some_and(|c| c.is_ascii_digit()) {
            match number(&mut i, &mut next) {
                Some(n) if chars.get(i) == Some(&':') => {
                    next = n;
                    i += 1;
                }
                _ => i = save,
            }
        }
        let left = chars.get(i) == Some(&'-');
        if left {
            i += 1;
        }
        let width = number(&mut i, &mut next);
        let prec = if chars.get(i) == Some(&'.') {
            i += 1;
            Some(number(&mut i, &mut next).unwrap_or(0))
        } else {
            None
        };
        let Some(&kind) = chars.get(i) else {
            out.extend(&chars[start..]);
            break;
        };
        i += 1;
        let Some(arg) = args.get(next) else {
            out.extend(&chars[start..i]);
            continue;
        };
        next += 1;
        let text = match kind.to_ascii_lowercase() {
            'd' => min_digits(&arg.to_f64().trunc().to_string().replace("-0", "0"), prec),
            'u' => min_digits(&(arg.to_i64() as u32).to_string(), prec),
            'x' => min_digits(&format!("{:X}", arg.to_i64() as u32), prec),
            'e' => exponent(arg.to_f64(), prec.unwrap_or(15), 3),
            'f' => fixed(arg.to_f64(), prec.unwrap_or(2)),
            'g' => general(arg.to_f64(), prec.unwrap_or(15), 0),
            'n' => thousands(&fixed(arg.to_f64(), prec.unwrap_or(2))),
            'm' => money(arg.to_f64(), prec.unwrap_or(2)),
            's' => {
                let s = arg.to_string_val();
                match prec {
                    Some(p) => s.chars().take(p).collect(),
                    None => s,
                }
            }
            _ => {
                out.extend(&chars[start..i]);
                continue;
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
    out
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

    fn trim(&mut self) {
        while self.digits.last() == Some(&0) {
            self.digits.pop();
        }
        if self.digits.is_empty() {
            self.exp = 0;
            self.neg = false;
        }
    }

    /// Keeps `n` digits, rounding half away from zero (as Delphi does).
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
    let mut d = Decimal::of(v);
    d.round_to(d.exp as i64 + decimals as i64);
    let mut s = String::from(d.sign());
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
    let digits = digits.max(1);
    let mut d = Decimal::of(v);
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
    if rounded.abs() < 0.00001 || d.exp > precision as i32 {
        // ffGeneral writes no `+` in the exponent (`1E20`, `1E-6`).
        let s = exponent(rounded, precision, exp_digits);
        let (mantissa, exp) = s.split_once('E').unwrap_or((&s, ""));
        return format!("{}E{}", trim_zeros(mantissa), exp.trim_start_matches('+'));
    }
    let decimals = (precision as i64 - d.exp as i64).max(0) as usize;
    trim_zeros(&fixed(rounded, decimals))
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

/// `$1,234.50`, negative `($1,234.50)` (US currency settings).
fn money(v: f64, decimals: usize) -> String {
    let s = format!("${}", thousands(&fixed(v.abs(), decimals)));
    if v < 0.0 && fixed(v, decimals).starts_with('-') {
        format!("({s})")
    } else {
        s
    }
}

/// A number as `STR$` and `PRINT` show it: Delphi's `FloatToStr` (15
/// significant digits, the shorter of fixed and scientific), so
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
        for (v, s) in [(0.1 + 0.2, "0.3"), (1.0 / 3.0, "0.333333333333333"), (1024.0, "1024"), (-2.5, "-2.5"), (1e20, "1E20"), (1e15, "1E15"), (123456789012345.0, "123456789012345"), (0.00001, "0.00001"), (0.000001, "1E-6"), (-0.0, "0"), (f64::NAN, "NAN"), (2.0 / 3.0, "0.666666666666667")] {
            assert_eq!(float_to_str(v), s, "{v}");
        }
    }
}
