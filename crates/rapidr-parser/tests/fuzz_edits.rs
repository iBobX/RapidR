//! Random edits of real programs through the preprocessor, the lexer and
//! the parser (docs/ide-plan.md, I0): an editor feeds them every half-typed
//! state of a file, so nothing may panic or hang, and the lossless view
//! must still print each edited text back.
//!
//! Three generators feed the same entry points: real programs with random
//! edits (`random_edits_never_panic_or_hang`), random soup of RapidR's own
//! words and symbols (`random_token_soup_…`), and random bytes
//! (`random_bytes_…`). The property is "never panics, never hangs" (an
//! input taking 20 s is a hang).
//!
//! `RAPIDR_FUZZ_ITERS` sets the number of inputs each test tries (default
//! 2000 edited programs, 20,000 soups and 20,000 byte strings: under 10 s
//! each; regress runs the default; a long run: `RAPIDR_FUZZ_ITERS=200000`,
//! best with `--release`; `RAPIDR_FUZZ_CASES` is the older name),
//! `RAPIDR_FUZZ_SEED` the first seed. A failure names its seed: rerun with
//! that seed and 1 iteration.

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

/// How many inputs a test tries: `RAPIDR_FUZZ_ITERS` (or the older
/// `RAPIDR_FUZZ_CASES`), `default` otherwise.
fn iterations(default: u64) -> u64 {
    ["RAPIDR_FUZZ_ITERS", "RAPIDR_FUZZ_CASES"]
        .iter()
        .find_map(|name| std::env::var(name).ok().and_then(|v| v.parse().ok()))
        .unwrap_or(default)
}

/// Runs `one(rng)` for every seed on a thread of its own; a panic or an
/// input that takes 20 s fails the test with its seed.
fn run_guarded<F>(what: &'static str, default_iters: u64, one: F)
where
    F: Fn(&mut Rng) + Send + std::panic::RefUnwindSafe + 'static,
{
    let cases = iterations(default_iters);
    let first: u64 = std::env::var("RAPIDR_FUZZ_SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
    let (tx, rx) = mpsc::channel::<Event>();
    std::thread::spawn(move || {
        for seed in first..first + cases {
            let _ = tx.send(Event::Started(seed));
            let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
            if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| one(&mut rng))) {
                let message = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
                let _ = tx.send(Event::Panicked(seed, message));
                return;
            }
        }
        let _ = tx.send(Event::Finished);
    });
    let mut current = first;
    loop {
        // (one input taking this long is a hang)
        match rx.recv_timeout(Duration::from_secs(20)) {
            Ok(Event::Started(seed)) => current = seed,
            Ok(Event::Finished) => break,
            Ok(Event::Panicked(seed, message)) => panic!("{what}: seed {seed} panicked: {message} (RAPIDR_FUZZ_SEED={seed} RAPIDR_FUZZ_ITERS=1)"),
            Err(mpsc::RecvTimeoutError::Timeout) => panic!("{what}: seed {current} hangs (RAPIDR_FUZZ_SEED={current} RAPIDR_FUZZ_ITERS=1)"),
            Err(mpsc::RecvTimeoutError::Disconnected) => panic!("{what}: the fuzzing thread stopped at seed {current}"),
        }
    }
}

#[test]
fn random_edits_never_panic_or_hang() {
    let texts = corpus();
    assert!(!texts.is_empty());
    run_guarded("random edits", 2000, move |rng| {
        let text = mutate(&texts[rng.below(texts.len())], rng);
        exercise(&text);
    });
}

/// Words and symbols of the language in random order: far from any real
/// program, so the parser's error paths (and its recovery) run all the time.
const WORDS: &[&str] = &[
    "SUB", "END", "FUNCTION", "IF", "THEN", "ELSE", "ELSEIF", "FOR", "TO", "STEP", "NEXT", "WHILE", "WEND", "DO", "LOOP", "UNTIL", "SELECT", "CASE", "IS",
    "DIM", "AS", "INTEGER", "STRING", "BYTE", "DOUBLE", "TYPE", "RECORD", "CREATE", "QFORM", "RFORM", "RBUTTON", "WITH", "CALL", "DECLARE", "LIB", "ALIAS",
    "CONST", "DEFINT", "DEFSTR", "SET", "EXIT", "GOTO", "GOSUB", "RETURN", "ON", "ERROR", "RESUME", "PRINT", "INPUT", "OPEN", "CLOSE", "REDIM", "PRESERVE",
    "ANDALSO", "AND", "OR", "XOR", "NOT", "MOD", "SHL", "SHR", "EVENT", "PROPERTY", "GET", "LET", "$INCLUDE", "$IFDEF", "$ENDIF", "$DEFINE", "$MACRO",
    "$TYPECHECK", "$ESCAPECHARS", "$OPTIMIZE", "$APPTYPE", "RUSTSTART", "RUSTEND", "REM", "x", "y", "i", "name$", "n%", "d#", "f!", "Form1", "Caption",
    "Click", "Form1.Caption", "a.b.c", "0", "1", "42", "3.14", "1E5", "&HFF", "&O17", "&B101", "\"text\"", "\"\"", "\"a\\nb\"", "'comment", "(", ")", "[",
    "]", ",", ":", ".", "=", "<>", "<=", ">=", "<", ">", "+", "-", "*", "/", "\\", "^", "&", "?", "@", "#", ";", "_", "\n", "\n", "\n", "\r\n", " ", " ",
    " ", "\t",
];

#[test]
fn random_token_soup_never_panics_or_hangs() {
    run_guarded("token soup", 20_000, |rng| {
        let mut text = String::new();
        for _ in 0..1 + rng.below(120) {
            text.push_str(WORDS[rng.below(WORDS.len())]);
            if rng.below(3) != 0 {
                text.push(' ');
            }
        }
        exercise(&text);
    });
}

#[test]
fn random_bytes_never_panic_or_hang() {
    run_guarded("random bytes", 20_000, |rng| {
        const PUNCTUATION: &[u8] = b"\r\n\t\"'_:(),.$&#?@ ";
        let len = rng.below(300);
        let bytes: Vec<u8> = (0..len)
            .map(|_| match rng.below(4) {
                // mostly text, with every kind of byte among it
                0 => rng.below(256) as u8,
                1 => PUNCTUATION[rng.below(PUNCTUATION.len())],
                _ => b' ' + rng.below(95) as u8,
            })
            .collect();
        exercise(&rapidr_preprocessor::decode_source(&bytes).0);
    });
}
