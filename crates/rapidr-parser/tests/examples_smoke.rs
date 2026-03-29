use std::fs;
use std::path::Path;

use rapidr_parser::parse_file;

#[test]
fn parses_every_example_program_into_partial_typed_ast() {
    let examples_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");

    let mut files = fs::read_dir(&examples_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("rp"))
        .collect::<Vec<_>>();

    files.sort();

    let mut failures = Vec::new();
    for file in files {
        match parse_file(&file) {
            Ok(program) if !program.statements.is_empty() => {}
            Ok(_) => failures.push(format!("{} => parser produced no statements", file.display())),
            Err(error) => failures.push(format!("{} => {error}", file.display())),
        }
    }

    assert!(
        failures.is_empty(),
        "example parse failures:\n{}",
        failures.join("\n")
    );
}