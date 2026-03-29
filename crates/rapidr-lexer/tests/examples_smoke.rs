use std::fs;
use std::path::Path;

use rapidr_lexer::lex_file;

#[test]
fn lexes_every_example_program() {
    let examples_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples");

    let mut files = fs::read_dir(&examples_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("rp"))
        .collect::<Vec<_>>();

    files.sort();

    let mut failures = Vec::new();
    for file in files {
        if let Err(error) = lex_file(&file) {
            failures.push(format!("{} => {error}", file.display()));
        }
    }

    assert!(
        failures.is_empty(),
        "example lex failures:\n{}",
        failures.join("\n")
    );
}