//! BASIC string functions, shared by every runtime so both backends behave
//! identically. Positions and lengths count characters (not UTF-8 bytes) and
//! are 1-based, so `MID$("héllo", 2, 1)` is "é" rather than half a character.

use crate::basic_string::{byte_of_char_from_end, Chars};
use crate::{v_int, Value};
use std::borrow::Cow;

/// The text of `v` to find positions in: a string's own, read in place
/// through its character index (nothing copied: a MID$ on a long string
/// costs what it returns), or a number's text.
fn text(v: &Value) -> Chars<'_> {
    match v {
        Value::String(s) => Chars::Indexed(s),
        other => Chars::Plain(Cow::Owned(other.to_string_val())),
    }
}

/// `t`'s bytes `range` as a string value — `whole` itself (shared, not
/// copied) when the range is all of it.
fn slice(whole: &Value, t: &Chars<'_>, range: std::ops::Range<usize>) -> Value {
    if let Value::String(s) = whole {
        if range.start == 0 && range.end == s.len() {
            return Value::String(s.clone());
        }
    }
    Value::String(t.as_str()[range].into())
}

/// Clamp a 1-based BASIC position to a 0-based char index.
fn index0(pos: &Value) -> usize {
    (pos.to_i64() - 1).max(0) as usize
}

fn count(n: &Value) -> usize {
    n.to_i64().max(0) as usize
}

pub fn len(s: &Value) -> Value {
    v_int(text(s).char_count() as i64)
}

/// MID$(s, i, n); without n (`MID$(s, i)`), the rest of s, as in QBasic.
pub fn mid(s: &Value, start: &Value, length: &Value) -> Value {
    let t = text(s);
    let range = if matches!(length, Value::Null) {
        t.byte_of_char(index0(start))..t.as_str().len()
    } else {
        t.byte_range(index0(start), count(length))
    };
    slice(s, &t, range)
}

pub fn left(s: &Value, n: &Value) -> Value {
    let t = text(s);
    let range = t.byte_range(0, count(n));
    slice(s, &t, range)
}

pub fn right(s: &Value, n: &Value) -> Value {
    let t = text(s);
    let from = byte_of_char_from_end(t.as_str(), count(n));
    slice(s, &t, from..t.as_str().len())
}

/// INSTR([start,] haystack, needle): 1-based position, 0 if absent — as
/// RapidQ computes it (RC.EXE): an empty needle is never found; the search
/// is in the text from `start` (before the first character: all of it) and
/// the place found is counted from `start` (0 counts as 1), so
/// `INSTR(-5, "abc", "b")` is -4. A real start (RC.EXE, 2026-10-22) is
/// rounded half to even to find where the search begins, and the place is
/// counted from the start as given: `INSTR(2.5, "abcabc", "c")` searches
/// from character 2 and is 3.5, `INSTR(2.7, "abcabc", "b")` from 3 and 4.7.
pub fn instr(start: &Value, haystack: &Value, needle: &Value) -> Value {
    let hay = text(haystack);
    let nee = text(needle);
    let nee = nee.as_str();
    if nee.is_empty() {
        return v_int(0);
    }
    // (a real start that isn't whole: where the search begins, and the
    // start itself for the place found)
    let real = match start {
        Value::Double(d) if d.fract() != 0.0 && d.is_finite() => Some(*d),
        _ => None,
    };
    let start = match (start, real) {
        (Value::Null, _) => 1,
        (_, Some(d)) => crate::format::int32(d),
        (s, None) => match s.to_i64() {
            0 => 1,
            n => n,
        },
    };
    let from = (start - 1).max(0);
    // (a match of whole UTF-8 characters starts on a character, so the
    // byte search finds what a search by characters finds; the characters
    // up to it are counted as far as the search went)
    let at = hay.byte_of_char(from as usize);
    let rest = &hay.as_str()[at..];
    match rest.find(nee) {
        Some(b) => {
            let i = from + rest[..b].chars().count() as i64;
            match real {
                Some(d) => crate::v_dbl(i as f64 - from as f64 + d),
                None => v_int(i + 1 - from + (start - 1)),
            }
        }
        None => v_int(0),
    }
}

