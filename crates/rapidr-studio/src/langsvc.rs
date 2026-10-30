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
//! | `ProjectDiagnostics` → text | the open project's problems (`Project`: its main file), as Problems lists them: the program compiled whole (a module is never checked alone), grouped by file — the files with errors first — and in a file errors, warnings, notes; what works on one runtime only once per component, with how many uses (S5). The same fields as `Diagnostics` |
//! | `Project` | the open project's main file ("" none): a file it includes is analysed as part of it, here and in the code editors |
//! | `Targets` | where the project runs: `desktop` (the default), `web` or `both`; what one runtime only has is noted when it runs on the other |
//! | `ErrorCount` | errors in the last Diagnostics or ProjectDiagnostics |
//! | `CompatMode` | `"rapidq"`: RapidR's extensions reported (a RapidQ-compatible project) |
//!
//! The language service grows here (completion, hover, rename … for the
//! editor: I3's UI wiring).

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rapidr_langsvc::{Analysis, OutlineItem, OutlineKind, Options, Targets};
use rapidr_value::Value;

use crate::{int_arg, slashes, text_arg};

#[derive(Default)]
struct Service {
    analysis: Option<Analysis>,
    rapidq: bool,
    errors: i64,
    /// The open project's main file ("" none).
    project: String,
    targets: Targets,
}

impl Service {
    fn options(&self) -> Options {
        Options { rapidq_compatible: self.rapidq, project: (!self.project.is_empty()).then(|| PathBuf::from(&self.project)), targets: self.targets, ..Options::default() }
    }

    fn analysis(&mut self) -> &mut Analysis {
        let options = self.options();
        self.analysis.get_or_insert_with(|| Analysis::new(options))
    }

