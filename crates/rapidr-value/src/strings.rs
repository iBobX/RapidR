//! BASIC string functions, shared by every runtime so both backends behave
//! identically. Positions and lengths count characters (not UTF-8 bytes) and
//! are 1-based, so `MID$("héllo", 2, 1)` is "é" rather than half a character.

use crate::{v_int, v_str, Value};

fn chars(v: &Value) -> Vec<char> {
    v.to_string_val().chars().collect()
}

/// Clamp a 1-based BASIC position to a 0-based char index.
fn index0(pos: &Value) -> usize {
    (pos.to_i64() - 1).max(0) as usize
}

fn count(n: &Value) -> usize {
    n.to_i64().max(0) as usize
}

/// Char index of the first occurrence of `needle` in `hay` at or after `from`.
fn find_chars(hay: &[char], needle: &[char], from: usize) -> Option<usize> {
    if needle.is_empty() {
        return (from <= hay.len()).then_some(from);
    }
    (from..=hay.len().saturating_sub(needle.len())).find(|&i| hay[i..].starts_with(needle))
}

pub fn len(s: &Value) -> Value {
    v_int(s.to_string_val().chars().count() as i64)
}

pub fn mid(s: &Value, start: &Value, length: &Value) -> Value {
    let c = chars(s);
    let start = index0(start).min(c.len());
    let end = start.saturating_add(count(length)).min(c.len());
    Value::String(c[start..end].iter().collect())
}

pub fn left(s: &Value, n: &Value) -> Value {
    Value::String(s.to_string_val().chars().take(count(n)).collect())
}

pub fn right(s: &Value, n: &Value) -> Value {
    let c = chars(s);
    let start = c.len().saturating_sub(count(n));
    Value::String(c[start..].iter().collect())
}

/// INSTR([start,] haystack, needle): 1-based position, 0 if absent.
pub fn instr(start: &Value, haystack: &Value, needle: &Value) -> Value {
    let hay = chars(haystack);
    match find_chars(&hay, &chars(needle), index0(start)) {
        Some(i) => v_int(i as i64 + 1),
        None => v_int(0),
    }
}

/// RINSTR(haystack, needle): 1-based position of the last occurrence.
pub fn rinstr(haystack: &Value, needle: &Value) -> Value {
    let hay = chars(haystack);
    let nee = chars(needle);
    if nee.len() > hay.len() {
        return v_int(0);
    }
    match (0..=hay.len() - nee.len()).rev().find(|&i| hay[i..].starts_with(&nee)) {
        Some(i) => v_int(i as i64 + 1),
        None => v_int(0),
    }
}

/// INSERT$(s, pos, text): insert `text` before 1-based position `pos`.
pub fn insert(s: &Value, pos: &Value, text: &Value) -> Value {
    let mut c = chars(s);
    let at = index0(pos).min(c.len());
    c.splice(at..at, text.to_string_val().chars());
    Value::String(c.into_iter().collect())
}

/// DELETE$(s, start, n): remove `n` characters from 1-based `start`.
pub fn delete(s: &Value, start: &Value, n: &Value) -> Value {
    let mut c = chars(s);
    let from = index0(start).min(c.len());
    let to = from.saturating_add(count(n)).min(c.len());
    c.drain(from..to);
    Value::String(c.into_iter().collect())
}

/// REPLACESUBSTR$(s, find, replacement): replace every occurrence.
pub fn replace_all(s: &Value, find: &Value, replacement: &Value) -> Value {
    let find = find.to_string_val();
    if find.is_empty() {
        return v_str(&s.to_string_val());
    }
    Value::String(s.to_string_val().replace(&find, &replacement.to_string_val()))
}

/// ASC(s): character code of the first character (0 for an empty string),
/// the inverse of CHR$ — `ASC("é")` is 233, not a UTF-8 byte.
pub fn asc(s: &Value) -> Value {
    v_int(s.to_string_val().chars().next().map_or(0, |c| c as i64))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(x: &str) -> Value {
        v_str(x)
    }

    #[test]
    fn character_positions_not_bytes() {
        assert_eq!(mid(&s("héllo"), &v_int(2), &v_int(1)).to_string_val(), "é");
        assert_eq!(left(&s("ñandú"), &v_int(2)).to_string_val(), "ña");
        assert_eq!(right(&s("ñandú"), &v_int(2)).to_string_val(), "dú");
        assert_eq!(len(&s("ñandú")).to_i64(), 5);
        assert_eq!(instr(&v_int(1), &s("añb"), &s("b")).to_i64(), 3);
        assert_eq!(rinstr(&s("ñañ"), &s("ñ")).to_i64(), 3);
        assert_eq!(delete(&s("héllo"), &v_int(2), &v_int(2)).to_string_val(), "hlo");
        assert_eq!(insert(&s("hllo"), &v_int(2), &s("é")).to_string_val(), "héllo");
        assert_eq!(asc(&s("é")).to_i64(), 233);
    }

    #[test]
    fn out_of_range_arguments_are_clamped() {
        assert_eq!(mid(&s("abc"), &v_int(10), &v_int(2)).to_string_val(), "");
        assert_eq!(mid(&s("abc"), &v_int(0), &v_int(2)).to_string_val(), "ab");
        assert_eq!(left(&s("abc"), &v_int(-1)).to_string_val(), "");
        assert_eq!(right(&s("abc"), &v_int(99)).to_string_val(), "abc");
        assert_eq!(instr(&v_int(9), &s("abc"), &s("c")).to_i64(), 0);
        assert_eq!(delete(&s("abc"), &v_int(3), &v_int(99)).to_string_val(), "ab");
    }

    #[test]
    fn replace_all_occurrences() {
        assert_eq!(replace_all(&s("aXbX"), &s("X"), &s("-")).to_string_val(), "a-b-");
        assert_eq!(replace_all(&s("abc"), &s(""), &s("-")).to_string_val(), "abc");
    }
}
