//! The language service's golden suite: tests/langsvc/ (docs/ide-plan.md,
//! I3 acceptance). Each case is a folder of source files; `|` marks places
//! (anywhere but on `'!` lines, numbered in order: main.bas / main.rr first, then the
//! other files by name), and `'!` comment lines say what to expect there:
//!
//! ```text
//! '! 1 completion has Caption Show OnClick
//! '! 1 completion lacks Form
//! '! 2 hover has "property of QForm"
//! '! 2 hover none
//! '! 3 definition util.inc:3:5
//! '! 3 definition none
//! '! 4 references main.bas:2:5 util.inc:4:3      (every place, declaration included)
//! '! 5 signature "SUB Greet(who AS STRING, n AS INTEGER)" active 1
//! '! 6 rename Total main.bas:1:5 main.bas:4:1
//! '! 6 rename b error "already the name"
//! '! diagnostic main.bas:3:1 "Unknown SUB or FUNCTION 'Nope'"
//! '! diagnostics none
//! '! outline Form Btn Greet          (the main file's outline, flattened)
//! '! options rapidq-compatible       (the case is a RapidQ-compatible project)
//! ```
//!
//! Positions are `file:line:column`, 1-based, columns in bytes.
//! `GOLDEN=name` runs one case.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use rapidr_langsvc::{Analysis, LineIndex, Location, Options};

struct Marker {
    file: PathBuf,
    offset: usize,
}

fn cases_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/langsvc")
}

/// Removes the markers (every `|` but those on `'!` lines) of a text.
fn strip_markers(text: &str) -> (String, Vec<usize>) {
    let mut out = String::with_capacity(text.len());
    let mut marks = Vec::new();
    for line in text.split_inclusive('\n') {
        if line.trim_start().starts_with("'!") {
            out.push_str(line);
            continue;
        }
        for c in line.chars() {
            if c == '|' {
                marks.push(out.len());
                continue;
            }
            out.push(c);
        }
    }
    (out, marks)
}

fn position(files: &BTreeMap<PathBuf, String>, loc: &Location, root: &Path) -> String {
    let text = files.get(&loc.file).cloned().unwrap_or_default();
    let index = LineIndex::new(&text);
    let (line, col) = index.line_col(loc.start);
    let name = loc.file.strip_prefix(root).unwrap_or(&loc.file).display().to_string();
    format!("{name}:{}:{}", line + 1, col + 1)
}