/// RINSTR([start,] haystack, needle): 1-based position of the last
/// occurrence, 0 if absent — as RapidQ computes it (RC.EXE, 2026-10-22):
/// with a `start` (a 32-bit integer, a real one rounded half to even) the
/// needle must END at or before character `start`, i.e. the search is in
/// `LEFT$(haystack, start)` (`RINSTR(4, "hehe they", "he")` is 3,
/// `RINSTR(3, …)` 1); a start of 0 or less finds nothing, one past the end
/// searches all of it. `start` is `Value::Null` for the two-argument form.
/// An empty needle is never found (RapidQ stops with an access violation
/// there; INSTR never finds one either).
pub fn rinstr(start: &Value, haystack: &Value, needle: &Value) -> Value {
    let hay = text(haystack);
    let nee = text(needle);
    let nee = nee.as_str();
    let end = match start {
        Value::Null => hay.as_str().len(),
        Value::Double(d) => hay.byte_of_char(crate::format::int32(*d).max(0) as usize),
        s => hay.byte_of_char(crate::format::int32_of(s.to_i64()).max(0) as usize),
    };
    if nee.is_empty() {
        return v_int(0);
    }
    // (the last place the needle fits whole in the text's first `start`
    // characters; a needle longer than them fits nowhere)
    match hay.as_str()[..end].rfind(nee) {
        Some(b) => v_int(hay.char_of_byte(b) as i64 + 1),
        None => v_int(0),
    }
}

/// INSERT$(s, pos, text): insert `text` before 1-based position `pos`.
pub fn insert(s: &Value, pos: &Value, text_in: &Value) -> Value {
    let t = text(s);
    let add = text(text_in);
    let at = t.byte_of_char(index0(pos));
    let (a, b) = t.as_str().split_at(at);
    let mut out = String::with_capacity(a.len() + add.as_str().len() + b.len());
    out.push_str(a);
    out.push_str(add.as_str());
    out.push_str(b);
    Value::String(out.into())
}

/// DELETE$(s, start, n): remove `n` characters from 1-based `start`.
pub fn delete(s: &Value, start: &Value, n: &Value) -> Value {
    let t = text(s);
    let gone = t.byte_range(index0(start), count(n));
    let all = t.as_str();
    let mut out = String::with_capacity(all.len() - gone.len());
    out.push_str(&all[..gone.start]);
    out.push_str(&all[gone.end..]);
    Value::String(out.into())
}

/// REPLACESUBSTR$(s, find, replacement): replace every occurrence.
pub fn replace_all(s: &Value, find: &Value, replacement: &Value) -> Value {
    let find = text(find);
    let t = text(s);
    if find.as_str().is_empty() || !t.as_str().contains(find.as_str()) {
        return slice(s, &t, 0..t.as_str().len());
    }
    Value::String(t.as_str().replace(find.as_str(), text(replacement).as_str()).into())
}

/// Longest string a program can build (characters): SPACE$(1E12) or
/// doubling a string in a loop stops with a run-time error instead of
/// exhausting memory.
pub const MAX_STRING_LEN: usize = 1 << 28;

/// A repetition count checked against [`MAX_STRING_LEN`].
fn repeat_count(n: &Value, what: &str) -> usize {
    let n = n.to_i64().max(0) as u64;
    if n > MAX_STRING_LEN as u64 {
        crate::runtime_error(&format!("{what}({n}): strings are limited to {MAX_STRING_LEN} characters"));
    }
    n as usize
}

/// SPACE$(n).
pub fn space(n: &Value) -> Value {
    Value::String(" ".repeat(repeat_count(n, "SPACE$")).into())
}

/// STRING$(n, c): `c` is a character code (`STRING$(3, 65)` = "AAA") or a
/// string whose first character repeats.
pub fn string_of(n: &Value, c: &Value) -> Value {
    let ch = match c {
        // (a real code rounded half to even, as RapidQ's: STRING$(2, 65.7) is "BB")
        Value::Double(d) => char::from_u32(crate::format::int32(*d).clamp(0, 0x10FFFF) as u32).unwrap_or('?'),
        Value::Integer(_) | Value::Boolean(_) => char::from_u32(c.to_i64().clamp(0, 0x10FFFF) as u32).unwrap_or('?'),
        other => other.to_string_val().chars().next().unwrap_or(' '),
    };
    Value::String(std::iter::repeat_n(ch, repeat_count(n, "STRING$")).collect())
}

