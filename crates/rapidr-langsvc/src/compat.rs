//! RapidQ compatibility (docs/q-and-r-components.md §5): in a
//! RapidQ-compatible project, RapidR's own components are reported, and so
//! are Q-prefixed names RapidQ doesn't have (`QPLOT`: RapidR accepts it as
//! RPLOT), with a fix.
//!
//! (Members, builtins, statements and directives join once the registry
//! records each one's origin: L-REG.)

use std::path::Path;

use rapidr_ast::Statement;
use rapidr_diagnostics::{Severity, TextSpan};

use crate::context::pretty_component;
use crate::registry::{self, Origin};
use crate::text::is_name_char;
use crate::{CodeAction, FileDiagnostic, Snapshot, TextEdit};

pub(crate) const RAPIDR_ONLY: &str = "rapidr-only";
pub(crate) const NOT_RAPIDQ_NAME: &str = "not-a-rapidq-name";

/// RapidQ's other names for components (its manual and RAPIDQ2.INC).
const RAPIDQ_ALIASES: &[&str] = &["QGAUGE", "QOUTLINE", "COMPORT", "QCOMPORT"];

pub(crate) fn check(s: &Snapshot, file: &Path) -> Vec<FileDiagnostic> {
    let mut out = Vec::new();
    let mut spans = Vec::new();
    collect(&s.parsed.program.statements, &mut spans);
    for span in spans {
        let Some(loc) = s.locate_range(span) else { continue };
        if loc.file != file {
            continue;
        }
        let Some(text) = s.parsed.file_text(&loc.file) else { continue };
        // The type as written: the word after the first line's last AS.
        let line = text[loc.start..loc.end.min(text.len())].lines().next().unwrap_or("");
        let upper = line.to_ascii_uppercase();
        let Some(as_at) = upper.rfind(" AS ") else { continue };
        let rest = &line[as_at + 4..];
        let lead = rest.len() - rest.trim_start().len();
        let len = rest.trim_start().find(|c: char| !is_name_char(c)).unwrap_or(rest.trim_start().len());
        let start = loc.start + as_at + 4 + lead;
        let written = text[start..start + len].to_ascii_uppercase();
        let Some(c) = registry::component(&written) else { continue };
        let end = start + len;
        let is_rapidq_name = c.rapidq.is_some_and(|q| q.eq_ignore_ascii_case(&written)) || RAPIDQ_ALIASES.contains(&written.as_str());
        if written.starts_with('Q') && !is_rapidq_name {
            out.push(FileDiagnostic {
                file: loc.file.clone(),
                start,
                end,
                severity: Severity::Warning,
                message: format!("RapidQ has no {written}: RapidR reads it as {}", pretty_component(c.name)),
                code: Some(NOT_RAPIDQ_NAME.into()),
            });
        } else if c.origin == Origin::RapidR {
            out.push(FileDiagnostic {
                file: loc.file.clone(),
                start,
                end,
                severity: Severity::Warning,
                message: format!("{} is RapidR's own component: RapidQ doesn't have it (this project is RapidQ-compatible)", pretty_component(c.name)),
                code: Some(RAPIDR_ONLY.into()),
            });
        }
    }
    out
}

/// The DIM and CREATE statements of a program (their types are checked).
fn collect(statements: &[Statement], out: &mut Vec<TextSpan>) {
    for st in statements {
        match st {
            Statement::Dim(d) if !d.type_name.is_empty() => out.push(d.span),
            Statement::Create(c) => {
                out.push(c.span);
                collect(&c.body, out);
            }
            Statement::Subroutine(s) => collect(&s.body, out),
            Statement::Function(f) => collect(&f.body, out),
            Statement::If(i) => {
                collect(&i.then_body, out);
                collect(&i.else_body, out);
                for b in &i.elseif_branches {
                    collect(&b.body, out);
                }
            }
            Statement::For(f) => collect(&f.body, out),
            Statement::While(w) => collect(&w.body, out),
            Statement::DoLoop(d) => collect(&d.body, out),
            _ => {}
        }
    }
}

/// Fixes for compatibility diagnostics.
pub(crate) fn actions(diags: &[FileDiagnostic]) -> Vec<CodeAction> {
    let mut out = Vec::new();
    for d in diags {
        if d.code.as_deref() == Some(NOT_RAPIDQ_NAME) {
            if let Some(name) = d.message.rsplit(' ').next() {
                out.push(CodeAction {
                    title: format!("Write {name}"),
                    edit: vec![(d.file.clone(), vec![TextEdit { start: d.start, end: d.end, text: name.to_string() }])],
                    fixes: Some(d.clone()),
                    preferred: true,
                });
            }
        }
    }
    out
}
