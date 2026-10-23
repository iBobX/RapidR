//! Random patterns and texts through Find (Preview 4 security, X2): a
//! person (or a program, through RCodeEditor and RProject's `Find`) types a
//! regular expression, and Find in Files runs it over every file of a
//! project. Compiling it and searching with it must never panic or hang,
//! whatever the pattern: unbalanced groups, huge repetitions, lookaround and
//! back-references (which the engine refuses), `$` templates in Replace.
//!
//! Studio's Find in Files (crates/rapidr-studio/src/find.rs) is this
//! `Searcher` plus a line index, so this is the path it takes.
//!
//! `RAPIDR_FUZZ_ITERS` sets the number of inputs (default 4000; a long run:
//! `RAPIDR_FUZZ_ITERS=200000`), `RAPIDR_FUZZ_SEED` the first seed. A failure
//! names its seed. An input taking 20 s is a hang.

use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use rapidr_editor::search::{SearchQuery, Searcher};

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

enum Event {
    Started(u64),
    Panicked(u64, String),
    Finished,
}

fn iterations(default: u64) -> u64 {
    std::env::var("RAPIDR_FUZZ_ITERS").ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

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
        match rx.recv_timeout(Duration::from_secs(20)) {
            Ok(Event::Started(seed)) => current = seed,
            Ok(Event::Finished) => break,
            Ok(Event::Panicked(seed, message)) => panic!("{what}: seed {seed} panicked: {message} (RAPIDR_FUZZ_SEED={seed} RAPIDR_FUZZ_ITERS=1)"),
            Err(mpsc::RecvTimeoutError::Timeout) => panic!("{what}: seed {current} hangs (RAPIDR_FUZZ_SEED={current} RAPIDR_FUZZ_ITERS=1)"),
            Err(mpsc::RecvTimeoutError::Disconnected) => panic!("{what}: the fuzzing thread stopped at seed {current}"),
        }
    }
}

/// Pieces of regular expressions, valid and not.
const PIECES: &[&str] = &[
    "(", ")", "[", "]", "{", "}", "*", "+", "?", "|", "^", "$", ".", "\\", "\\d", "\\w", "\\s", "\\b", "\\B", "\\A", "\\z", "\\n", "\\r", "\\t", "\\p{L}", "\\pL", "\\P{L}",
    "\\p{Greek}", "\\p{Nope}", "\\x41", "\\x{1F600}", "\\u0041", "\\1", "\\k<a>", "(?i)", "(?s)", "(?m)", "(?x)", "(?U)", "(?-i)", "(?i:", "(?:", "(?P<n>", "(?<n>", "(?=", "(?!", "(?<=",
    "(?#", "[a-z]", "[^a-z]", "[[:alpha:]]", "[a-z&&[^aeiou]]", "[\\d\\s]", "[]", "[^]", "[a-", "a{2}", "a{2,}", "a{2,5}", "a{5,2}", "a{1000}", "a{100000}", "a{1000}{1000}", "(a{100}){100}",
    "(a*)*", "(a|aa)*", ".*", ".*.*.*", "(.*)(.*)(.*)", "x*y*z*", "a|", "|a", "()", "(|)", "\\Qx\\E", "SUB", "End", "PRINT", "x", "é", "ñ", "€", "\u{1F600}", " ", "\t", "\n",
    "\r\n", "\0", "-", ",", "'", "\"", "$1", "${n}", "${", "$$", "$ ", "&", "#", "~", "<", ">", "=",
];

/// A random pattern: pieces of regular expressions, and now and then a
/// whole piece of a text (a literal search).
fn random_pattern(rng: &mut Rng, text: &str) -> String {
    let mut p = String::new();
    for _ in 0..1 + rng.below(12) {
        match rng.below(8) {
            0 if !text.is_empty() => {
                let mut at = rng.below(text.len());
                while !text.is_char_boundary(at) {
                    at -= 1;
                }
                let mut end = (at + 1 + rng.below(12)).min(text.len());
                while !text.is_char_boundary(end) {
                    end += 1;
                }
                p.push_str(&text[at..end]);
            }
            1 => p.push((b' ' + rng.below(95) as u8) as char),
            _ => p.push_str(PIECES[rng.below(PIECES.len())]),
        }
    }
    p
}

