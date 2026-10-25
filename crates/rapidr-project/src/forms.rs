//! A form or a module added to a program (RapidR Studio's Project > Add
//! Form / Add Module, as RapidQ's File > New > Form): the new file's text,
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
/// names; `mdi`: an MDI main window, `RFormMDI` / `QFORMMDI`) with its
/// Caption and RapidQ's starting size, hidden until the program shows it
/// (`name.Show`, `name.ShowModal`).
pub fn new_form_text(file_title: &str, name: &str, rapidq_names: bool, mdi: bool, eol: &str) -> String {
    let ty = match (rapidq_names, mdi) {
        (true, false) => "QFORM",
        (false, false) => "RForm",
        (true, true) => "QFORMMDI",
        (false, true) => "RFormMDI",
    };
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

/// Where `main`'s `$INCLUDE` of `old` names it, for the file renamed or
/// moved to `new`: (the line, from 0; the path's first and last-but-one
/// columns, in characters; the new path as written) — `None` when `main`
/// doesn't include `old`. The path is replaced alone: the line's quotes,
/// spacing and comment stay.
pub fn include_rename(main: &str, old: &str, new: &str) -> Option<(usize, usize, usize, String)> {
    let norm = |p: &str| p.trim().replace('\\', "/").trim_start_matches("./").to_ascii_lowercase();
    let want = norm(old);
    for (i, line) in main.lines().enumerate() {
        let t = line.trim_start();
        if !t.get(..8).is_some_and(|h| h.eq_ignore_ascii_case("$INCLUDE")) {
            continue;
        }
        let Some(open) = line.find(['"', '<']) else { continue };
        let close = if line[open..].starts_with('"') { '"' } else { '>' };
        let Some(len) = line[open + 1..].find(close) else { continue };
        let path = &line[open + 1..open + 1 + len];
        if norm(path) != want {
            continue;
        }
        let start = line[..open + 1].chars().count();
        let end = start + path.chars().count();
        // (written with the old one's separators)
        let written = if path.contains('\\') { new.replace('/', "\\") } else { new.replace('\\', "/") };
        return Some((i, start, end, written));
    }
    None
}

/// The line (from 0) of `main`'s `$INCLUDE` of `file`, to take out with
/// the file (Project > Remove: the program no longer includes it); `None`
/// when it doesn't include it.
pub fn include_line(main: &str, file: &str) -> Option<usize> {
    let norm = |p: &str| p.trim().replace('\\', "/").trim_start_matches("./").to_ascii_lowercase();
    let want = norm(file);
    main.lines().position(|line| include_targets(line).iter().any(|t| norm(t) == want))
}

/// Whether `line` is a `$THEME` directive (`$THEME name`, any case); the
/// text after it, inline comment and quotes taken off.
fn theme_directive(line: &str) -> Option<String> {
    let t = line.trim_start();
    t.get(..6).filter(|h| h.eq_ignore_ascii_case("$THEME"))?;
    let after = &t[6..];
    // ($THEMES, $THEMEX … are other words)
    if after.chars().next().is_some_and(|c| !c.is_whitespace() && c != '\'') {
        return None;
    }
    let name = after.split('\'').next().unwrap_or("").trim().trim_matches('"').trim();
    Some(name.to_string())
}

/// The line (from 0) of the `$THEME` the program ends up with — the last
/// one, which is the one that runs last — and the name it gives (as
/// written, "" when it names none).
pub fn theme_line(main: &str) -> Option<(usize, String)> {
    main.lines().enumerate().filter_map(|(i, l)| theme_directive(l).map(|n| (i, n))).last()
}

/// The look the main program names: its `$THEME` line's name as written
/// ("" when it has none: RapidR's look as the system is).
pub fn theme_of(main: &str) -> String {
    theme_line(main).map(|(_, n)| n).unwrap_or_default()
}

/// One edit of a text: from (line, column) to (line, column), both from 0
/// in characters, replaced by `text`.
pub type Edit = (usize, usize, usize, usize, String);

/// The edit that makes the main program's look `name` ("" for RapidR's
/// default, which a program says by saying nothing): its `$THEME` line
/// changed in place, taken out for the default, or — when it has none —
/// inserted where the program's header ends (after `$APPTYPE` and the
/// directives like it, before the `$INCLUDE`s that make its forms; else
/// before its first statement). `None` when the text is that already.
pub fn theme_edit(main: &str, name: &str) -> Option<Edit> {
    let name = name.trim();
    let eol = line_end(main);
    match theme_line(main) {
        Some((i, written)) => {
            if written == name {
                return None;
            }
            if name.is_empty() {
                return Some((i, 0, i + 1, 0, String::new()));
            }
            let line = main.lines().nth(i).unwrap_or("");
            let indent = line.chars().take_while(|c| c.is_whitespace()).count();
            Some((i, indent, i, line.chars().count(), format!("$THEME {name}")))
        }
        None if name.is_empty() => None,
        None => {
            let lines: Vec<&str> = main.lines().collect();
            let comment = |l: &str| {
                let t = l.trim_start();
                t.starts_with('\'') || t.get(..4).is_some_and(|h| h.eq_ignore_ascii_case("REM ")) || t.eq_ignore_ascii_case("REM")
            };
            let word = |l: &str| l.trim_start().split_whitespace().next().unwrap_or("").to_ascii_uppercase();
            let (mut after_directive, mut first_include, mut first_statement) = (None, None, lines.len());
            for (i, l) in lines.iter().enumerate() {
                if l.trim().is_empty() || comment(l) {
                    continue;
                }
                let w = word(l);
                if w == "$INCLUDE" {
                    first_include.get_or_insert(i);
                    continue;
                }
                if w.starts_with('$') && !matches!(w.as_str(), "$IF" | "$IFDEF" | "$IFNDEF" | "$ELSE" | "$ELSEIF" | "$ENDIF" | "$END") && first_include.is_none() {
                    after_directive = Some(i + 1);
                    continue;
                }
                first_statement = i;
                break;
            }
            let at = after_directive.or(first_include).unwrap_or(first_statement);
            // (nothing after the last line's end: a line of its own)
            let text = if at >= lines.len() && !(main.is_empty() || main.ends_with('\n')) { format!("{eol}$THEME {name}{eol}") } else { format!("$THEME {name}{eol}") };
            let at = at.min(lines.len());
            Some((at, 0, at, 0, text))
        }
    }
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

    /// Applies an [`Edit`] to a text (lines and columns in characters).
    fn applied(text: &str, e: Option<Edit>) -> String {
        let Some((sl, sc, el, ec, t)) = e else { return text.to_string() };
        let lines: Vec<&str> = text.split_inclusive('\n').collect();
        let offset = |l: usize, c: usize| -> usize {
            if l >= lines.len() {
                return text.len();
            }
            let before: usize = lines[..l].iter().map(|x| x.len()).sum();
            before + lines[l].char_indices().nth(c).map_or(lines[l].len(), |(b, _)| b)
        };
        let (a, b) = (offset(sl, sc), offset(el, ec));
        format!("{}{}{}", &text[..a], t, &text[b..])
    }

    #[test]
    fn the_program_names_its_look_in_one_line() {
        let gui = "' my app\n$APPTYPE GUI\n$INCLUDE \"Form2.rr\"\n\nCREATE Form1 AS RForm\nEND CREATE\n";
        assert_eq!(theme_of(gui), "");
        // inserted after the header's directives, before the includes
        let dark = applied(gui, theme_edit(gui, "RapidR Dark"));
        assert_eq!(dark, "' my app\n$APPTYPE GUI\n$THEME RapidR Dark\n$INCLUDE \"Form2.rr\"\n\nCREATE Form1 AS RForm\nEND CREATE\n");
        assert_eq!(theme_of(&dark), "RapidR Dark");
        // changed in place, the same twice is nothing
        let light = applied(&dark, theme_edit(&dark, "RapidR Light"));
        assert_eq!(light, dark.replace("Dark", "Light"));
        assert_eq!(theme_edit(&light, "RapidR Light"), None);
        // the default is no line: back to exactly the first text
        assert_eq!(applied(&dark, theme_edit(&dark, "")), gui);
        assert_eq!(theme_edit(gui, ""), None);
        // without a directive: before the first statement; the line ends the file uses
        let plain = "' hi\r\nCREATE F AS RForm\r\nEND CREATE\r\n";
        let t = applied(plain, theme_edit(plain, "Classic"));
        assert_eq!(t, "' hi\r\n$THEME Classic\r\nCREATE F AS RForm\r\nEND CREATE\r\n");
        assert_eq!(applied(&t, theme_edit(&t, "")), plain);
        // nothing, or comments with no end of line
        assert_eq!(applied("", theme_edit("", "Classic")), "$THEME Classic\n");
        assert_eq!(applied("' x", theme_edit("' x", "Classic")), "' x\n$THEME Classic\n");
        // as written: quotes, a comment, any case, indentation; the last one wins
        assert_eq!(theme_of("  $theme \"Dark\" ' night\n"), "Dark");
        assert_eq!(theme_of("$THEMES x\n$THEME a\n$THEME b\n"), "b");
        assert_eq!(applied("  $theme dark ' night\n", theme_edit("  $theme dark ' night\n", "Classic")), "  $THEME Classic\n");
    }

    #[test]
    fn a_renamed_file_is_included_by_its_new_name() {
        let main = "$APPTYPE GUI\n$INCLUDE \"Form2.rr\"   ' the second form\n$include \"lib\\Util.inc\"\n";
        assert_eq!(include_rename(main, "form2.rr", "About.rr"), Some((1, 10, 18, "About.rr".into())));
        assert_eq!(include_rename(main, "lib/util.inc", "lib/Tools.inc"), Some((2, 10, 22, "lib\\Tools.inc".into())));
        assert_eq!(include_rename(main, "Other.rr", "X.rr"), None);
        assert_eq!(include_line(main, "LIB/util.inc"), Some(2));
        assert_eq!(include_line(main, "Other.rr"), None);
    }

    #[test]
    fn new_files_in_the_programs_names() {
        let f = new_form_text("Form2.rr", "Form2", false, false, "\n");
        assert_eq!(f, "' Form2.rr: the program shows Form2 with Form2.Show\n\nCREATE Form2 AS RForm\n    Caption = \"Form2\"\n    Width = 320\n    Height = 240\nEND CREATE\n");
        assert!(new_form_text("About.bas", "About", true, false, "\r\n").contains("CREATE About AS QFORM\r\n"));
        assert!(new_form_text("Main2.rr", "Main2", false, true, "\n").contains("CREATE Main2 AS RFormMDI\n"));
        assert!(uses_rapidq_names("$APPTYPE GUI\nCREATE Form1 AS QFORM\n", "main.rr"));
        assert!(!uses_rapidq_names("CREATE Form1 AS RForm\n", "main.bas"));
        assert!(uses_rapidq_names("PRINT 1\n", "main.bas") && !uses_rapidq_names("PRINT 1\n", "main.rr"));
        assert_eq!(new_module_text("Module1.rr", "\n"), "' Module1.rr: SUBs and FUNCTIONs the program's files share\n\n");
    }
}
