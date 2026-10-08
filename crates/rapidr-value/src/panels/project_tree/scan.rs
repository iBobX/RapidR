//! The components a source's CREATE blocks make, nested as written: a
//! small scanner the project tree lists a form's components with (the
//! designer model replaces it later; it stays this one function).
//!
//! It reads statements — lines, split at `:` outside strings — skipping
//! comments (`'` and `REM`) and strings, and follows
//! `CREATE name AS type … END CREATE`; anything else is passed over, and
//! text that doesn't close its blocks keeps what it opened.

/// A component a CREATE block makes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comp {
    pub name: String,
    /// Its type as written, uppercase (`QBUTTON`).
    pub ty: String,
    /// The line its CREATE is on (from 1).
    pub line: usize,
    pub children: Vec<Comp>,
}

/// The components `text`'s CREATE blocks make, nested as written.
pub fn creates(text: &str) -> Vec<Comp> {
    // (the blocks open now, innermost last)
    let mut open: Vec<Comp> = Vec::new();
    let mut out: Vec<Comp> = Vec::new();
    let close = |open: &mut Vec<Comp>, out: &mut Vec<Comp>| {
        if let Some(c) = open.pop() {
            match open.last_mut() {
                Some(parent) => parent.children.push(c),
                None => out.push(c),
            }
        }
    };
    for (n, line) in text.lines().enumerate() {
        for st in statements(line) {
            let words: Vec<&str> = st.split_whitespace().collect();
            match words.as_slice() {
                [c, name, kw, ty, ..] if c.eq_ignore_ascii_case("CREATE") && kw.eq_ignore_ascii_case("AS") => {
                    let ty: String = ty.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
                    let name: String = name.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '.').collect();
                    if name.is_empty() || ty.is_empty() {
                        continue;
                    }
                    open.push(Comp { name, ty: ty.to_ascii_uppercase(), line: n + 1, children: Vec::new() });
                }
                [e, c, ..] if e.eq_ignore_ascii_case("END") && c.eq_ignore_ascii_case("CREATE") => close(&mut open, &mut out),
                _ => {}
            }
        }
    }
    while !open.is_empty() {
        close(&mut open, &mut out);
    }
    out
}

/// A line's statements: split at `:` outside strings, without its comment
/// (`'` outside a string, or a statement starting `REM`); strings blanked
/// (their text can't look like a CREATE).
fn statements(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_str = false;
    for ch in line.chars() {
        if in_str {
            if ch == '"' {
                in_str = false;
            }
            cur.push(if ch == '"' { '"' } else { ' ' });
            continue;
        }
        match ch {
            '"' => {
                in_str = true;
                cur.push('"');
            }
            '\'' => break,
            ':' => {
                if !is_rem(&cur) {
                    out.push(std::mem::take(&mut cur));
                } else {
                    // (a REM runs to the line's end)
                    return out;
                }
            }
            c => cur.push(c),
        }
    }
    if !is_rem(&cur) {
        out.push(cur);
    }
    out
}

/// Whether a statement is a REM comment.
fn is_rem(st: &str) -> bool {
    let t = st.trim_start();
    t.get(..3).is_some_and(|h| h.eq_ignore_ascii_case("REM")) && t[3..].chars().next().is_none_or(|c| !(c.is_alphanumeric() || c == '_'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(c: &[Comp]) -> String {
        c.iter()
            .map(|c| if c.children.is_empty() { format!("{}:{}", c.name, c.ty) } else { format!("{}:{}({})", c.name, c.ty, names(&c.children)) })
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn nested_blocks_as_written() {
        let src = "CREATE Form1 AS QFORM\n  Caption = \"Hi\"\n  CREATE Panel1 AS QPANEL\n    CREATE Button1 AS QBUTTON\n    END CREATE\n  END CREATE\n  create Edit1 as qedit: Text = \"x\": end create\nEND CREATE\nCREATE Timer1 AS QTIMER\nEND CREATE\n";
        let c = creates(src);
        assert_eq!(names(&c), "Form1:QFORM(Panel1:QPANEL(Button1:QBUTTON) Edit1:QEDIT) Timer1:QTIMER");
        assert_eq!(c[0].line, 1);
        assert_eq!(c[0].children[0].children[0].line, 4);
    }

    #[test]
    fn comments_and_strings_are_skipped() {
        let src = "' CREATE Ghost AS QBUTTON\nREM CREATE Ghost2 AS QBUTTON\nCREATE F AS QFORM ' CREATE Ghost3 AS QLABEL\n  Caption = \"CREATE Ghost4 AS QEDIT: END CREATE\"\n  PRINT \"it's\": CREATE L AS QLABEL\n  END CREATE ' END CREATE\n  rem: END CREATE\n  Remark = 1: CREATE B AS QBUTTON: END CREATE\nEND CREATE\n";
        assert_eq!(names(&creates(src)), "F:QFORM(L:QLABEL B:QBUTTON)");
    }

    #[test]
    fn unclosed_blocks_keep_what_they_opened() {
        assert_eq!(names(&creates("CREATE F AS QFORM\nCREATE B AS QBUTTON\n")), "F:QFORM(B:QBUTTON)");
        assert_eq!(names(&creates("END CREATE\nCREATE X AS\nCREATE Y AS QEDIT\nEND CREATE")), "Y:QEDIT");
        assert!(creates("").is_empty());
    }
}
