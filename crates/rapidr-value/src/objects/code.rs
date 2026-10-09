//! RCODEEDITOR's outline (`codeedit::CodeEditor`): the SUBs and FUNCTIONs
//! it finds (GetSubList, GotoSub) and where a line starts (GotoLine) — the
//! same on every runtime. RapidQ has no code editor, so these rules are
//! RapidR's own.

/// The SUB or FUNCTION a line declares (its name), if it does.
fn declared(line: &str) -> Option<&str> {
    let t = line.trim();
    let upper = t.to_ascii_uppercase();
    if !(upper.starts_with("SUB ") || upper.starts_with("FUNCTION ")) {
        return None;
    }
    t.split('(').next()?.split_whitespace().nth(1)
}

/// GetSubList: the SUBs' and FUNCTIONs' names, in order.
pub fn sub_list(text: &str) -> Vec<String> {
    text.lines().filter_map(declared).map(str::to_string).collect()
}

/// GotoSub's line: the first SUB / FUNCTION line whose text contains
/// `name` (any case).
pub fn sub_line(text: &str, name: &str) -> Option<usize> {
    let target = name.to_uppercase();
    text.lines().position(|l| declared(l).is_some() && l.trim().to_uppercase().contains(&target))
}

/// Where line `n` (from 0) starts, in characters ('\n' counts one); past
/// the last line, the text's end.
pub fn line_start(text: &str, n: usize) -> usize {
    let mut at = 0;
    for (i, line) in text.split('\n').enumerate() {
        if i == n {
            return at;
        }
        at += line.chars().count() + 1;
    }
    text.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subs_and_lines() {
        let src = "DIM a\nSUB Foo(x AS INTEGER)\nEND SUB\n  function Bar\nsubroutine\nSUB Foo2";
        assert_eq!(sub_list(src), ["Foo", "Bar", "Foo2"]);
        assert_eq!(sub_line(src, "bar"), Some(3));
        assert_eq!(sub_line(src, "foo2"), Some(5));
        assert_eq!(sub_line(src, "Baz"), None);
        assert_eq!(line_start(src, 0), 0);
        assert_eq!(line_start(src, 2), 28);
        assert_eq!(line_start(src, 99), src.chars().count());
    }
}