/// A pattern built from a piece of the text itself, each character kept,
/// generalised (`.`, `\\w`, `\\d`, `\\s`, a class) or repeated, with groups,
/// alternations and anchors here and there: it matches, so the match,
/// replacement and whole-word code runs, not only the compiler.
fn pattern_from_text(rng: &mut Rng, text: &str) -> String {
    if text.is_empty() {
        return "x".to_string();
    }
    let at = char_boundary(text, rng.below(text.len()));
    let mut p = String::new();
    if rng.below(8) == 0 {
        p.push('^');
    }
    if rng.below(8) == 0 {
        p.push_str("\\b");
    }
    let mut chars = text[at..].chars();
    for _ in 0..1 + rng.below(16) {
        let Some(c) = chars.next() else { break };
        match rng.below(10) {
            0 => p.push('.'),
            1 => p.push_str(if c.is_ascii_digit() { "\\d" } else { "\\w" }),
            2 => p.push_str("\\s"),
            3 => p.push_str(&format!("[{}x]", regex::escape(&c.to_string()))),
            4 => p.push_str(&format!("[^{}]", regex::escape(&c.to_string()))),
            5 => p.push_str(&format!("({})", regex::escape(&c.to_string()))),
            6 => p.push_str(&format!("(?:{}|q)", regex::escape(&c.to_string()))),
            _ => p.push_str(&regex::escape(&c.to_string())),
        }
        match rng.below(12) {
            0 => p.push('*'),
            1 => p.push('+'),
            2 => p.push('?'),
            3 => p.push_str("{1,3}"),
            4 => p.push_str("*?"),
            _ => {}
        }
    }
    if rng.below(10) == 0 {
        p.push('$');
    }
    p
}

/// A random options string, as RCodeEditor and RProject write them.
fn random_options(rng: &mut Rng) -> String {
    let mut parts = Vec::new();
    for word in ["case", "word", "regex", "REGEX", "Case", "bogus"] {
        if rng.below(2) == 0 {
            parts.push(word);
        }
    }
    parts.join(["", ",", " ", ";"][rng.below(4)])
}

/// The texts searched: the repository's own programs, in pieces.
fn corpus() -> Vec<String> {
    let mut texts = vec![String::new(), "a".to_string(), "\r\n\r\n".to_string(), "é\u{1F600}€ ñ\n".repeat(50), "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa!".to_string()];
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut stack = vec![root.join("examples"), root.join("tests/conformance/cases")];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rr" || e == "bas" || e == "md") {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    if text.len() < 20_000 {
                        texts.push(text);
                    }
                }
            }
        }
    }
    texts.sort();
    texts
}

