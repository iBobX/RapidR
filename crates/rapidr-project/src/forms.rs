//! A form or a module added to a program (RapidR Studio's Project > Add
//! Form / Add Module, as Delphi's File > New > Form): the new file's text,
//! in the program's own names (RapidR's `RForm`, or RapidQ's `QFORM` for a
//! program written with them), and the `$INCLUDE` that brings it into the
//! main program — RapidQ's way of making a program of several files: an
//! included form is created where it is included, hidden until shown.

use crate::implicit::include_targets;

/// Whether a source file is written with RapidQ's names, so what is added
/// to its program is too: its first `CREATE … AS` type says (`QFORM`,
/// `QButton`); a file without one is RapidQ's when it is a `.bas` (RapidQ's
/// own extension), else RapidR's.
pub fn uses_rapidq_names(text: &str, path: &str) -> bool {
    for line in text.lines() {
        let words: Vec<&str> = line.split_whitespace().collect();
        if let [c, _, kw, ty, ..] = words.as_slice() {
            if c.eq_ignore_ascii_case("CREATE") && kw.eq_ignore_ascii_case("AS") {
                return ty.starts_with(['Q', 'q']);
            }
        }
    }
    path.to_ascii_lowercase().ends_with(".bas")
}

/// The line ends `text` uses (`\r\n` when it has any, else `\n`).
pub fn line_end(text: &str) -> &'static str {
    if text.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

/// A new form file's text: `CREATE name AS RForm` (`QFORM` with RapidQ's
/// names) with its Caption and RapidQ's starting size, hidden until the
/// program shows it (`name.Show`, `name.ShowModal`).
pub fn new_form_text(file_title: &str, name: &str, rapidq_names: bool, eol: &str) -> String {
    let ty = if rapidq_names { "QFORM" } else { "RForm" };
    let caption = crate::basic_string(name);
    [
        format!("' {file_title}: the program shows {name} with {name}.Show"),
        String::new(),
        format!("CREATE {name} AS {ty}"),
        format!("    Caption = {caption}"),
        "    Width = 320".into(),
        "    Height = 240".into(),
        "END CREATE".into(),
        String::new(),
    ]
    .join(eol)
}

/// A new module's text: a comment naming it, for its SUBs and FUNCTIONs.
pub fn new_module_text(file_title: &str, eol: &str) -> String {
    format!("' {file_title}: SUBs and FUNCTIONs the program's files share{eol}{eol}")
}

/// Whether `main` already `$INCLUDE`s `file` (by its path as written, `\`
/// or `/`, any case).
pub fn includes(main: &str, file: &str) -> bool {
    let norm = |p: &str| p.trim().replace('\\', "/").trim_start_matches("./").to_ascii_lowercase();
    let want = norm(file);
    include_targets(main).iter().any(|t| norm(t) == want)
}

/// Where `$INCLUDE "file"` goes in the main program `main`: (the line, from
/// 0, it is inserted before; the text inserted), or `None` when `main`
/// includes it already.
///
/// After the program's last `$INCLUDE` (the files it includes stay in the
/// order they were added: a later one may use an earlier one's forms);
/// else after the directives that open it (`$APPTYPE`, `$TYPECHECK`,
/// `$OPTION` … among its first comments); else before its first statement,
/// a blank line after. Always before the code that shows the form.
pub fn include_insertion(main: &str, file: &str) -> Option<(usize, String)> {
    if includes(main, file) {
        return None;
    }
    let eol = line_end(main);
    let line = format!("$INCLUDE \"{}\"", file.replace('\\', "/"));
    let lines: Vec<&str> = main.lines().collect();
    let directive = |l: &str| l.trim_start().starts_with('$');
    let is_include = |l: &str| l.trim_start().get(..8).is_some_and(|h| h.eq_ignore_ascii_case("$INCLUDE"));
    // (the conditional directives open a block: what follows them isn't the
    // program's header any more)
    let conditional = |l: &str| {
        let w = l.trim_start().split_whitespace().next().unwrap_or("").to_ascii_uppercase();
        matches!(w.as_str(), "$IF" | "$IFDEF" | "$IFNDEF" | "$ELSE" | "$ELSEIF" | "$ENDIF" | "$END")
    };
    let comment = |l: &str| {
        let t = l.trim_start();
        t.starts_with('\'') || t.get(..4).is_some_and(|h| h.eq_ignore_ascii_case("REM ")) || t.eq_ignore_ascii_case("REM")
    };
    if let Some(last) = lines.iter().rposition(|l| is_include(l)) {
        return Some((last + 1, format!("{line}{eol}")));
    }
    // the header: comments, blank lines and directives before the first
    // statement
    let mut last_directive = None;
    let mut first_statement = lines.len();
    for (i, l) in lines.iter().enumerate() {
        if l.trim().is_empty() || comment(l) {
            continue;
        }
        if directive(l) && !conditional(l) {
            last_directive = Some(i);
            continue;
        }
        first_statement = i;
        break;
    }
    Some(match last_directive {
        Some(i) => (i + 1, format!("{line}{eol}")),
        None if first_statement < lines.len() => (first_statement, format!("{line}{eol}{eol}")),
        // (nothing but comments, or nothing: at the end)
        None => (lines.len(), if main.is_empty() || main.ends_with('\n') { format!("{line}{eol}") } else { format!("{eol}{line}{eol}") }),
    })
}

