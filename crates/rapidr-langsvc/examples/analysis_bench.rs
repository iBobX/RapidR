//! What one analysis of a program costs, stage by stage (docs/speed-audit-
//! 2026-11.md, "Studio: the edit pause"): the parse and the names (one
//! snapshot), the compiler's checks with the registry's word, the colours,
//! the outline, the helpers, the project's problems.
//!
//!     cargo run --release -p rapidr-langsvc --example analysis_bench -- FILE [RUNS]

use std::path::PathBuf;
use std::time::Instant;

use rapidr_langsvc::{Analysis, Options};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let file = PathBuf::from(args.get(1).expect("a file"));
    let runs: usize = args.get(2).and_then(|r| r.parse().ok()).unwrap_or(5);
    let text = std::fs::read_to_string(&file).expect("its text");
    let mut best: Vec<(String, f64)> = Vec::new();
    for _ in 0..runs {
        let mut a = Analysis::new(Options::default());
        let mut times = Vec::new();
        let mut t = Instant::now();
        let mut lap = |what: String, t: &mut Instant| {
            times.push((what, t.elapsed().as_secs_f64() * 1000.0));
            *t = Instant::now();
        };
        a.update(file.clone(), text.clone());
        a.refresh(&file);
        lap("parse + names (snapshot)".into(), &mut t);
        let d = a.diagnostics(&file);
        lap(format!("diagnostics ({})", d.len()), &mut t);
        let tokens = a.semantic_tokens(&file);
        lap(format!("semantic tokens ({})", tokens.len()), &mut t);
        let outline = a.outline(&file);
        lap(format!("outline ({})", outline.len()), &mut t);
        let helpers = a.helpers(&file);
        lap(format!("helpers ({})", helpers.len()), &mut t);
        let p = a.project_diagnostics(&file);
        lap(format!("project diagnostics ({})", p.len()), &mut t);
        for (i, (what, ms)) in times.into_iter().enumerate() {
            if best.len() <= i {
                best.push((what, ms));
            } else if ms < best[i].1 {
                best[i].1 = ms;
            }
        }
    }
    println!("{} (the fastest of {runs}):", file.display());
    for (what, ms) in best {
        println!("  {what:<36} {ms:9.2} ms");
    }
}