    /// The project and its targets told to the analysis and to the code
    /// editors' service (the same answers in both).
    fn project_changed(&mut self) {
        let options = self.options();
        if let Some(a) = self.analysis.as_mut() {
            let mut o = a.options().clone();
            o.project = options.project.clone();
            o.targets = options.targets;
            a.set_options(o);
        }
        let (main, targets) = (self.project.clone(), self.targets.name());
        rapidr_editor::service::with(|s| s.set_project(&main, targets));
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
            "compatmode" => Value::String(if s.rapidq { "rapidq".into() } else { String::new().into() }),
            "errorcount" => Value::Integer(s.errors),
            "project" => Value::String(s.project.clone().into()),
            "targets" => Value::String(s.targets.name().into()),
            // (the build's own: the workspace's version, as Help > About says it)
            "version" => Value::String(env!("CARGO_PKG_VERSION").into()),
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
        "errorcount" | "version" => true,
        "project" => {
            let p = slashes(&v.to_string_val());
            if p != s.project {
                s.project = p;
                s.project_changed();
            }
            true
        }
        "targets" => {
            let t = Targets::parse(&v.to_string_val());
            if t != s.targets {
                s.targets = t;
                s.project_changed();
            }
            true
        }
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

/// The byte offset of (`line`, `column`), both from 1 (columns in
/// characters; past the line's end: its end).
fn line_col_offset(text: &str, line: i64, column: i64) -> Option<usize> {
    if line < 1 {
        return None;
    }
    let mut start = 0;
    for (i, l) in text.split('\n').enumerate() {
        if i as i64 + 1 == line {
            let col = (column.max(1) - 1) as usize;
            return Some(start + l.char_indices().nth(col).map_or(l.trim_end_matches('\r').len(), |(b, _)| b));
        }
        start += l.len() + 1;
    }
    None
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
    // (the program's language — RapidR Studio's —: the registry's docs in
    // hovers and help, the compiler's messages in Problems)
    let lang = rapidr_value::i18n::language();
    if rapidr_lang::docs_i18n::doc_language() != lang {
        rapidr_lang::docs_i18n::set_doc_language(&lang);
    }
    Some(match method {
        "update" => {
            let text = text_arg(args, 1);
            let included = stored_includes(&file, &text);
            with(name, |s| {
                let a = s.analysis();
                a.update(file, text);
                // (the files it includes that only the runtime's store has —
                // the web's: the analysis reads the disk itself)
                for (path, t) in included {
                    if a.text(&path).is_none() {
                        a.update(path, t);
                    }
                }
            });
            Value::Null
        }
        "close" => {
            with(name, |s| s.analysis().close(&file));
            Value::Null
        }
        // (the Help pane: Help(Word, [OfType]) — crate::help)
        "help" => Value::String(crate::help::text(&text_arg(args, 0), &text_arg(args, 1)).into()),
        // (HelpPage(Word, [OfType], [File], [Line], [Column], [CanInsert]):
        // the pane's page — the program's own name at the caret first)
        "helppage" => {
            let (word, of) = (text_arg(args, 0), text_arg(args, 1));
            let file = PathBuf::from(slashes(&text_arg(args, 2)));
            let (line, col) = (int_arg(args, 3, 0), int_arg(args, 4, 0));
            let mut ask = crate::help::Ask { word, of, can_insert: int_arg(args, 5, 0) != 0, ..Default::default() };
            if !file.as_os_str().is_empty() {
                with(name, |s| {
                    let a = s.analysis();
                    ask.program = a.program_names(&file);
                    let text = a.text(&file).unwrap_or_default();
                    match line_col_offset(&text, line, col) {
                        Some(at) if line > 0 => {
                            // (a member of something: the component it is of)
                            if let Some(owner) = a.member_owner(&file, at) {
                                ask.of = owner;
                            }
                            if ask.of.is_empty() {
                                ask.mine = a.symbol_help(&file, at).filter(|m| ask.word.is_empty() || m.name.eq_ignore_ascii_case(ask.word.trim()));
                            }
                        }
                        _ => {
                            // (typed in the search box: the program's name of that name)
                            if !ask.word.trim().is_empty() && ask.of.is_empty() {
                                ask.mine = a.symbol_help_named(&file, ask.word.trim());
                            }
                        }
                    }
                });
            }
            Value::String(crate::help::page(&ask).into())
        }
        // (HelpFind(Query, [File]): what Query may mean, a line each: name,
        // what it is — the program's own names too)
        "helpfind" => {
            let file = PathBuf::from(slashes(&text_arg(args, 1)));
            let program = if file.as_os_str().is_empty() { Vec::new() } else { with(name, |s| s.analysis().program_names(&file)) };
            let mut out = String::new();
            for (n, what) in crate::help::suggest(&text_arg(args, 0), &program, 12) {
                out.push_str(&format!("{n}\t{what}\n"));
            }
            Value::String(out.into())
        }
        "outline" => Value::String(with(name, |s| {
            let a = s.analysis();
            let text = a.text(&file).unwrap_or_default();
            let mut out = String::new();
            outline_lines(&a.outline(&file), &text, 0, &mut out);
            out
        }).into()),
        // ManualPage(Page): a page of the manual in Studio's language
        // (docs/manual/<language>/<page>.md), else in English with a note
        // that says so; "" for no such page
        "manualpage" => {
            let page = text_arg(args, 0);
            Value::String(match crate::manual_search::page_in(&page, &lang) {
                Some((text, true)) => text.to_string().into(),
                Some((text, false)) if !lang.starts_with("en") => {
                    let note = rapidr_value::i18n::tr_ui("Not translated yet: this page is in English.");
                    format!("> *{note}*\n\n{text}").into()
                }
                Some((text, false)) => text.to_string().into(),
                None => String::new().into(),
            })
        }
        "diagnostics" | "projectdiagnostics" => Value::String(with(name, |s| {
            let project = (method == "projectdiagnostics").then(|| PathBuf::from(&s.project)).filter(|p| !p.as_os_str().is_empty());
            let a = s.analysis();
            let mut out = String::new();
            let mut texts: HashMap<PathBuf, String> = HashMap::new();
            let mut errors = 0;
            let found = match &project {
                Some(main) => a.project_diagnostics(main),
                None if method == "projectdiagnostics" => Vec::new(),
                None => {
                    // (as the project's are: errors first, grouped by file)
                    let mut d = a.diagnostics(&file);
                    rapidr_langsvc::sort_problems(&mut d, &file);
                    d
                }
            };
            for d in found {
                let text = texts.entry(d.file.clone()).or_insert_with(|| a.text(&d.file).unwrap_or_default());
                let (line, col) = line_col(text, d.start);
                let severity = format!("{:?}", d.severity).to_ascii_lowercase();
                if severity == "error" {
                    errors += 1;
                }
                // (the compiler's words in Studio's language: RQ… / RR… messages
                // a catalog translates; the rest as the compiler says them)
                let message = rapidr_diagnostics::messages::localize(&d.message, &lang);
                out.push_str(&format!("{severity}\t{line}\t{col}\t{}\t{}\n", clean(&message), slashes(&d.file.to_string_lossy())));
            }
            s.errors = errors;
            out
        }).into()),
        _ => return None,
    })
}

/// The files `text` (of `file`) includes, and theirs, that aren't on a disk
/// the analysis can read but are in the runtime's store (the web page's):
/// their paths beside `file` and their text.
fn stored_includes(file: &Path, text: &str) -> Vec<(PathBuf, String)> {
    let mut out: Vec<(PathBuf, String)> = Vec::new();
    let mut todo: Vec<(PathBuf, String)> = vec![(file.to_path_buf(), text.to_string())];
    while let Some((from, src)) = todo.pop() {
        let folder = crate::folder_of(&slashes(&from.to_string_lossy()));
        for line in src.lines() {
            let t = line.trim_start();
            if !t.get(..8).is_some_and(|h| h.eq_ignore_ascii_case("$include")) {
                continue;
            }
            let Some(name) = t[8..].split('"').nth(1).or_else(|| t[8..].split(['<', '>']).nth(1)) else { continue };
            let path = PathBuf::from(crate::join(&folder, name));
            if out.len() >= 64 || path.exists() || out.iter().any(|(p, _)| *p == path) {
                continue;
            }
            if let Ok(t) = crate::read_text(&path.to_string_lossy()) {
                out.push((path.clone(), t.clone()));
                todo.push((path, t));
            }
        }
    }
    out
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

    /// The Help pane's page: the caret on one of the program's own names,
    /// on a member after a dot, on a word of the language, and a typed name.
    #[test]
    fn the_help_page_knows_the_programs_own_names() {
        let src = "DECLARE SUB Greet(Who AS STRING)\nDIM PickedFont AS RFont\nCREATE Form AS RForm\n  CREATE Hello AS RButton\n  END CREATE\nEND CREATE\nSUB Go\n  PickedFont.Name = \"Inter\"\n  Hello.Font.Color = &H800000\n  Greet \"x\"\n  PRINT 1\nEND SUB\nSUB Greet(Who AS STRING)\nEND SUB\n";
        let f = Value::String("/w/helpdemo.rr".into());
        let call = |m: &str, args: &[Value]| call("ls-help", m, args).unwrap().to_string_val();
        call("update", &[f.clone(), Value::String(src.into())]);
        let ask = |word: &str, line: i64, col: i64| call("helppage", &[Value::String(word.into()), Value::String(String::new().into()), f.clone(), Value::Integer(line), Value::Integer(col), Value::Integer(1)]);
        // (the caret just after PickedFont, on its use: line 8)
        let t = ask("PickedFont", 8, 13);
        let (meta, md) = t.split_once('\n').unwrap();
        assert_eq!(meta, "mine\t\t/w/helpdemo.rr\t2\t5", "{t}");
        assert!(md.starts_with("## PickedFont\n\nA variable in this program"), "{t}");
        assert!(md.contains("DIM PickedFont AS RFont") && md.contains("Declared in [helpdemo.rr, line 2](help-goto:)"), "{t}");
        assert!(md.contains("Its type is [RFont](help:RFont).") && md.contains("#### Properties ("), "{t}");
        // (a component of the form: its type's help below)
        let t = ask("Hello", 9, 8);
        assert!(t.starts_with("mine\t\t/w/helpdemo.rr\t4\t10\n## Hello\n") && t.contains("Its type is [RButton](help:RButton)."), "{t}");
        // (a SUB: its signature; the references link)
        let t = ask("Greet", 10, 8);
        assert!(t.contains("## Greet\n\nA SUB in this program") && t.contains("SUB Greet(Who AS STRING)") && t.contains("help-refs:"), "{t}");
        // (Color after Font.: RFont's property, not "nothing called Color")
        let t = ask("Color", 9, 17);
        assert!(t.starts_with("entry\t") && t.contains("## RFont.Color"), "{t}");
        assert!(t.contains("A member of [RFont](help:RFont)."), "{t}");
        // (a word of the language at the caret: the registry's entry, Insert offered)
        let t = ask("PRINT", 11, 8);
        assert!(t.starts_with("entry\tPRINT") && t.contains("[Insert into the code](help-insert:)"), "{t}");
        // (typed, no caret: the program's name of that name, else the language's)
        let t = ask("picked", 0, 0);
        assert!(t.starts_with("none\t"), "{t}");
        let t = ask("PickedFont", 0, 0);
        assert!(t.starts_with("mine\t"), "{t}");
        let t = ask("RLabel", 0, 0);
        assert!(t.starts_with("entry\t"), "{t}");
        // (the suggestions: the program's names among the language's)
        let found = call("helpfind", &[Value::String("pick".into()), f.clone()]);
        assert!(found.starts_with("PickedFont\tyour variable\n"), "{found}");
        // (the build's version: the workspace's)
        assert_eq!(get("ls-help", "version").unwrap().to_string_val(), env!("CARGO_PKG_VERSION"));
        assert_eq!(line_col_offset("ab\ncé d", 2, 3), Some(6));
    }

    /// Help > About says the version of the build: the workspace's.
    #[test]
    fn the_version_is_the_workspaces() {
        let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.toml")).unwrap();
        let version = manifest.lines().skip_while(|l| l.trim() != "[workspace.package]").find_map(|l| l.trim().strip_prefix("version = ")).unwrap().trim_matches('"').to_string();
        assert_eq!(get("ls-version", "version").unwrap().to_string_val(), version);
    }
}
