//! The semantic model on real programs: every RapidQ example of the corpus
//! (`$RAPIDQ_DIR`, default `~/Downloads/Rapidq`) and every program of the
//! repo. No panic in the compiler while recording, and every reference
//! holds its name, in the right file through `$INCLUDE`s.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use rapidr_bcgen::semantic::{analyze, name_key};

static PANICS: AtomicUsize = AtomicUsize::new(0);

fn programs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            if !matches!(name.as_str(), "target" | "node_modules" | ".git" | ".work" | ".claude" | "scratch") {
                programs(&path, out);
            }
        } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("bas") || e.eq_ignore_ascii_case("rr")) {
            out.push(path);
        }
    }
}

#[test]
fn every_program_has_a_sound_semantic_model() {
    let mut files = Vec::new();
    programs(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."), &mut files);
    let rapidq = std::env::var_os("RAPIDQ_DIR").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join("Downloads/Rapidq")));
    if let Some(dir) = rapidq {
        programs(&dir.join("examples"), &mut files);
    }
    files.sort();
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        PANICS.fetch_add(1, Ordering::SeqCst);
        default_hook(info);
    }));
    let mut failures = Vec::new();
    let (mut symbols, mut references, mut in_includes) = (0, 0, 0);
    for path in &files {
        let Ok(parse) = rapidr_parser::parse_file_for_tools(path, Default::default()) else { continue };
        let before = PANICS.load(Ordering::SeqCst);
        let source = &parse.preprocessed.source;
        let model = analyze(&parse.program, Some(source));
        if PANICS.load(Ordering::SeqCst) != before {
            failures.push(format!("{}: the compiler panicked while recording", path.display()));
        }
        symbols += model.symbols.len();
        references += model.references.len();
        for r in &model.references {
            let text = &source[r.span.start..r.span.end];
            let symbol = &model.symbols[r.symbol];
            // (RESULT in a FUNCTION is the FUNCTION's result)
            let result = name_key(text) == "result" && symbol.kind == rapidr_bcgen::semantic::SymbolKind::Function;
            if name_key(text) != name_key(&symbol.name) && !result {
                failures.push(format!("{}: {:?} at {} for {:?}", path.display(), text, r.span.start, symbol.name));
                continue;
            }
            match parse.locate(r.span) {
                Some(loc) if loc.exact => {
                    let file = &parse.preprocessed.origins.files[loc.file].text;
                    if file.get(loc.start..loc.end) != Some(text) {
                        failures.push(format!("{}: {:?} maps to {:?}", path.display(), text, file.get(loc.start..loc.end)));
                    }
                    if loc.path.as_deref() != Some(path.as_path()) {
                        in_includes += 1;
                    }
                }
                Some(_) => {}
                None => failures.push(format!("{}: {:?} at {} has no origin", path.display(), text, r.span.start)),
            }
        }
        // References are sorted and don't overlap.
        if model.references.windows(2).any(|w| w[1].span.start < w[0].span.end) {
            failures.push(format!("{}: overlapping references", path.display()));
        }
    }
    let _ = std::panic::take_hook();
    eprintln!("{} programs: {symbols} symbols, {references} references ({in_includes} in included files)", files.len());
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.iter().take(30).cloned().collect::<Vec<_>>().join("\n"));
}
