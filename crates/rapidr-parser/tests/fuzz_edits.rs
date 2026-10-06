//! Random edits of real programs through the preprocessor, the lexer and
//! the parser (docs/ide-plan.md, I0): an editor feeds them every half-typed
//! state of a file, so nothing may panic or hang, and the lossless view
//! must still print each edited text back.
//!
//! `RAPIDR_FUZZ_CASES` sets the number of edited texts (default 400, a
//! couple of seconds; regress runs the default), `RAPIDR_FUZZ_SEED` the
//! first seed. A failure names its seed: rerun with that seed and 1 case.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use rapidr_lexer::{Lexer, LosslessFile};

/// xorshift64*: deterministic, no dependency.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next() % n as u64) as usize }
    }
}

/// Snippets an edit may insert: the lexer's and parser's edge cases.
const SNIPPETS: &[&str] = &[
    "\"", "'", "_", "_\n", "\r\n", "\n", ":", "(", ")", "[", "]", "{", "}", ",", ".", "&H", "&O", "&B", "&", "?", "??", "@", "#", "$",
    "$INCLUDE \"x.inc\"\n", "$IFDEF WIN32\n", "$IFNDEF X\n", "$ELSE\n", "$ENDIF\n", "$DEFINE A B\n", "$MACRO M(a) = (a)\n", "$MACRO = x\n",
    "#If 1 Then\n", "#End If\n", "#Const = 1\n", "$RESOURCE R AS \"r.bmp\"\n", "$ESCAPECHARS ON\n", "\"\\x4", "\\", "REM ", "DATA 1, \"a",
    "SUB ", "END SUB", "FUNCTION F(", "END FUNCTION", "IF ", " THEN ", "ELSE", "END IF", "FOR i = 1 TO ", "NEXT", "WHILE ", "WEND",
    "DO", "LOOP UNTIL ", "SELECT CASE ", "CASE IS > ", "END SELECT", "CREATE f AS QFORM\n", "END CREATE", "WITH ", "END WITH",
    "TYPE T\n", "END TYPE", "DIM a(", " AS ", "STRING * ", "DECLARE SUB ", " LIB \"", "RUSTSTART\n", "RUSTEND\n", "GOTO ", "GOSUB ",
    "1E", "1.5e+", ".5", "0??", "é", "€", "\u{1F600}", "\t", "    ", "\0", "\u{7f}", "`", "~", "|",
];

fn mutate(text: &str, rng: &mut Rng) -> String {
    let mut bytes = text.as_bytes().to_vec();
    for _ in 0..1 + rng.below(6) {
        let len = bytes.len();
        let at = rng.below(len + 1);
        match rng.below(6) {
            // delete a run
            0 if len > 0 => {
                let end = (at + 1 + rng.below(40)).min(len);
                bytes.drain(at.min(len)..end);
            }
            // insert a snippet
            1 | 2 => {
                let s = SNIPPETS[rng.below(SNIPPETS.len())];
                bytes.splice(at..at, s.bytes());
            }
            // duplicate a slice elsewhere
            3 if len > 0 => {
                let from = rng.below(len);
                let end = (from + 1 + rng.below(80)).min(len);
                let slice = bytes[from..end].to_vec();
                bytes.splice(at..at, slice);
            }
            // a random byte
            4 => bytes.insert(at, rng.below(256) as u8),
            // cut the text here
            _ => bytes.truncate(at),
        }
    }
    // (editors hold text: invalid UTF-8 is read as RapidR reads files)
    rapidr_preprocessor::decode_source(&bytes).0
}

/// Everything a tool runs on a text.
fn exercise(text: &str) {
    let file = LosslessFile::lex(text, None);
    assert_eq!(file.print(), text, "lossless print");
    let _ = file.lines();
    let (pre, _) = rapidr_preprocessor::preprocess_source_recovering(text, ".", Some(PathBuf::from("fuzz.bas")), Default::default());
    let (tokens, _) = Lexer::new(&pre.source, None).tokenize_recovering();
    let (program, _) = rapidr_parser::parse_tokens_recovering(&tokens);
    for s in &program.statements {
        let span = rapidr_parser::statement_span(s);
        let _ = pre.origins.origin_span(span.start, span.end);
    }
    // The compiler's own entry points too.
    let _ = Lexer::new(text, None).tokenize().map(|t| rapidr_parser::parse_tokens(&t));
}

fn corpus() -> Vec<String> {
    let mut texts = Vec::new();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut dirs = vec![root.join("examples"), root.join("tests/conformance/cases")];
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(Path::new(&home).join("Downloads/Rapidq/examples"));
    }
    if let Some(dir) = std::env::var_os("RAPIDQ_DIR") {
        dirs.push(Path::new(&dir).join("examples"));
    }
    for dir in dirs {
        let mut stack = vec![dir];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = fs::read_dir(&dir) else { continue };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("bas") || e.eq_ignore_ascii_case("rr")) {
                    if let Ok(bytes) = fs::read(&path) {
                        // (large files make each case slow without finding more)
                        if bytes.len() < 60_000 {
                            texts.push(rapidr_preprocessor::decode_source(&bytes).0);
                        }
                    }
                }
            }
        }
    }
    texts.sort();
    texts
}

enum Event {
    Started(u64),
    Panicked(u64, String),
    Finished,
}

#[test]
fn random_edits_never_panic_or_hang() {
    let cases: u64 = std::env::var("RAPIDR_FUZZ_CASES").ok().and_then(|v| v.parse().ok()).unwrap_or(400);
    let first: u64 = std::env::var("RAPIDR_FUZZ_SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
    let texts = corpus();
    assert!(!texts.is_empty());
    let (tx, rx) = mpsc::channel::<Event>();
    std::thread::spawn(move || {
        for seed in first..first + cases {
            let _ = tx.send(Event::Started(seed));
            let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
            let text = mutate(&texts[rng.below(texts.len())], &mut rng);
            if let Err(e) = std::panic::catch_unwind(|| exercise(&text)) {
                let message = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
                let _ = tx.send(Event::Panicked(seed, message));
                return;
            }
        }
        let _ = tx.send(Event::Finished);
    });
    let mut current = first;
    loop {
        // (one edited text taking this long is a hang)
        match rx.recv_timeout(Duration::from_secs(20)) {
            Ok(Event::Started(seed)) => current = seed,
            Ok(Event::Finished) => break,
            Ok(Event::Panicked(seed, message)) => panic!("seed {seed} panicked: {message} (RAPIDR_FUZZ_SEED={seed} RAPIDR_FUZZ_CASES=1)"),
            Err(mpsc::RecvTimeoutError::Timeout) => panic!("seed {current} hangs (RAPIDR_FUZZ_SEED={current} RAPIDR_FUZZ_CASES=1)"),
            Err(mpsc::RecvTimeoutError::Disconnected) => panic!("the fuzzing thread stopped at seed {current}"),
        }
    }
}
