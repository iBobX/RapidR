// ----------------------------------------------------------- placeholders --

/// `text` with each `{name}` replaced by `value(name)`: `{{` and `}}` are
/// literal braces; a name `value` doesn't know (None) stays as written.
pub fn fill(text: &str, value: impl Fn(&str) -> Option<String>) -> String {
    if !text.contains(['{', '}']) {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len() + 16);
    let mut rest = text;
    while let Some(i) = rest.find(['{', '}']) {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        if tail.starts_with("{{") || tail.starts_with("}}") {
            out.push(tail.as_bytes()[0] as char);
            rest = &tail[2..];
        } else if let Some(after) = tail.strip_prefix('}') {
            out.push('}');
            rest = after;
        } else {
            // `{name}`: up to the next brace, which must close it.
            match tail[1..].find(['{', '}']) {
                Some(j) if tail.as_bytes()[1 + j] == b'}' => {
                    let name = &tail[1..1 + j];
                    match value(name.trim()) {
                        Some(v) => out.push_str(&v),
                        None => out.push_str(&tail[..j + 2]),
                    }
                    rest = &tail[j + 2..];
                }
                _ => {
                    out.push('{');
                    rest = &tail[1..];
                }
            }
        }
    }
    out.push_str(rest);
    out
}
