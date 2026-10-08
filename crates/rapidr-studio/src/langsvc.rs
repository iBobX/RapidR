//! RLANGUAGESERVICE: RapidR's language service (`rapidr-langsvc`, the one
//! `rapidr lsp` and the VS Code extension use) as a component — what
//! RapidR Studio's Outline and Problems panes show, and what a program can
//! ask about BASIC source (docs/ide-components.md §3.9). Answers are text,
//! one line per item, fields separated by tabs (CHR$(9)), so a RapidQ-style
//! program reads them with FIELD$.
//!
//! | Member | |
//! |---|---|
//! | `Update(File, Text)` | the editor's text of a file (unsaved) |
//! | `Close(File)` | the file is read from its store again |
//! | `Outline(File)` → text | `depth, kind, name, detail, line` — kind: `sub`, `function`, `type`, `field`, `method`, `event`, `component`, `constant`, `variable`, `label`; line from 1 |
//! | `Diagnostics(File)` → text | `severity, line, column, message, file` — severity: `error`, `warning`, `note`; line and column from 1; the compiler's own messages |
//! | `ErrorCount` | errors in the last Diagnostics |
//! | `CompatMode` | `"rapidq"`: RapidR's extensions reported (a RapidQ-compatible project) |
//!
//! The language service grows here (completion, hover, rename … for the
//! editor: I3's UI wiring).

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rapidr_langsvc::{Analysis, OutlineItem, OutlineKind, Options};
use rapidr_value::Value;

use crate::{slashes, text_arg};

#[derive(Default)]
struct Service {
    analysis: Option<Analysis>,
    rapidq: bool,
    errors: i64,
}

impl Service {
    fn analysis(&mut self) -> &mut Analysis {
        let rapidq = self.rapidq;
        self.analysis.get_or_insert_with(|| Analysis::new(Options { rapidq_compatible: rapidq, ..Options::default() }))
    }
}

thread_local! {
    static SERVICES: RefCell<HashMap<String, Service>> = RefCell::new(HashMap::new());
}

fn with<R>(name: &str, f: impl FnOnce(&mut Service) -> R) -> R {
    SERVICES.with(|s| f(s.borrow_mut().entry(name.to_ascii_lowercase()).or_default()))
}

pub fn get(name: &str, prop: &str) -> Option<Value> {
    with(name, |s| {
        Some(match prop {
            "compatmode" => Value::String(if s.rapidq { "rapidq".into() } else { String::new() }),
            "errorcount" => Value::Integer(s.errors),
            _ => return None,
        })
    })
}

pub fn set(name: &str, prop: &str, v: &Value) -> bool {
    with(name, |s| match prop {
        "compatmode" => {
            s.rapidq = v.to_string_val().eq_ignore_ascii_case("rapidq");
            if let Some(a) = s.analysis.as_mut() {
                let mut o = a.options().clone();
                o.rapidq_compatible = s.rapidq;
                a.set_options(o);
            }
            true
        }
        "errorcount" => true,
        _ => false,
    })
}

fn kind_name(k: OutlineKind) -> &'static str {
    match k {
        OutlineKind::Sub => "sub",
        OutlineKind::Function => "function",
        OutlineKind::Type => "type",
        OutlineKind::Field => "field",
        OutlineKind::Method => "method",
        OutlineKind::Event => "event",
        OutlineKind::Component => "component",
        OutlineKind::Constant => "constant",
        OutlineKind::Variable => "variable",
        OutlineKind::Label => "label",
    }
}

/// (line, column) from 1 of byte `offset` in `text`, columns in characters.
pub fn line_col(text: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(text.len());
    let before = text.get(..offset).unwrap_or(text);
    let line = before.matches('\n').count() + 1;
    let start = before.rfind('\n').map_or(0, |i| i + 1);
    (line, before[start..].chars().count() + 1)
}

fn clean(s: &str) -> String {
    s.replace(['\t', '\r', '\n'], " ")
}

fn outline_lines(items: &[OutlineItem], text: &str, depth: usize, out: &mut String) {
    for item in items {
        let (line, _) = line_col(text, item.name_start);
        out.push_str(&format!("{depth}\t{}\t{}\t{}\t{line}\n", kind_name(item.kind), clean(&item.name), clean(item.detail.as_deref().unwrap_or(""))));
        outline_lines(&item.children, text, depth + 1, out);
    }
}

pub fn call(name: &str, method: &str, args: &[Value]) -> Option<Value> {
    let file = PathBuf::from(slashes(&text_arg(args, 0)));
    Some(match method {
        "update" => {
            with(name, |s| s.analysis().update(file, text_arg(args, 1)));
            Value::Null
        }
        "close" => {
            with(name, |s| s.analysis().close(&file));
            Value::Null
        }
        "outline" => Value::String(with(name, |s| {
            let a = s.analysis();
            let text = a.text(&file).unwrap_or_default();
            let mut out = String::new();
            outline_lines(&a.outline(&file), &text, 0, &mut out);
            out
        })),
        "diagnostics" => Value::String(with(name, |s| {
            let a = s.analysis();
            let mut out = String::new();
            let mut texts: HashMap<PathBuf, String> = HashMap::new();
            let mut errors = 0;
            for d in a.diagnostics(&file) {
                let text = texts.entry(d.file.clone()).or_insert_with(|| a.text(&d.file).unwrap_or_default());
                let (line, col) = line_col(text, d.start);
                let severity = format!("{:?}", d.severity).to_ascii_lowercase();
                if severity == "error" {
                    errors += 1;
                }
                out.push_str(&format!("{severity}\t{line}\t{col}\t{}\t{}\n", clean(&d.message), slashes(&d.file.to_string_lossy())));
            }
            s.errors = errors;
            out
        })),
        _ => return None,
    })
}

/// (for the tests: a file's outline as the component gives it)
#[doc(hidden)]
pub fn outline_of(path: &Path, text: &str) -> String {
    let mut a = Analysis::new(Options::default());
    a.update(path.to_path_buf(), text.to_string());
    let mut out = String::new();
    outline_lines(&a.outline(path), text, 0, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outline_and_diagnostics() {
        let src = "DIM n AS INTEGER\nCREATE Form AS QFORM\n  CREATE B AS QBUTTON\n  END CREATE\nEND CREATE\nSUB Go\nEND SUB\nFUNCTION Twice(x AS INTEGER) AS INTEGER\n  Result = x * 2\nEND FUNCTION\n";
        let out = outline_of(Path::new("/w/a.rr"), src);
        assert!(out.contains("component\tForm"), "{out}");
        assert!(out.contains("1\tcomponent\tB"), "{out}");
        assert!(out.contains("sub\tGo\t") && out.lines().any(|l| l.starts_with("0\tsub\tGo") && l.ends_with("\t6")), "{out}");
        let call = |m: &str, args: &[Value]| call("ls1", m, args).unwrap();
        call("update", &[Value::String("/w/b.rr".into()), Value::String("PRINT 1\nNope x\n".into())]);
        let d = call("diagnostics", &[Value::String("/w/b.rr".into())]).to_string_val();
        assert!(d.starts_with("error\t2\t"), "{d}");
        assert_eq!(get("ls1", "errorcount").unwrap().to_i64(), 1);
        call("update", &[Value::String("/w/b.rr".into()), Value::String("PRINT 1\n".into())]);
        assert_eq!(call("diagnostics", &[Value::String("/w/b.rr".into())]).to_string_val(), "");
        assert_eq!(line_col("ab\ncé d", 7), (2, 4));
    }
}
