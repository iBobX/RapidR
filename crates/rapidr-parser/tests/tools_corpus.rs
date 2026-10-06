//! The parser for tools on real programs (docs/ide-plan.md, I0 acceptance):
//! every RapidQ example of the corpus and every .bas / .rr / .inc file of
//! the repo parses with trivia and prints back byte for byte, and spans
//! point into the right file at the right byte, through `$INCLUDE`s.
//!
//! The RapidQ corpus is read from `$RAPIDQ_DIR` (default
//! `~/Downloads/Rapidq`: its `examples` and every `.inc`); without it only
//! the repo's files are checked.

use std::fs;
use std::path::{Path, PathBuf};

use rapidr_lexer::{LosslessFile, Piece};
use rapidr_parser::{parse_file_for_tools, ToolsParse};
use rapidr_preprocessor::{decode_source, encode_source};

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            if !matches!(name.as_str(), "target" | "node_modules" | ".git" | ".work" | ".claude" | "scratch") {
                walk(&path, out);
            }
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ["bas", "rr", "inc"].contains(&ext.to_ascii_lowercase().as_str()) {
                out.push(path);
            }
        }
    }
}

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn rapidq_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("RAPIDQ_DIR").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join("Downloads/Rapidq")))?;
    dir.join("examples").is_dir().then_some(dir)
}

/// Lexes a file losslessly and checks it prints back to its bytes.
fn check_lossless(path: &Path, failures: &mut Vec<String>) -> Option<LosslessFile> {
    let bytes = fs::read(path).ok()?;
    let (text, encoding) = decode_source(&bytes);
    let file = LosslessFile::lex(&text, Some(path.display().to_string()));
    check_pieces(path, &file, failures);
    if encode_source(&file.print(), encoding) != bytes {
        failures.push(format!("{}: tokens + trivia don't give the file's bytes back", path.display()));
    }
    Some(file)
}

fn check_pieces(path: &Path, file: &LosslessFile, failures: &mut Vec<String>) {
    let mut at = 0;
    for piece in file.pieces() {
        let span = piece.span();
        if span.start != at {
            failures.push(format!("{}: gap or overlap at byte {at} ({:?})", path.display(), piece));
            return;
        }
        if let Piece::Trivia(t) = piece {
            if t.kind == rapidr_lexer::TriviaKind::Other {
                failures.push(format!("{}: unclassified trivia {:?} at {}", path.display(), file.piece_text(piece), span.start));
            }
        }
        at = span.end;
    }
    if at != file.text.len() {
        failures.push(format!("{}: pieces end at {at} of {}", path.display(), file.text.len()));
    }
    if file.print() != file.text {
        failures.push(format!("{}: print() differs from the text", path.display()));
    }
}

/// The origin map covers the preprocessed text, and every token whose span
/// maps exactly is the same text in its file.
fn check_origins(path: &Path, parse: &ToolsParse, failures: &mut Vec<String>) -> usize {
    let pre = &parse.preprocessed;
    let mut at = 0;
    for s in &pre.origins.segments {
        if s.pp_start != at {
            failures.push(format!("{}: origin segments not contiguous at {at}", path.display()));
            return 0;
        }
        at = s.pp_end;
    }
    if at != pre.source.len() {
        failures.push(format!("{}: origin map ends at {at} of {}", path.display(), pre.source.len()));
    }
    let mut checked = 0;
    for token in parse.tokens.iter().filter(|t| t.span.end > t.span.start) {
        let Some(loc) = parse.locate(token.span) else {
            failures.push(format!("{}: token at {} has no origin", path.display(), token.span.start));
            continue;
        };
        if !loc.exact {
            continue;
        }
        let text = &pre.origins.files[loc.file].text;
        if text.get(loc.start..loc.end) != Some(&pre.source[token.span.start..token.span.end]) {
            failures.push(format!(
                "{}: token {:?} maps to {:?} at {}:{}",
                path.display(),
                &pre.source[token.span.start..token.span.end],
                text.get(loc.start..loc.end),
                loc.path.as_ref().map(|p| p.display().to_string()).unwrap_or_default(),
                loc.line
            ));
        }
        checked += 1;
    }
    for (i, file) in parse.files.iter().enumerate() {
        check_pieces(pre.origins.files[i].path.as_deref().unwrap_or(path), file, failures);
    }
    checked
}

fn run(files: &[PathBuf], label: &str) {
    let mut failures = Vec::new();
    let (mut programs, mut statements, mut diagnostics, mut tokens, mut lossless) = (0, 0, 0, 0, 0);
    for path in files {
        if check_lossless(path, &mut failures).is_some() {
            lossless += 1;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        if ext == "inc" {
            continue;
        }
        match parse_file_for_tools(path, Default::default()) {
            Ok(parse) => {
                programs += 1;
                statements += parse.program.statements.len();
                diagnostics += parse.diagnostics.len();
                tokens += check_origins(path, &parse, &mut failures);
            }
            Err(e) => failures.push(format!("{}: {e}", path.display())),
        }
    }
    eprintln!("{label}: {lossless} files round-trip byte for byte; {programs} programs parsed for tools ({statements} top-level statements, {diagnostics} diagnostics); {tokens} token spans checked against their files");
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.iter().take(40).cloned().collect::<Vec<_>>().join("\n"));
}