/// ASC(s): character code of the first character (0 for an empty string),
/// the inverse of CHR$ — `ASC("é")` is 233, not a UTF-8 byte.
pub fn asc(s: &Value) -> Value {
    v_int(text(s).as_str().chars().next().map_or(0, |c| c as i64))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v_str;

    fn s(x: &str) -> Value {
        v_str(x)
    }

    #[test]
    fn space_and_string_of() {
        assert_eq!(space(&v_int(3)).to_string_val(), "   ");
        assert_eq!(space(&v_int(-2)).to_string_val(), "");
        assert_eq!(string_of(&v_int(3), &v_int(65)).to_string_val(), "AAA");
        assert_eq!(string_of(&v_int(2), &s("xyz")).to_string_val(), "xx");
    }

    #[test]
    fn character_positions_not_bytes() {
        assert_eq!(mid(&s("héllo"), &v_int(2), &v_int(1)).to_string_val(), "é");
        assert_eq!(left(&s("ñandú"), &v_int(2)).to_string_val(), "ña");
        assert_eq!(right(&s("ñandú"), &v_int(2)).to_string_val(), "dú");
        assert_eq!(len(&s("ñandú")).to_i64(), 5);
        assert_eq!(instr(&v_int(1), &s("añb"), &s("b")).to_i64(), 3);
        assert_eq!(rinstr(&Value::Null, &s("ñañ"), &s("ñ")).to_i64(), 3);
        assert_eq!(delete(&s("héllo"), &v_int(2), &v_int(2)).to_string_val(), "hlo");
        assert_eq!(insert(&s("hllo"), &v_int(2), &s("é")).to_string_val(), "héllo");
        assert_eq!(asc(&s("é")).to_i64(), 233);
    }

    #[test]
    fn out_of_range_arguments_are_clamped() {
        assert_eq!(mid(&s("abc"), &v_int(10), &v_int(2)).to_string_val(), "");
        assert_eq!(mid(&s("abc"), &v_int(0), &v_int(2)).to_string_val(), "ab");
        assert_eq!(mid(&s("hello"), &v_int(2), &Value::Null).to_string_val(), "ello");
        assert_eq!(left(&s("abc"), &v_int(-1)).to_string_val(), "");
        assert_eq!(right(&s("abc"), &v_int(99)).to_string_val(), "abc");
        assert_eq!(instr(&v_int(9), &s("abc"), &s("c")).to_i64(), 0);
        assert_eq!(delete(&s("abc"), &v_int(3), &v_int(99)).to_string_val(), "ab");
    }

    /// RC.EXE's answers (2026-10-22): a real start begins the search where
    /// it rounds half to even, and the place counts from it as given.
    #[test]
    fn instr_with_a_real_start_as_rapidq() {
        let r = |start: f64, h: &str, n: &str| instr(&Value::Double(start), &s(h), &s(n)).to_f64();
        assert_eq!(r(2.5, "abcabc", "c"), 3.5);
        assert_eq!(r(2.7, "abcabc", "b"), 4.7);
        assert_eq!(r(3.5, "abcabc", "c"), 5.5);
        assert_eq!(r(4.5, "abcabc", "a"), 4.5);
        assert_eq!(r(2.5, "abcabc", "z"), 0.0);
        assert!(matches!(instr(&Value::Double(2.0), &s("abcabc"), &s("c")), Value::Integer(3)));
    }

    /// RC.EXE's answers (2026-10-22): the needle ends at or before `start`.
    #[test]
    fn rinstr_with_a_start_as_rapidq() {
        let r = |start: Value, h: &str, n: &str| rinstr(&start, &s(h), &s(n)).to_i64();
        assert_eq!(r(Value::Null, "Hello World", "l"), 10);
        assert_eq!(r(v_int(4), "hehe they", "he"), 3);
        assert_eq!(r(v_int(3), "hehe they", "he"), 1);
        assert_eq!(r(v_int(1), "hehe they", "he"), 0);
        assert_eq!(r(v_int(0), "hehe they", "he"), 0);
        assert_eq!(r(v_int(-5), "hehe they", "he"), 0);
        assert_eq!(r(v_int(7), "hehe they", "he"), 3);
        assert_eq!(r(v_int(8), "hehe they", "he"), 7);
        assert_eq!(r(v_int(100), "hehe they", "he"), 7);
        assert_eq!(r(v_int(3), "aaaa", "aa"), 2);
        assert_eq!(r(v_int(5), "abcabc", "bc"), 2);
        assert_eq!(r(Value::Double(2.5), "abcabc", "c"), 0);
        assert_eq!(r(Value::Double(2.7), "abcabc", "c"), 3);
        assert_eq!(r(Value::Double(3e9), "abcabc", "c"), 0);
        assert_eq!(r(Value::Null, "", "a"), 0);
        // (RapidQ stops with an access violation on an empty needle)
        assert_eq!(r(Value::Null, "abc", ""), 0);
        assert_eq!(r(v_int(2), "abc", ""), 0);
    }

    #[test]
    fn replace_all_occurrences() {
        assert_eq!(replace_all(&s("aXbX"), &s("X"), &s("-")).to_string_val(), "a-b-");
        assert_eq!(replace_all(&s("abc"), &s(""), &s("-")).to_string_val(), "abc");
    }

    /// The functions as they were before the character index (STRING-SPEED,
    /// 2026-10-29): the text copied into a `Vec<char>` at every call. The
    /// index must give exactly their answers.
    mod walked {
        use crate::{v_int, Value};

        fn chars(v: &Value) -> Vec<char> {
            v.to_string_val().chars().collect()
        }
        fn index0(pos: &Value) -> usize {
            (pos.to_i64() - 1).max(0) as usize
        }
        fn count(n: &Value) -> usize {
            n.to_i64().max(0) as usize
        }
        fn find_chars(hay: &[char], needle: &[char], from: usize) -> Option<usize> {
            (from..=hay.len().saturating_sub(needle.len())).find(|&i| hay[i..].starts_with(needle))
        }
        pub fn len(s: &Value) -> i64 {
            s.to_string_val().chars().count() as i64
        }
        pub fn mid(s: &Value, start: &Value, length: &Value) -> String {
            let c = chars(s);
            let start = index0(start).min(c.len());
            let n = if matches!(length, Value::Null) { c.len() } else { count(length) };
            let end = start.saturating_add(n).min(c.len());
            c[start..end].iter().collect()
        }
        pub fn left(s: &Value, n: &Value) -> String {
            s.to_string_val().chars().take(count(n)).collect()
        }
        pub fn right(s: &Value, n: &Value) -> String {
            let c = chars(s);
            c[c.len().saturating_sub(count(n))..].iter().collect()
        }
        pub fn instr(start: &Value, hay: &Value, nee: &Value) -> Value {
            let (hay, nee) = (chars(hay), chars(nee));
            if nee.is_empty() {
                return v_int(0);
            }
            let start = match start {
                Value::Null => 1,
                s => match s.to_i64() {
                    0 => 1,
                    n => n,
                },
            };
            let from = (start - 1).max(0);
            match find_chars(&hay, &nee, from as usize) {
                Some(i) => v_int(i as i64 + 1 - from + (start - 1)),
                None => v_int(0),
            }
        }
        pub fn rinstr(start: &Value, hay: &Value, nee: &Value) -> i64 {
            let (hay, nee) = (chars(hay), chars(nee));
            let end = match start {
                Value::Null => hay.len(),
                s => crate::format::int32_of(s.to_i64()).clamp(0, hay.len() as i64) as usize,
            };
            if nee.is_empty() || nee.len() > end {
                return 0;
            }
            (0..=end - nee.len()).rev().find(|&i| hay[i..end].starts_with(&nee)).map_or(0, |i| i as i64 + 1)
        }
        pub fn insert(s: &Value, pos: &Value, text: &Value) -> String {
            let mut c = chars(s);
            let at = index0(pos).min(c.len());
            c.splice(at..at, text.to_string_val().chars());
            c.into_iter().collect()
        }
        pub fn delete(s: &Value, start: &Value, n: &Value) -> String {
            let mut c = chars(s);
            let from = index0(start).min(c.len());
            let to = from.saturating_add(count(n)).min(c.len());
            c.drain(from..to);
            c.into_iter().collect()
        }
    }

    /// A deterministic stream of small numbers (xorshift).
    struct Rand(u64);
    impl Rand {
        fn next(&mut self, below: u64) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0 % below
        }
    }

    /// Texts of every kind — empty, short, ASCII, bytes CHR$(0..255), wider
    /// characters, around the index's thresholds and well past 255
    /// characters — and positions all over them, past their ends and
    /// before their starts: the same answers as the walk.
    #[test]
    fn long_and_mixed_strings_answer_as_the_character_walk() {
        let mut r = Rand(0x9e3779b97f4a7c15);
        let alphabets: [&[char]; 4] = [
            &['a', 'b', 'c', ' ', 'z'],
            &['a', 'é', '€', '😀', '\u{0}', 'ÿ', '\u{80}'],
            &['é', 'ñ'],
            &['x', 'y', '日', '本'],
        ];
        for round in 0..400 {
            let alphabet = alphabets[round % alphabets.len()];
            let n = match round % 5 {
                0 => r.next(8),
                1 => 60 + r.next(10),
                2 => 250 + r.next(12),
                _ => r.next(3000),
            } as usize;
            let text: String = (0..n).map(|_| alphabet[r.next(alphabet.len() as u64) as usize]).collect();
            let v = v_str(&text);
            let chars = n as i64;
            assert_eq!(len(&v).to_i64(), walked::len(&v), "LEN of {n}");
            for _ in 0..40 {
                let p = r.next(chars as u64 + 6) as i64 - 3;
                let k = r.next(80) as i64 - 3;
                let (pv, kv) = (v_int(p), v_int(k));
                assert_eq!(mid(&v, &pv, &kv).to_string_val(), walked::mid(&v, &pv, &kv), "MID$({n} chars, {p}, {k})");
                assert_eq!(mid(&v, &pv, &Value::Null).to_string_val(), walked::mid(&v, &pv, &Value::Null), "MID$({n} chars, {p})");
                assert_eq!(left(&v, &pv).to_string_val(), walked::left(&v, &pv), "LEFT$({n} chars, {p})");
                assert_eq!(right(&v, &pv).to_string_val(), walked::right(&v, &pv), "RIGHT$({n} chars, {p})");
                assert_eq!(insert(&v, &pv, &s("ñx")).to_string_val(), walked::insert(&v, &pv, &s("ñx")), "INSERT$ at {p}");
                assert_eq!(delete(&v, &pv, &kv).to_string_val(), walked::delete(&v, &pv, &kv), "DELETE$({p}, {k})");
                // (a needle taken from the text, or one of its characters)
                let needle = if n > 0 && r.next(2) == 0 {
                    walked::mid(&v, &v_int(1 + r.next(n as u64) as i64), &v_int(1 + r.next(3) as i64))
                } else {
                    alphabet[r.next(alphabet.len() as u64) as usize].to_string()
                };
                let nv = v_str(&needle);
                assert_eq!(instr(&pv, &v, &nv), walked::instr(&pv, &v, &nv), "INSTR({p}, {n} chars, {needle:?})");
                assert_eq!(instr(&Value::Null, &v, &nv), walked::instr(&Value::Null, &v, &nv));
                assert_eq!(rinstr(&pv, &v, &nv).to_i64(), walked::rinstr(&pv, &v, &nv), "RINSTR({p}, {n} chars, {needle:?})");
                assert_eq!(rinstr(&Value::Null, &v, &nv).to_i64(), walked::rinstr(&Value::Null, &v, &nv));
                let at = mid(&v, &pv, &v_int(1));
                assert_eq!(asc(&at).to_i64(), walked::mid(&v, &pv, &v_int(1)).chars().next().map_or(0, |c| c as i64));
            }
        }
        // (a number's text, as before)
        assert_eq!(mid(&v_int(12345), &v_int(2), &v_int(3)).to_string_val(), "234");
        assert_eq!(len(&crate::v_dbl(2.5)).to_i64(), 3);
        assert_eq!(instr(&Value::Null, &v_int(1234), &v_int(3)).to_i64(), 3);
    }

    /// A slice that is the whole string is that string, shared (no copy).
    #[test]
    fn the_whole_string_is_shared_not_copied() {
        let v = v_str(&"é".repeat(500));
        let Value::String(original) = &v else { unreachable!() };
        for whole in [mid(&v, &v_int(1), &v_int(9999)), left(&v, &v_int(500)), right(&v, &v_int(600)), mid(&v, &v_int(0), &Value::Null)] {
            let Value::String(w) = &whole else { panic!() };
            assert!(crate::BasicString::ptr_eq(original, w));
        }
    }
}
