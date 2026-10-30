//! What the compiler doesn't say, because RapidQ compiles it and it runs:
//! in a RapidQ program (`.bas` …), a constant of RAPIDQ.INC (or of another
//! RapidQ include) used without the `$INCLUDE` is RapidQ's implicit
//! variable, always 0 (`mbYes` → 0: a MessageDlg with only an OK button).
//! RapidR keeps that meaning there; this is a warning beside the squiggles
//! and in Problems, its fix adding the include. It reads the compiler's
//! semantic model: its implicit symbols and how each use reads or writes
//! them (a name the program stores in is its own variable). A RapidR
//! program (`.rr`) has RAPIDQ.INC's constants built in: never this hint.
//!
//! Also `NOT FILEEXISTS(x)` ([`not_one`]): FILEEXISTS gives 1, NOT 1 is -2,
//! so the test is always true — a warning, its fix `FILEEXISTS(x) = 0`.

use std::path::Path;

use rapidr_diagnostics::Severity;
use rapidr_lang::Origin;

use crate::model::{Access, SymbolKind};
use crate::{CodeAction, FileDiagnostic, Snapshot, TextEdit};

pub(crate) const RAPIDQ_INC: &str = "RAPIDQ.INC";
/// A RapidQ include's constant used without the include.
pub(crate) const NEEDS_INCLUDE: &str = "needs-include";
/// `NOT FILEEXISTS(x)`: always true (docs/rai-mistake-ledger.md).
pub(crate) const NOT_ONE: &str = "not-one-is-yes";
/// A FUNCTION's parameter or DIM named `Result` (never reachable there).
pub(crate) const HIDDEN_RESULT: &str = "hidden-result";