#[test]
fn every_repo_program_parses_with_trivia_and_round_trips() {
    let mut files = Vec::new();
    walk(&repo_root(), &mut files);
    files.sort();
    assert!(files.len() > 100, "the repo's programs are found ({})", files.len());
    run(&files, "repo");
}

#[test]
fn every_rapidq_example_parses_with_trivia_and_round_trips() {
    let Some(dir) = rapidq_dir() else {
        eprintln!("RapidQ corpus not found (set RAPIDQ_DIR): skipped");
        return;
    };
    let mut files = Vec::new();
    walk(&dir, &mut files);
    files.retain(|p| p.starts_with(dir.join("examples")) || p.extension().is_some_and(|e| e.eq_ignore_ascii_case("inc")));
    files.sort();
    let examples = files.iter().filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("bas"))).count();
    assert!(examples >= 380, "the corpus' examples are found ({examples})");
    run(&files, "RapidQ corpus");
}

#[test]
fn spans_point_into_included_files() {
    let unique = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let root = std::env::temp_dir().join(format!("rapidr-tools-{unique}"));
    fs::create_dir_all(root.join("lib")).unwrap();
    let inc = "' helpers\r\n$DEFINE GREETING \"hi\"\r\nSUB Greet (who AS STRING)\r\n    PRINT GREETING; who\r\nEND SUB\r\n";
    let inner = "FUNCTION Twice (n AS INTEGER) AS INTEGER\n    Twice = n * 2\nEND FUNCTION\n";
    fs::write(root.join("lib/greet.inc"), inc).unwrap();
    fs::write(root.join("lib/twice.inc"), inner).unwrap();
    let main = "$INCLUDE \"lib\\greet.inc\"\n' the program\n\n$INCLUDE \"lib/twice.inc\"\n$INCLUDE \"missing.inc\"\nGreet \"you\"\nPRINT Twice(21)\n";
    let path = root.join("main.bas");
    fs::write(&path, main).unwrap();
    let parse = parse_file_for_tools(&path, Default::default()).unwrap();

    // The missing include: one error, on its line of main.bas, and the rest parsed.
    let located = parse.diagnostic_locations();
    assert_eq!(located.len(), 1, "{:?}", parse.diagnostics);
    let (loc, d) = &located[0];
    assert!(d.message.contains("missing.inc"));
    let loc = loc.as_ref().unwrap();
    assert_eq!((loc.path.as_deref(), loc.line, &main[loc.start..loc.end]), (Some(path.as_path()), 5, "$INCLUDE \"missing.inc\""));

    let find = |name: &str| {
        parse
            .program
            .statements
            .iter()
            .find_map(|s| match s {
                rapidr_ast::Statement::Subroutine(r) if r.name == name => Some(r.span),
                rapidr_ast::Statement::Function(f) if f.name == name => Some(f.span),
                _ => None,
            })
            .unwrap()
    };
    // The SUB: in greet.inc, from `SUB` to `END SUB`, at its byte.
    let loc = parse.locate(find("Greet")).unwrap();
    let file = &parse.preprocessed.origins.files[loc.file];
    assert_eq!(file.path.as_deref(), Some(root.join("lib/greet.inc").as_path()));
    assert_eq!((loc.start, loc.line, loc.column), (inc.find("SUB Greet").unwrap(), 3, 1));
    assert!(file.text[loc.start..loc.end].starts_with("SUB Greet (who AS STRING)") && file.text[loc.start..loc.end].trim_end().ends_with("END SUB"));
    // The FUNCTION: in twice.inc.
    let loc = parse.locate(find("Twice")).unwrap();
    assert_eq!(parse.preprocessed.origins.files[loc.file].path.as_deref(), Some(root.join("lib/twice.inc").as_path()));
    assert_eq!(loc.start, 0);
    // The main program's own statements: in main.bas.
    let last = parse.program.statements.last().unwrap();
    let loc = parse.locate(rapidr_parser::statement_span(last)).unwrap();
    assert_eq!((loc.path.as_deref(), &main[loc.start..loc.end]), (Some(path.as_path()), "PRINT Twice(21)"));
    // A $DEFINE's value maps (not exactly) to the name in the include.
    let greeting = parse.tokens.iter().find(|t| t.lexeme == "hi").unwrap();
    let loc = parse.locate(greeting.span).unwrap();
    assert!(!loc.exact);
    assert_eq!(&inc[loc.start..loc.end], "GREETING");
    // From a file position (the caret on `Twice` in twice.inc) back to the AST's text.
    let twice = parse.file_id(&root.join("lib/twice.inc")).unwrap();
    let pp = parse.to_preprocessed(twice, inner.find("n * 2").unwrap()).unwrap();
    assert_eq!(&parse.preprocessed.source[pp..pp + 5], "n * 2");
    // Every file lossless, with the directives as trivia.
    for (i, f) in parse.files.iter().enumerate() {
        assert_eq!(f.print(), parse.preprocessed.origins.files[i].text);
    }
    let main_file = parse.file_id(&path).unwrap();
    let directives = parse.files[main_file].trivia.iter().filter(|t| t.kind == rapidr_lexer::TriviaKind::Directive).count();
    assert_eq!(directives, 3);
    let _ = fs::remove_dir_all(&root);
}
