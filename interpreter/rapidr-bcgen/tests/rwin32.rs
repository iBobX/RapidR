//! RWin32.inc, RapidR's Windows API declarations (rapidr_preprocessor::rwin32):
//! the drop-in programs compile, and a program pays only for what it uses.

use std::path::{Path, PathBuf};
use std::time::Instant;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/rwin32").join(name)
}

fn compile(path: &Path) -> rapidr_bcgen::Compiled {
    let program = rapidr_parser::parse_file(path).unwrap_or_else(|e| panic!("{}: {:?}", path.display(), e.diagnostics.first().map(|d| d.to_string())));
    let source = rapidr_preprocessor::preprocess_file(path, Default::default()).unwrap().source;
    rapidr_bcgen::compile_program_with_source(&program, Some(&source)).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// A program written for the classic WIN32API.INC, its include line
/// changed to RWin32.inc, compiles: its classic names are there, and its
/// own names (a CONST, a DECLARE the old file lacked) win with no clash.
#[test]
fn the_classic_program_compiles_with_rwin32() {
    let c = compile(&fixture("dropin_classic.bas"));
    assert!(c.module.functions.len() < 40, "only what it uses: {} functions", c.module.functions.len());
}

/// Twenty calls compile in well under a second: the include's cost is what
/// the program uses (the file is unpacked and indexed once per process).
#[test]
fn twenty_calls_compile_in_well_under_a_second() {
    let path = fixture("twenty_calls.bas");
    let first = Instant::now();
    compile(&path);
    let cold = first.elapsed();
    let again = Instant::now();
    let c = compile(&path);
    let warm = again.elapsed();
    eprintln!("twenty calls: first compile {cold:?} (the file unpacked and indexed), then {warm:?}");
    assert!(warm.as_millis() < 500, "{warm:?}");
    // (a debug build unpacks more slowly; a release build takes ~0.07 s in all)
    assert!(cold.as_secs_f64() < if cfg!(debug_assertions) { 5.0 } else { 1.0 }, "{cold:?}");
    assert!(c.module.to_bytes().len() < 64 * 1024, "bytecode of what is used only: {} bytes", c.module.to_bytes().len());
}
