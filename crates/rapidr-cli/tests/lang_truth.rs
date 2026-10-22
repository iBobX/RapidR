//! `rapidr lang truth` passes: the language reference tells the truth
//! (docs/preview4-plan.md K7). Every `syntax` line of a statement, a
//! directive and a builtin compiles, every `example` compiles, and every
//! example with an `expect` is run and prints what it says. A failure names
//! the registry entry and what is wrong (crates/rapidr-lang/data/*.toml).

use std::process::{Command, Stdio};

#[test]
fn the_reference_tells_the_truth() {
    let scratch = std::env::temp_dir().join(format!("rapidr-lang-truth-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).unwrap();
    // (nothing the programs do reaches a real printer, the user's settings or keys)
    let out = Command::new(env!("CARGO_BIN_EXE_rapidr"))
        .args(["lang", "truth"])
        .current_dir(&scratch)
        .env("RAPIDR_PRINT_TO", scratch.join("prints"))
        .env("RAPIDR_REGISTRY", scratch.join("registry.reg"))
        .env("RAPIDR_AI_KEYSTORE", "memory")
        .stdin(Stdio::null())
        .output()
        .expect("runs rapidr");
    let _ = std::fs::remove_dir_all(&scratch);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "rapidr lang truth failed:\n{text}{}", String::from_utf8_lossy(&out.stderr));
    // (it did the work: syntax lines, examples, and the ones with an `expect` run)
    assert!(text.contains("programs from syntax lines") && text.contains("run and print what `expect` says"), "{text}");
}
