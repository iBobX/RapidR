//! Diagnostics equal the compiler's (docs/ide-plan.md, I3 acceptance): for
//! every conformance error case (`tests/conformance/cases/*.expected-error`,
//! the compiler's own contract), the service reports each expected message
//! at the expected line and column.

use std::fs;
use std::path::Path;

use rapidr_langsvc::{Analysis, LineIndex, Options};

#[test]
fn every_conformance_error_case() {
    let cases = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/cases");
    let mut checked = 0;
    let mut failures = Vec::new();
    let mut entries: Vec<_> = fs::read_dir(&cases).unwrap().map(|e| e.unwrap().path()).collect();
    entries.sort();
    for expected in entries.iter().filter(|p| p.extension().is_some_and(|e| e == "expected-error")) {
        let source = expected.with_extension("bas");
        let text = rapidr_preprocessor::read_source(&source).unwrap();
        let mut analysis = Analysis::new(Options::default());
        analysis.update(source.clone(), text.clone());
        let index = LineIndex::new(&text);
        let got: Vec<(String, String)> = analysis
            .diagnostics(&source)
            .iter()
            .filter(|d| d.file == source)
            .map(|d| {
                let (l, c) = index.line_col(d.start);
                (format!("{}:{}", l + 1, c + 1), d.message.clone())
            })
            .collect();
        let want = fs::read_to_string(expected).unwrap();
        let mut position: Option<String> = None;
        for line in want.lines().map(str::trim).filter(|l| !l.is_empty()) {
            let is_position = line.split_once(':').is_some_and(|(a, b)| a.parse::<usize>().is_ok() && b.parse::<usize>().is_ok());
            if is_position {
                position = Some(line.to_string());
                continue;
            }
            let ok = got.iter().any(|(p, m)| m.contains(line) && position.as_ref().is_none_or(|want| want == p));
            if !ok {
                failures.push(format!("{}: {:?} {line:?}\n    got {got:?}", source.file_name().unwrap().to_string_lossy(), position));
            }
            position = None;
        }
        checked += 1;
    }
    assert!(checked >= 10, "only {checked} cases");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