/// `NOT` before a builtin that gives 1 for yes — FILEEXISTS, DIREXISTS (the
/// registry's: their docs say "True (1)") —: RapidQ-exact, they give 1,
/// and NOT 1 is -2, true as well, so the test is always true. A warning,
/// its fix `FILEEXISTS(x) = 0` ([`not_one_fix`]). A model wrote it on
/// Robert's evening with DeepSeek (2026-10-20), and so do people.
fn not_one(s: &Snapshot) -> Vec<FileDiagnostic> {
    use rapidr_ast::{Expression, UnaryOperator};
    let mut found: Vec<(rapidr_diagnostics::TextSpan, rapidr_diagnostics::TextSpan, &'static str)> = Vec::new();
    rapidr_ast::walk(&s.parsed.program.statements, &mut |_| {}, &mut |e| {
        let Expression::Unary(u) = e else { return };
        if u.operator != UnaryOperator::Not {
            return;
        }
        let Expression::FunctionCall(c) = u.operand.as_ref() else { return };
        let Expression::Identifier(name) = c.callee.as_ref() else { return };
        if let Some(b) = rapidr_lang::builtin(&name.name).filter(|b| b.doc.starts_with("True (1)")) {
            found.push((u.span, c.span, b.name));
        }
    });
    let mut out = Vec::new();
    for (span, call, name) in found {
        let (Some(at), Some(c)) = (s.locate_range(span), s.locate_range(call)) else { continue };
        if c.file != at.file {
            continue;
        }
        let written = s.parsed.file_text(&c.file).and_then(|t| t.get(c.start..c.end)).unwrap_or(name).to_string();
        let shown = if written.chars().count() > 40 || written.contains('\n') { format!("{name}(…)") } else { written };
        out.push(FileDiagnostic {
            file: at.file,
            start: at.start,
            end: at.end.max(at.start + 1),
            severity: Severity::Warning,
            message: format!("{name} gives 1, and NOT 1 is -2 (true): write {shown} = 0"),
            code: Some(NOT_ONE.into()),
            hint: None,
        });
    }
    out
}

/// A FUNCTION's parameter or DIM named `Result`: inside the FUNCTION,
/// `Result` is its own value (RapidQ's RESULT), so that variable can never
/// be read or set (rapidr_ast::function_result) — compiled as RapidQ does,
/// and a warning here.
fn hidden_result(s: &Snapshot) -> Vec<FileDiagnostic> {
    let mut out = Vec::new();
    for (span, function) in rapidr_ast::function_result::hidden(&s.parsed.program) {
        let Some(at) = s.locate_range(span) else { continue };
        out.push(FileDiagnostic {
            file: at.file,
            start: at.start,
            end: at.end.max(at.start + 1),
            severity: Severity::Warning,
            message: format!("Inside FUNCTION {function}, Result is the FUNCTION's own value (RESULT): this variable can't be used there. Give it another name"),
            code: Some(HIDDEN_RESULT.into()),
            hint: None,
        });
    }
    out
}

/// Another language's code RapidR compiles, and what it does there isn't
/// what its writer meant (docs/manual/diagnostics.md, "Coming from another
/// language?"):
///
/// * `a && b` — RapidQ takes `&&` (two `&`: text joining), so it compiles
///   and the test is never true; RapidR's error for it would not be RapidQ's
///   behaviour, so it is a warning whose hint says AND / OR;
/// * `console.log("hi")` (and `System.out.println`, `Console.WriteLine`) —
///   it compiles as a call on a name nobody declared, and warns when it
///   runs; here a warning before that, its hint PRINT.
///
/// The program's own files only (a library's are its authors'); code only
/// (not strings, not comments). RapidR Studio's assistant treats both as
/// errors (check_code).
fn foreign_code(s: &Snapshot) -> Vec<FileDiagnostic> {
    let folder = s.parsed.root.parent().map(Path::to_path_buf).filter(|f| !f.as_os_str().is_empty());
    // (a program that declares its own `console` or `system` is not quoting JavaScript)
    let declared = |name: &str| s.model.symbols.iter().any(|sym| !sym.implicit && sym.name.eq_ignore_ascii_case(name));
    let mut out = Vec::new();
    for file in s.parsed.files() {
        if folder.as_ref().is_some_and(|f| !file.starts_with(f)) {
            continue;
        }
        let Some(text) = s.parsed.file_text(&file) else { continue };
        let mut offset = 0;
        for line in text.split_inclusive('\n') {
            let line_start = offset;
            offset += line.len();
            let code = crate::foreign::code_of(line);
            // (`x &&HFF` is `x & &HFF`: a hex number after a join)
            let mut from = 0;
            while let Some(i) = code[from..].find("&&") {
                let at = from + i;
                from = at + 2;
                let rest = &code[at + 2..];
                let number = rest.chars().next().is_some_and(|c| "hHoObB".contains(c)) && rest[1..].chars().next().is_some_and(|c| c.is_ascii_hexdigit());
                if number || rest.trim().is_empty() {
                    continue;
                }
                out.push(FileDiagnostic {
                    file: file.clone(),
                    start: line_start + at,
                    end: line_start + at + 2,
                    severity: Severity::Warning,
                    message: "&& is not a logical AND in RapidR (& joins text), so this test is never what was meant".into(),
                    code: Some(FOREIGN_AND.into()),
                    hint: crate::foreign::hint("and-and"),
                });
            }
            let trimmed = code.trim_start();
            let lower = trimmed.to_ascii_lowercase();
            for (prefix, owner) in FOREIGN_PRINTS {
                if lower.starts_with(prefix) && !declared(owner) {
                    let start = line_start + (code.len() - trimmed.len());
                    out.push(FileDiagnostic {
                        file: file.clone(),
                        start,
                        end: start + prefix.len() - 1,
                        severity: Severity::Warning,
                        message: format!("{} is not RapidR: nothing is printed", &trimmed[..prefix.len() - 1]),
                        code: Some(FOREIGN_PRINT.into()),
                        hint: crate::foreign::hint("console-log"),
                    });
                    break;
                }
            }
        }
    }
    out
}

/// `&&` used as a logical AND.
pub(crate) const FOREIGN_AND: &str = "foreign-and";
/// `console.log(…)` and friends.
pub(crate) const FOREIGN_PRINT: &str = "foreign-print";

/// How other languages print a line (with the `(` that follows), and the
/// name a program would have to declare for it to be its own.
const FOREIGN_PRINTS: &[(&str, &str)] = &[
    ("console.log(", "console"),
    ("console.info(", "console"),
    ("console.warn(", "console"),
    ("console.error(", "console"),
    ("console.debug(", "console"),
    ("system.out.println(", "system"),
    ("system.out.print(", "system"),
    ("system.console.writeline(", "system"),
];

/// The hints for the program of `s` (every file of it).
pub(crate) fn check(s: &Snapshot) -> Vec<FileDiagnostic> {
    let m = &s.model;
    let mut out = Vec::new();
    // (the symbols something is stored in: the program's own variables)
    let written: std::collections::HashSet<usize> = m.references.iter().filter(|r| r.access == Access::Write).map(|r| r.symbol).collect();
    // (each implicit variable's uses, in one pass over the references)
    let mut uses_of: std::collections::HashMap<usize, Vec<&crate::model::Reference>> = std::collections::HashMap::new();
    for r in &m.references {
        let sym = &m.symbols[r.symbol];
        if sym.implicit && matches!(sym.kind, SymbolKind::Global | SymbolKind::Static | SymbolKind::Local) && !written.contains(&r.symbol) {
            uses_of.entry(r.symbol).or_default().push(r);
        }
    }
    let mut ids: Vec<usize> = uses_of.keys().copied().collect();
    ids.sort_unstable();
    for id in ids {
        let sym = &m.symbols[id];
        let uses = &uses_of[&id];
        if uses.iter().any(|r| r.access != Access::Read) {
            continue;
        }
        let base = rapidr_ast::strip_type_suffix(&sym.name);
        let Some((_, group)) = rapidr_lang::constant(base).filter(|(_, g)| g.origin == Origin::RapidQ) else { continue };
        // (built into a RapidR program: RAPIDQ.INC's own needs no include)
        if s.rapidr_program() && group.source.eq_ignore_ascii_case(RAPIDQ_INC) {
            continue;
        }
        // (the include's own spelling: `mbYes`)
        let name = group.constants.iter().find(|(n, _)| n.eq_ignore_ascii_case(base)).map_or(base, |(n, _)| n);
        let inc = if group.source.eq_ignore_ascii_case(RAPIDQ_INC) { RAPIDQ_INC } else { group.source };
        let (severity, code, message) = (Severity::Warning, NEEDS_INCLUDE, format!("{name} is a {inc} constant — add $INCLUDE \"{inc}\""));
        for r in uses.iter() {
            if let Some(l) = s.locate(r.span) {
                out.push(FileDiagnostic { file: l.file, start: l.start, end: l.end.max(l.start + 1), severity, message: message.clone(), code: Some(code.into()), hint: None });
            }
        }
    }
    out.extend(not_one(s));
    out.extend(hidden_result(s));
    out.extend(foreign_code(s));
    out.sort_by(|a, b| (&a.file, a.start).cmp(&(&b.file, b.start)));
    out
}

/// The edit putting `$INCLUDE "inc"` in a file: before its first
/// `$INCLUDE`, conditional, or line of code — after the comments, blank
/// lines and directives (`$APPTYPE`, `$TYPECHECK` …) at its top, where
/// RapidQ programs have it.
pub(crate) fn include_edit(text: &str, inc: &str) -> TextEdit {
    let mut at = 0;
    for line in text.split_inclusive('\n') {
        let t = line.trim();
        let upper = t.to_ascii_uppercase();
        let directive = t.starts_with('$') && !["$INCLUDE", "$IF", "$ELSE", "$END", "$DEFINE", "$UNDEF", "$MACRO"].iter().any(|d| upper.starts_with(d));
        let comment = t.starts_with('\'') || upper == "REM" || upper.starts_with("REM ");
        if !(t.is_empty() || comment || directive) {
            break;
        }
        at += line.len();
    }
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    // (a top that is all comments, its last line without an end)
    let before = if at == text.len() && !text.is_empty() && !text.ends_with('\n') { nl } else { "" };
    TextEdit { start: at, end: at, text: format!("{before}$INCLUDE \"{inc}\"{nl}") }
}

/// The fixes for hints in a range: the include added (to the program's
/// main file when the name is used there, else to the file using it — an
/// include of RAPIDQ.INC twice is harmless).
pub(crate) fn actions(diags: &[FileDiagnostic], root: &Path, text_of: impl Fn(&Path) -> Option<String>) -> Vec<CodeAction> {
    let mut out: Vec<CodeAction> = Vec::new();
    for d in diags {
        match d.code.as_deref() {
            Some(NEEDS_INCLUDE) => {
                let Some(inc) = d.message.rsplit_once("$INCLUDE \"").map(|(_, r)| r.trim_end_matches('"').to_string()) else { continue };
                let file = if d.file == root { root.to_path_buf() } else { d.file.clone() };
                let Some(text) = text_of(&file) else { continue };
                let title = crate::words::tr_ui_fill("Add $INCLUDE \"{inc}\"", &[("inc", &inc)]);
                if out.iter().any(|a| a.title == title) {
                    continue;
                }
                out.push(CodeAction { title, edit: vec![(file, vec![include_edit(&text, &inc)])], fixes: Some(d.clone()), preferred: true });
            }
            Some(NOT_ONE) => {
                let Some(text) = text_of(&d.file) else { continue };
                let Some(fix) = not_one_fix(&text, d.start, d.end) else { continue };
                out.push(CodeAction { title: crate::words::tr_ui_fill("Write {name}", &[("name", &fix.text)]), edit: vec![(d.file.clone(), vec![fix])], fixes: Some(d.clone()), preferred: true });
            }
            _ => {}
        }
    }
    out
}

/// `NOT FILEEXISTS(x)` (bytes `start..end` of `text`) as `FILEEXISTS(x) =
/// 0` — in brackets unless it is a whole condition (`IF … THEN`, `WHILE …`,
/// `UNTIL …`), where `=` can't join with what is around it.
fn not_one_fix(text: &str, start: usize, end: usize) -> Option<TextEdit> {
    let written = text.get(start..end)?;
    let call = written.trim_start().get(3..)?.trim_start();
    if !written.trim_start().get(..3)?.eq_ignore_ascii_case("NOT") || call.is_empty() {
        return None;
    }
    let line_start = text[..start].rfind('\n').map_or(0, |i| i + 1);
    let line_end = text[end..].find('\n').map_or(text.len(), |i| end + i);
    let before = text[line_start..start].trim_end().to_ascii_uppercase();
    let after = text[end..line_end].trim_start().to_ascii_uppercase();
    let opens = ["IF", "ELSEIF", "WHILE", "UNTIL"].iter().any(|k| before == *k || before.ends_with(&format!(" {k}")) || before.ends_with(&format!(":{k}")));
    let closes = after.is_empty() || after.starts_with("THEN") || after.starts_with('\'') || after.starts_with(':');
    let fixed = if opens && closes { format!("{call} = 0") } else { format!("({call} = 0)") };
    Some(TextEdit { start, end, text: fixed })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_include_goes_after_the_header_before_other_includes() {
        let at = |t: &str| {
            let e = include_edit(t, RAPIDQ_INC);
            format!("{}{}{}", &t[..e.start], e.text, &t[e.end..])
        };
        assert_eq!(at("PRINT 1\n"), "$INCLUDE \"RAPIDQ.INC\"\nPRINT 1\n");
        assert_eq!(at("' my app\n$APPTYPE GUI\n\n$INCLUDE \"x.inc\"\nPRINT 1\n"), "' my app\n$APPTYPE GUI\n\n$INCLUDE \"RAPIDQ.INC\"\n$INCLUDE \"x.inc\"\nPRINT 1\n");
        assert_eq!(at("' only a comment"), "' only a comment\n$INCLUDE \"RAPIDQ.INC\"\n");
        assert_eq!(at("DIM a\r\nPRINT a\r\n"), "$INCLUDE \"RAPIDQ.INC\"\r\nDIM a\r\nPRINT a\r\n");
    }
}