/// `main` with `$INCLUDE "file"` inserted ([`include_insertion`]).
pub fn with_include(main: &str, file: &str) -> String {
    let Some((at, text)) = include_insertion(main, file) else { return main.to_string() };
    let mut offset = 0;
    for (i, l) in main.split_inclusive('\n').enumerate() {
        if i == at {
            break;
        }
        offset += l.len();
    }
    if at >= main.split_inclusive('\n').count() {
        offset = main.len();
    }
    let mut out = String::with_capacity(main.len() + text.len());
    out.push_str(&main[..offset]);
    out.push_str(&text);
    out.push_str(&main[offset..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_include_goes_after_the_last_one_or_the_header() {
        // after the directives that open the program
        let gui = "$APPTYPE GUI\n\nCREATE Form1 AS RForm\nEND CREATE\n\nForm1.ShowModal\n";
        assert_eq!(with_include(gui, "Form2.rr"), "$APPTYPE GUI\n$INCLUDE \"Form2.rr\"\n\nCREATE Form1 AS RForm\nEND CREATE\n\nForm1.ShowModal\n");
        // after the last include, the order kept
        let two = with_include(&with_include(gui, "Form2.rr"), "Module1.rr");
        assert!(two.starts_with("$APPTYPE GUI\n$INCLUDE \"Form2.rr\"\n$INCLUDE \"Module1.rr\"\n\nCREATE"), "{two}");
        // once
        assert_eq!(include_insertion(&two, "form2.RR"), None);
        assert_eq!(include_insertion("$include \"sub\\Form3.rr\"\n", "sub/form3.rr"), None);
        // comments first, no directive: before the first statement, a
        // blank line after
        let plain = "' my program\n' does things\n\nCREATE Form1 AS RForm\nEND CREATE\n";
        assert_eq!(with_include(plain, "Form2.rr"), "' my program\n' does things\n\n$INCLUDE \"Form2.rr\"\n\nCREATE Form1 AS RForm\nEND CREATE\n");
        // the line ends the file uses
        assert_eq!(with_include("$APPTYPE GUI\r\nPRINT 1\r\n", "F.rr"), "$APPTYPE GUI\r\n$INCLUDE \"F.rr\"\r\nPRINT 1\r\n");
        // a conditional block isn't the header
        assert_eq!(include_insertion("$IFDEF X\nPRINT 1\n$ENDIF\n", "F.rr"), Some((0, "$INCLUDE \"F.rr\"\n\n".into())));
        // empty, or only comments
        assert_eq!(with_include("", "F.rr"), "$INCLUDE \"F.rr\"\n");
        assert_eq!(with_include("' nothing yet", "F.rr"), "' nothing yet\n$INCLUDE \"F.rr\"\n");
    }

    #[test]
    fn new_files_in_the_programs_names() {
        let f = new_form_text("Form2.rr", "Form2", false, "\n");
        assert_eq!(f, "' Form2.rr: the program shows Form2 with Form2.Show\n\nCREATE Form2 AS RForm\n    Caption = \"Form2\"\n    Width = 320\n    Height = 240\nEND CREATE\n");
        assert!(new_form_text("About.bas", "About", true, "\r\n").contains("CREATE About AS QFORM\r\n"));
        assert!(uses_rapidq_names("$APPTYPE GUI\nCREATE Form1 AS QFORM\n", "main.rr"));
        assert!(!uses_rapidq_names("CREATE Form1 AS RForm\n", "main.bas"));
        assert!(uses_rapidq_names("PRINT 1\n", "main.bas") && !uses_rapidq_names("PRINT 1\n", "main.rr"));
        assert_eq!(new_module_text("Module1.rr", "\n"), "' Module1.rr: SUBs and FUNCTIONs the program's files share\n\n");
    }
}