/// Splits `a "b c" d` into words, quoted parts whole.
fn words(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for c in s.chars() {
        match c {
            '"' => {
                if quoted {
                    out.push(std::mem::take(&mut cur));
                }
                quoted = !quoted;
            }
            c if c.is_whitespace() && !quoted => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn run_case(dir: &Path) -> Vec<String> {
    let name = dir.file_name().unwrap().to_string_lossy().to_string();
    let work = std::env::temp_dir().join(format!("rapidr-langsvc-golden-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&work).unwrap();
    let mut entries: Vec<PathBuf> = fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.is_file()).collect();
    entries.sort_by_key(|p| {
        let n = p.file_name().unwrap().to_string_lossy().to_ascii_lowercase();
        (!n.starts_with("main."), n)
    });
    let mut files = BTreeMap::new();
    let mut markers = Vec::new();
    let mut expectations = Vec::new();
    let mut main = None;
    for p in &entries {
        let text = fs::read_to_string(p).unwrap();
        let (clean, marks) = strip_markers(&text);
        let target = work.join(p.file_name().unwrap());
        fs::write(&target, &clean).unwrap();
        for m in marks {
            markers.push(Marker { file: target.clone(), offset: m });
        }
        for line in clean.lines() {
            if let Some(e) = line.trim_start().strip_prefix("'!") {
                expectations.push(e.trim().to_string());
            }
        }
        if main.is_none() {
            main = Some(target.clone());
        }
        files.insert(target, clean);
    }
    let main = main.expect("a case has files");
    let rapidq_compatible = expectations.iter().any(|e| e == "options rapidq-compatible");
    let mut analysis = Analysis::new(Options { rapidq_compatible, ..Default::default() });
    for (p, t) in &files {
        analysis.update(p.clone(), t.clone());
    }
    let mut failures = Vec::new();
    for e in &expectations {
        let w = words(e);
        let fail = |msg: String| format!("{name}: '! {e}\n      {msg}");
        if w.first().map(String::as_str) == Some("options") {
            continue;
        }
        if w.first().map(String::as_str) == Some("outline") {
            fn flat(items: &[rapidr_langsvc::OutlineItem], out: &mut Vec<String>) {
                for i in items {
                    out.push(i.name.clone());
                    flat(&i.children, out);
                }
            }
            let mut got = Vec::new();
            flat(&analysis.outline(&main), &mut got);
            if got != w[1..] {
                failures.push(fail(format!("got {got:?}")));
            }
            continue;
        }
        if w.first().map(String::as_str) == Some("diagnostic") || w.first().map(String::as_str) == Some("diagnostics") {
            let diags = analysis.diagnostics(&main);
            let got: Vec<String> = diags
                .iter()
                .map(|d| format!("{} {}", position(&files, &Location { file: d.file.clone(), start: d.start, end: d.end }, &work), d.message))
                .collect();
            if w[0] == "diagnostics" && w.get(1).map(String::as_str) == Some("none") {
                if !got.is_empty() {
                    failures.push(fail(format!("got {got:?}")));
                }
            } else {
                let want = format!("{} {}", w.get(1).cloned().unwrap_or_default(), w.get(2).cloned().unwrap_or_default());
                if !got.contains(&want) {
                    failures.push(fail(format!("got {got:?}")));
                }
            }
            continue;
        }
        let Some(idx) = w.first().and_then(|n| n.parse::<usize>().ok()) else {
            failures.push(fail("expectations start with a marker number".into()));
            continue;
        };
        let Some(m) = markers.get(idx - 1) else {
            failures.push(fail(format!("no marker {idx}")));
            continue;
        };
        let what = w.get(1).map(String::as_str).unwrap_or("");
        let args = &w[2.min(w.len())..];
        match what {
            "completion" => {
                let got = analysis.completions(&m.file, m.offset);
                let labels: Vec<String> = got.items.iter().map(|c| c.label.to_ascii_lowercase()).collect();
                let mode = args.first().map(String::as_str).unwrap_or("");
                for want in &args[1.min(args.len())..] {
                    let has = labels.contains(&want.to_ascii_lowercase());
                    if mode == "has" && !has || mode == "lacks" && has {
                        let shown: Vec<&str> = got.items.iter().take(40).map(|c| c.label.as_str()).collect();
                        failures.push(fail(format!("{mode} {want}: got {} items: {shown:?}…", got.items.len())));
                    }
                }
            }
            "hover" => {
                let got = analysis.hover(&m.file, m.offset).map(|h| h.markdown);
                match args.first().map(String::as_str) {
                    Some("none") if got.is_some() => failures.push(fail(format!("got {got:?}"))),
                    Some("has") => {
                        for want in &args[1..] {
                            if !got.as_deref().unwrap_or("").contains(want.as_str()) {
                                failures.push(fail(format!("got {got:?}")));
                            }
                        }
                    }
                    _ => {}
                }
            }
            "definition" => {
                let got: Vec<String> = analysis.definition(&m.file, m.offset).iter().map(|l| position(&files, l, &work)).collect();
                let want: Vec<String> = if args.first().map(String::as_str) == Some("none") { Vec::new() } else { args.to_vec() };
                if got != want {
                    failures.push(fail(format!("got {got:?}")));
                }
            }
            "references" => {
                let mut got: Vec<String> = analysis.references(&m.file, m.offset, true).iter().map(|l| position(&files, l, &work)).collect();
                got.sort();
                let mut want = args.to_vec();
                want.sort();
                if got != want {
                    failures.push(fail(format!("got {got:?}")));
                }
            }
            "signature" => {
                let got = analysis.signature(&m.file, m.offset);
                let label = got.as_ref().and_then(|s| s.signatures.first()).map(|s| s.label.clone());
                let active = got.as_ref().map(|s| s.active_param);
                let want_label = args.first().cloned();
                let want_active = args.iter().position(|a| a == "active").and_then(|i| args.get(i + 1)).and_then(|n| n.parse::<usize>().ok());
                if want_label.as_deref() == Some("none") {
                    if got.is_some() {
                        failures.push(fail(format!("got {label:?}")));
                    }
                } else if label != want_label || want_active.is_some() && active != want_active {
                    failures.push(fail(format!("got {label:?} active {active:?}")));
                }
            }
            "rename" => {
                let new_name = args.first().cloned().unwrap_or_default();
                let got = analysis.rename(&m.file, m.offset, &new_name);
                if args.get(1).map(String::as_str) == Some("error") {
                    match got {
                        Ok(edits) => failures.push(fail(format!("renamed: {edits:?}"))),
                        Err(msg) => {
                            if let Some(want) = args.get(2) {
                                if !msg.contains(want.as_str()) {
                                    failures.push(fail(format!("error {msg:?}")));
                                }
                            }
                        }
                    }
                } else {
                    match got {
                        Err(msg) => failures.push(fail(format!("error {msg:?}"))),
                        Ok(edits) => {
                            let mut got: Vec<String> = edits
                                .iter()
                                .flat_map(|(f, es)| es.iter().map(move |e| Location { file: f.clone(), start: e.start, end: e.end }))
                                .map(|l| position(&files, &l, &work))
                                .collect();
                            got.sort();
                            let mut want = args[1..].to_vec();
                            want.sort();
                            if got != want {
                                failures.push(fail(format!("got {got:?}")));
                            }
                        }
                    }
                }
            }
            other => failures.push(fail(format!("unknown expectation '{other}'"))),
        }
    }
    let _ = fs::remove_dir_all(&work);
    if expectations.is_empty() {
        failures.push(format!("{name}: no expectations"));
    }
    failures
}

#[test]
fn golden_cases() {
    let only = std::env::var("GOLDEN").ok();
    let mut dirs: Vec<PathBuf> = fs::read_dir(cases_dir()).unwrap().map(|e| e.unwrap().path()).filter(|p| p.is_dir()).collect();
    dirs.sort();
    let mut failures = Vec::new();
    let mut ran = 0;
    for d in dirs {
        if only.as_deref().is_some_and(|o| !d.ends_with(o)) {
            continue;
        }
        ran += 1;
        failures.extend(run_case(&d));
    }
    assert!(ran > 0, "no cases ran");
    assert!(failures.is_empty(), "{} failure(s) in {ran} cases:\n{}", failures.len(), failures.join("\n"));
}
