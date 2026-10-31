//! A text against a template with `{name}` where a value goes: the values
//! taken out, so a text built from a template (the compiler's messages, the
//! assistant's step notes) can be found again and translated whole: the
//! template's translation filled with the same values.

#[derive(Debug)]
pub enum Part<'a> {
    Lit(&'a str),
    Var(&'a str),
}

pub fn parts(template: &str) -> Vec<Part<'_>> {
    let mut out = Vec::new();
    let mut rest = template;
    while let Some(i) = rest.find('{') {
        let Some(j) = rest[i..].find('}') else { break };
        if i > 0 {
            out.push(Part::Lit(&rest[..i]));
        }
        out.push(Part::Var(&rest[i + 1..i + j]));
        rest = &rest[i + j + 1..];
    }
    if !rest.is_empty() {
        out.push(Part::Lit(rest));
    }
    out
}

/// `text` against template parts from `k`: the values taken (in order), and
/// how much of `text` is left over when `prefix` allows that.
pub fn match_from(parts: &[Part], k: usize, text: &str, values: &mut Vec<(String, String)>, prefix: bool) -> Option<usize> {
    let Some(part) = parts.get(k) else {
        return if text.is_empty() || prefix { Some(text.len()) } else { None };
    };
    match part {
        Part::Lit(l) => {
            let rest = text.strip_prefix(l)?;
            match_from(parts, k + 1, rest, values, prefix)
        }
        Part::Var(name) => {
            // (the same name twice: the same value)
            if let Some((_, v)) = values.iter().find(|(n, _)| n == name).cloned() {
                let rest = text.strip_prefix(v.as_str())?;
                return match_from(parts, k + 1, rest, values, prefix);
            }
            match parts.get(k + 1) {
                // (the last: the rest of the text, or the next word when a
                // longer message may follow)
                None if !prefix => {
                    if text.is_empty() {
                        return None;
                    }
                    values.push((name.to_string(), text.to_string()));
                    Some(0)
                }
                None => {
                    let end = text.find(|c: char| c.is_whitespace() || c == ',' || c == ';').unwrap_or(text.len());
                    if end == 0 {
                        return None;
                    }
                    values.push((name.to_string(), text[..end].to_string()));
                    Some(text.len() - end)
                }
                Some(Part::Lit(next)) => {
                    // (the shortest value the rest matches with)
                    let mut from = 1;
                    while from <= text.len() {
                        let Some(at) = text.get(from..).and_then(|t| t.find(next)).map(|i| i + from) else { return None };
                        values.push((name.to_string(), text[..at].to_string()));
                        if let Some(left) = match_from(parts, k + 1, &text[at..], values, prefix) {
                            return Some(left);
                        }
                        values.pop();
                        from = at + 1;
                        while from < text.len() && !text.is_char_boundary(from) {
                            from += 1;
                        }
                    }
                    None
                }
                Some(Part::Var(_)) => None,
            }
        }
    }
}


/// The values `text` is made of, when it is exactly `template` with its
/// `{name}`s filled (name, value; in order); None when it is not.
pub fn match_whole(template: &str, text: &str) -> Option<Vec<(String, String)>> {
    let ps = parts(template);
    let mut values = Vec::new();
    match_from(&ps, 0, text, &mut values, false).map(|_| values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_text_gives_back_its_values() {
        let v = match_whole("Read {file} ({lines} lines)", "Read main.rr (12 lines)").unwrap();
        assert_eq!(v, [("file".to_string(), "main.rr".to_string()), ("lines".to_string(), "12".to_string())]);
        assert!(match_whole("Read {file} ({lines} lines)", "Wrote main.rr").is_none());
        assert_eq!(match_whole("Listed the files", "Listed the files"), Some(vec![]));
        assert!(match_whole("Listed the files", "Listed the files again").is_none());
    }
}
