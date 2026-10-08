//! Putting program-derived strings (a project's name, its files' names) into
//! the pages a web build generates, each by the rules of where it goes
//! (docs/security-audit.md SEC-16): HTML text and attribute values, a
//! JavaScript string, a URL path segment, a file name inside the bundle.
//! Nothing a project names may become markup, script or another path.

/// `s` as HTML text or a quoted attribute value: `& < > " '` as references.
pub fn html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            // (control characters have no business in a title)
            c if c.is_control() => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// `s` as a double-quoted JavaScript (and JSON) string literal, quotes
/// included, that stays one inside an HTML `<script>` too: `<`, `>`, `&`
/// and the line separators are `\u` escapes, so `</script>` or `<!--`
/// can't end or change the script element around it.
pub fn js_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '<' | '>' | '&' | '\'' | '\u{2028}' | '\u{2029}' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c if (c as u32) < 0x20 || c == '\u{7f}' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `s` as one URL path segment (as JavaScript's `encodeURIComponent`): only
/// `A-Z a-z 0-9 - _ . ! ~ * ' ( )` stay, everything else is `%XX` of its
/// UTF-8 bytes — no `/`, `?`, `#` or `:` can make it another place.
pub fn url_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'(' | b')' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// A name made safe as a file name in the bundle's root: letters (any
/// script), digits, `-`, `_`, `.`, space and `()` stay; anything else (a
/// path separator, `:`, a control character, a quote…) becomes `_`; no
/// leading dot (so never `.`, `..` or a hidden file); `program` when nothing
/// is left. An ordinary name (`hello_web`, `Mi programa`) is unchanged.
pub fn file_name(name: &str) -> String {
    let mut out: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | ' ' | '(' | ')') { c } else { '_' })
        .collect();
    while out.starts_with('.') || out.starts_with(' ') {
        out.remove(0);
    }
    while out.ends_with(' ') || out.ends_with('.') {
        out.pop();
    }
    if out.is_empty() {
        "program".into()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_escapes_markup_and_quotes() {
        assert_eq!(html(r#"</title><img src=x onerror="a('b')">&"#), "&lt;/title&gt;&lt;img src=x onerror=&quot;a(&#39;b&#39;)&quot;&gt;&amp;");
        assert_eq!(html("Mi programa ñ"), "Mi programa ñ");
    }

    #[test]
    fn js_string_cannot_close_the_script() {
        let s = js_string("x</script><script>alert(1)</script>\"\\\u{2028}");
        assert!(!s.contains("</script"), "{s}");
        assert!(!s.contains('<'), "{s}");
        // (each `<` below stands for a backslash: the JavaScript escapes)
        let expected = "\"x<u003c/script<u003e<u003cscript<u003ealert(1)<u003c/script<u003e<\"<<<u2028\"".replace('<', "\u{5c}");
        assert_eq!(s, expected);
    }

    #[test]
    fn url_component_keeps_one_segment() {
        assert_eq!(url_component("a b/../c?d#e"), "a%20b%2F..%2Fc%3Fd%23e");
        assert_eq!(url_component("hello_web"), "hello_web");
    }

    #[test]
    fn file_names_stay_in_the_root() {
        assert_eq!(file_name("hello_web"), "hello_web");
        assert_eq!(file_name("../../etc/passwd"), "_.._etc_passwd");
        assert_eq!(file_name(".."), "program");
        assert_eq!(file_name("a\\b:c\u{0}"), "a_b_c_");
        assert_eq!(file_name("Mi programa (1)"), "Mi programa (1)");
    }
}