fn char_boundary(text: &str, mut at: usize) -> usize {
    at = at.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

/// Everything Find in Files, Find, FindNext and Replace do with a query.
/// Returns how far the query got: 0 refused (the pattern didn't compile), 1
/// compiled, 2 compiled and matched.
fn exercise(text: &str, query: &SearchQuery, rng: &mut Rng) -> u8 {
    let Ok(searcher) = Searcher::new(query) else { return 0 };
    let all = searcher.find_all(text, 0..text.len());
    // (matches are non-empty, in order, inside the text, on char boundaries)
    let mut last_end = 0;
    for m in &all {
        assert!(m.start >= last_end && m.start < m.end && m.end <= text.len(), "bad match {m:?}");
        assert!(text.is_char_boundary(m.start) && text.is_char_boundary(m.end), "match {m:?} splits a character");
        last_end = m.end;
    }
    let template = ["$1", "${n}", "$$", "$", "${", "x", "", "\\1", "$0$0", "é$"][rng.below(10)];
    for m in all.iter().take(50) {
        let _ = searcher.replacement(text, m, template);
    }
    let from = char_boundary(text, rng.below(text.len() + 1));
    let _ = searcher.find_from(text, from);
    let _ = searcher.find_before(text, from);
    let a = char_boundary(text, rng.below(text.len() + 1));
    let b = char_boundary(text, rng.below(text.len() + 1));
    let _ = searcher.find_all(text, a.min(b)..a.max(b));
    let _ = searcher.query();
    if all.is_empty() { 1 } else { 2 }
}

#[test]
fn random_patterns_never_panic_or_hang() {
    let texts = corpus();
    // (how far the patterns get: a pool whose patterns all fail to compile
    // tests nothing past the compiler)
    let compiled = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let matched = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (c, m) = (compiled.clone(), matched.clone());
    run_guarded("find patterns", 4000, move |rng| {
        let text = &texts[rng.below(texts.len())];
        // (a slice of the text: the cost of one input stays small)
        let start = char_boundary(text, rng.below(text.len() + 1));
        let end = char_boundary(text, (start + rng.below(6000)).min(text.len()));
        let text = &text[start..end];
        // (a third of the patterns come from the text and run as regular
        // expressions, so that many of them match)
        let from_text = rng.below(3) == 0;
        let pattern = if from_text { pattern_from_text(rng, text) } else { random_pattern(rng, text) };
        let options = if from_text { ["regex", "regex,case", "regex,word"][rng.below(3)].to_string() } else { random_options(rng) };
        let began = std::time::Instant::now();
        let reached = exercise(text, &SearchQuery::with_options(pattern.clone(), &options), rng);
        if reached >= 1 {
            c.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        if reached == 2 {
            m.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        // (a slow input is shown with `-- --nocapture`: what makes one
        // pattern slow is worth knowing)
        if began.elapsed() > Duration::from_millis(250) {
            eprintln!("slow ({:?}): pattern {pattern:?}, options {options:?}, text of {} bytes", began.elapsed(), text.len());
        }
    });
    let tried = iterations(4000) as usize;
    let (compiled, matched) = (compiled.load(std::sync::atomic::Ordering::Relaxed), matched.load(std::sync::atomic::Ordering::Relaxed));
    eprintln!("find patterns: {compiled} of {tried} compiled, {matched} matched");
    if tried >= 1000 {
        assert!(compiled * 100 >= tried * 20, "only {compiled} of {tried} patterns compiled");
        assert!(matched * 100 >= tried * 5, "only {matched} of {tried} patterns matched something");
    }
}

#[test]
fn random_literal_searches_find_what_is_there() {
    // (the property beyond "no panic": a literal piece of the text is found,
    // at least where it is)
    let texts = corpus();
    run_guarded("literal find", 2000, move |rng| {
        let text = &texts[rng.below(texts.len())];
        if text.is_empty() {
            return;
        }
        let start = char_boundary(text, rng.below(text.len()));
        let end = char_boundary(text, (start + 1 + rng.below(20)).min(text.len()));
        let piece = &text[start..end];
        if piece.trim().is_empty() {
            return;
        }
        let mut q = SearchQuery::literal(piece);
        q.case_sensitive = true;
        let s = Searcher::new(&q).expect("a literal always compiles");
        let all = s.find_all(text, 0..text.len());
        assert!(all.iter().any(|m| m.start <= start && start < m.end) || all.iter().any(|m| m.start >= start), "{piece:?} not found in its own text");
        for m in &all {
            assert_eq!(&text[m.clone()], piece);
        }
    });
}

/// Patterns that are valid but cost the most to compile or to run: huge
/// counted repetitions, nested ones, deep nesting, big classes. Each must
/// finish (compiled and run over a text, or refused for its size) well
/// inside a time budget: Find recompiles at every key typed.
#[test]
fn expensive_patterns_finish_within_a_budget() {
    let text = "SUB Main\n  PRINT \"héllo wörld\" ' 😀 a comment\r\nEND SUB\n".repeat(60);
    let deep = |open: &str, close: &str, n: usize| format!("{}a{}", open.repeat(n), close.repeat(n));
    let patterns = vec![
        "(\\pL{50}){50}".to_string(),
        "(\\pL{100}){100}".to_string(),
        "((a{100}){100}){100}".to_string(),
        "a{100000}".to_string(),
        "\\pL{1000}".to_string(),
        "(\\w+\\s*){1000}".to_string(),
        ".{100000}".to_string(),
        "[a-z]{10000}{10000}".to_string(),
        "(?i)\\pL{500}".to_string(),
        "(\\p{Greek}|\\p{Cyrillic}|\\p{Han}){200}".to_string(),
        "(a*)*(b*)*(c*)*$".to_string(),
        "(.*)(.*)(.*)(.*)(.*)(.*)(.*)(.*)x".to_string(),
        deep("(", ")", 100),
        deep("(", ")", 1000),
        deep("(?:", ")", 1000),
        deep("[", "]", 1000),
        "[\\p{L}\\p{N}\\p{P}\\p{S}\\p{Z}\\p{C}]".repeat(100),
        "a|".repeat(20_000),
        "\\w".repeat(5_000),
        "x".repeat(50_000),
    ];
    let budget = Duration::from_secs(10);
    for p in &patterns {
        for options in ["regex", "regex,case,word", ""] {
            let began = std::time::Instant::now();
            let mut rng = Rng(7);
            exercise(&text, &SearchQuery::with_options(p.clone(), options), &mut rng);
            let took = began.elapsed();
            let shown: String = p.chars().take(40).collect();
            if took > Duration::from_millis(250) {
                eprintln!("slow ({took:?}): pattern {shown:?}... ({} bytes), options {options:?}", p.len());
            }
            assert!(took < budget, "pattern {shown:?}... ({} bytes), options {options:?}: {took:?}", p.len());
        }
    }
}
