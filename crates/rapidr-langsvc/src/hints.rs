//! What the compiler doesn't say, because RapidQ compiles it and it runs:
//! a constant of RAPIDQ.INC (or of another RapidQ include) used without the
//! `$INCLUDE` is RapidQ's implicit variable, always 0 (`mbYes` → 0: a
//! MessageDlg with only an OK button). RapidR keeps that meaning; this is
//! a warning beside the squiggles and in Problems, its fix adding the
//! include. It reads the compiler's semantic model: its implicit symbols
//! and how each use reads or writes them (a name the program stores in is
//! its own variable).

use std::path::Path;

use rapidr_diagnostics::Severity;
use rapidr_lang::Origin;

use crate::model::{Access, SymbolKind};
use crate::{CodeAction, FileDiagnostic, Snapshot, TextEdit};

pub(crate) const RAPIDQ_INC: &str = "RAPIDQ.INC";
/// A RapidQ include's constant used without the include.
pub(crate) const NEEDS_INCLUDE: &str = "needs-include";

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
        // (the include's own spelling: `mbYes`)
        let name = group.constants.iter().find(|(n, _)| n.eq_ignore_ascii_case(base)).map_or(base, |(n, _)| n);
        let inc = if group.source.eq_ignore_ascii_case(RAPIDQ_INC) { RAPIDQ_INC } else { group.source };
        let (severity, code, message) = (Severity::Warning, NEEDS_INCLUDE, format!("{name} is a {inc} constant — add $INCLUDE \"{inc}\""));
        for r in uses.iter() {
            if let Some(l) = s.locate(r.span) {
                out.push(FileDiagnostic { file: l.file, start: l.start, end: l.end.max(l.start + 1), severity, message: message.clone(), code: Some(code.into()) });
            }
        }
    }
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
                let title = format!("Add $INCLUDE \"{inc}\"");
                if out.iter().any(|a| a.title == title) {
                    continue;
                }
                out.push(CodeAction { title, edit: vec![(file, vec![include_edit(&text, &inc)])], fixes: Some(d.clone()), preferred: true });
            }
            _ => {}
        }
    }
    out
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
