//! Random and mutated project files (Preview 4 security, X2): a `.rrproj`
//! (TOML), a web IDE v1 project (JSON) and a workspace file come from
//! whoever shares a project, so reading them must never panic or hang, and
//! a v1 import must never name a file outside the project's folder.
//!
//! The seeds are the repository's own projects (tests/fixtures/*.rrproj),
//! the templates', a v1 project with every kind of part and a workspace;
//! each input is a seed with random edits, or random text. The strings the
//! readers hand to other code (paths, data URLs, `$INCLUDE` lines) are
//! thrown at their functions too.
//!
//! `RAPIDR_FUZZ_ITERS` sets the number of inputs each test tries (default
//! 20,000; a long run: `RAPIDR_FUZZ_ITERS=200000`), `RAPIDR_FUZZ_SEED` the
//! first seed. A failure names its seed. An input taking 20 s is a hang.

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use rapidr_project::{
    decode_base64, decode_data_url, defines_form, forms, import_v1, include_targets, is_v1_json, kind_for_source, named_files, normalize_path,
    sanitize_file_name, top_level_forms, Project, Workspace, TEMPLATES,
};

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
        match rx.recv_timeout(Duration::from_secs(20)) {
            Ok(Event::Started(seed)) => current = seed,
            Ok(Event::Finished) => break,
            Ok(Event::Panicked(seed, message)) => panic!("{what}: seed {seed} panicked: {message} (RAPIDR_FUZZ_SEED={seed} RAPIDR_FUZZ_ITERS=1)"),
            Err(mpsc::RecvTimeoutError::Timeout) => panic!("{what}: seed {current} hangs (RAPIDR_FUZZ_SEED={current} RAPIDR_FUZZ_ITERS=1)"),
            Err(mpsc::RecvTimeoutError::Disconnected) => panic!("{what}: the fuzzing thread stopped at seed {current}"),
        }
    }
}

/// Pieces an edit may insert: TOML's and JSON's edge cases, and the
/// project formats' own words.
const SNIPPETS: &[&str] = &[
    "\"", "'", "\"\"\"", "'''", "\\", "\\u", "\\u00", "\\ud800", "\\udc00\\ud800", "\\0", "\n", "\r\n", "\0", "[", "]", "[[", "]]", "{", "}", ",", ":", "=", ".", "#",
    "-", "+", "0", "-1", "1e400", "1e-400", "0x", "0xFFFFFFFFFFFFFFFFF", "18446744073709551616", "-9223372036854775809", "nan", "inf", "true", "false", "null", "NaN",
    "format = 2\n", "format = 3\n", "format = \"2\"\n", "format = -1\n", "name = ", "main = ", "main = \"../../etc/passwd\"\n", "main = \"C:\\\\x\"\n",
    "[[files]]\npath = ", "kind = \"form\"\n", "kind = \"nope\"\n", "[build]\n", "[run]\n", "[compat]\n", "[designer]\n", "[ai]\n", "[[breakpoints]]\n", "[[open_documents]]\n",
    "line = 4294967296\n", "line = -1\n", "\"rapidr_project\": 1", "\"rapidr_project\": 2", "\"forms\": [", "\"modules\": [", "\"assets\": [", "\"props\": {", "\"children\": [",
    "\"dataUrl\": \"data:;base64,", "\"dataUrl\": \"data:text/plain;base64,====", "\"dataUrl\": \"data:,%", "\"dataUrl\": \"data:,%zz", "\"name\": \"../../x\"", "\"name\": \"CON\"",
    "\"name\": \"\"", "\"name\": \"a/b\\\\c\"", "\"startupForm\": ", "\"source\": \"\\n$INCLUDE \\\"../x\\\"\"", "é", "€", "\u{1F600}", "\u{202e}", "\u{feff}", "\t", "    ",
];

/// Values that stay valid inside a TOML or JSON string (no raw quote or
/// backslash): paths that climb out or go absolute, device names, long and
/// odd text.
const STRING_VALUES: &[&str] = &[
    "", " ", "../../etc/passwd", "..\\\\..\\\\x", "/abs/path", "C:\\\\Windows\\\\x", "\\\\\\\\server\\\\share", "a/./b//c", "CON", "nul.txt", "x\\u0000y", "%00", "é€\u{1F600}",
    "\u{202e}txt.exe", "main.rr\\n[evil]", "a b", ".hidden", "...", "-", "$INCLUDE", "data:,%41", "data:;base64,@@@@", "data:text/plain;base64,SGk=",
];

/// Syntax-preserving edits: replaces a few string contents and numbers with
/// awkward values, so most inputs still read and the readers' later code
/// (paths, numbers, names) is what gets tested.
fn mutate_values(text: &str, rng: &mut Rng) -> String {
    let b = text.as_bytes();
    // (string contents, and runs of digits outside strings)
    let mut spots: Vec<(usize, usize, bool)> = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'"' => {
                let start = i + 1;
                i += 1;
                while i < b.len() && b[i] != b'"' && b[i] != b'\n' {
                    i += if b[i] == b'\\' { 2 } else { 1 };
                }
                if i < b.len() && b[i] == b'"' {
                    spots.push((start, i, true));
                }
                i += 1;
            }
            b'0'..=b'9' => {
                let start = i;
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
                spots.push((start, i, false));
            }
            _ => i += 1,
        }
    }
    if spots.is_empty() {
        return text.to_string();
    }
    const NUMBERS: &[&str] = &["0", "-1", "1", "255", "65536", "2147483648", "4294967296", "9223372036854775808", "99999999999999999999", "1e999", "0.5", "1_0"];
    let mut out = text.to_string();
    let mut chosen: Vec<(usize, usize, bool)> = (0..1 + rng.below(3)).map(|_| spots[rng.below(spots.len())]).collect();
    // (from the end, so the offsets before stay right; overlaps are skipped)
    chosen.sort_unstable();
    chosen.dedup();
    let mut limit = usize::MAX;
    for (start, end, is_string) in chosen.into_iter().rev() {
        if end > limit || !out.is_char_boundary(start) || !out.is_char_boundary(end) {
            continue;
        }
        let value = if is_string { STRING_VALUES[rng.below(STRING_VALUES.len())] } else { NUMBERS[rng.below(NUMBERS.len())] };
        out.replace_range(start..end, value);
        limit = start;
    }
    out
}

/// A seed with random edits: mostly gentle ones, so a good share of the
/// inputs still read.
fn mutate(text: &str, rng: &mut Rng) -> String {
    match rng.below(4) {
        0 | 1 => mutate_values(text, rng),
        2 => mutate_bytes(&mutate_values(text, rng), rng, 2),
        _ => mutate_bytes(text, rng, 6),
    }
}

fn mutate_bytes(text: &str, rng: &mut Rng, max_edits: usize) -> String {
    let mut bytes = text.as_bytes().to_vec();
    for _ in 0..1 + rng.below(max_edits) {
        let len = bytes.len();
        let at = rng.below(len + 1);
        match rng.below(6) {
            0 if len > 0 => {
                let end = (at + 1 + rng.below(40)).min(len);
                bytes.drain(at.min(len)..end);
            }
            1 | 2 => {
                let s = SNIPPETS[rng.below(SNIPPETS.len())];
                bytes.splice(at..at, s.bytes());
            }
            3 if len > 0 => {
                let from = rng.below(len);
                let end = (from + 1 + rng.below(80)).min(len);
                let slice = bytes[from..end].to_vec();
                bytes.splice(at..at, slice);
            }
            4 => bytes.insert(at, rng.below(256) as u8),
            _ => bytes.truncate(at),
        }
    }
    // (files are read as text: invalid UTF-8 never gets as far as a reader)
    String::from_utf8_lossy(&bytes).into_owned()
}

/// A random string from the snippets and some ASCII.
fn random_text(rng: &mut Rng) -> String {
    let mut s = String::new();
    for _ in 0..rng.below(40) {
        if rng.below(2) == 0 {
            s.push_str(SNIPPETS[rng.below(SNIPPETS.len())]);
        } else {
            s.push((b' ' + rng.below(95) as u8) as char);
        }
    }
    s
}

const V1_SEED: &str = r##"{
  "rapidr_project": 1,
  "name": "Demo",
  "modules": [
    { "id": "m1", "name": "util", "source": "SUB Hello\n  PRINT \"hi\"\nEND SUB" },
    { "id": "m2", "name": "../evil/x.inc", "source": "$INCLUDE \"../x\"" }
  ],
  "assets": [
    { "name": "hello.txt", "mime": "text/plain", "dataUrl": "data:text/plain;base64,SGVsbG8sIHdvcmxkIQ==" },
    { "name": "note.txt", "mime": "text/plain", "dataUrl": "data:,a%20b%2Cc" },
    { "name": "..\\..\\CON", "mime": "text/plain", "dataUrl": "data:,x" },
    { "name": "remote.png", "mime": "image/png", "dataUrl": "https://example.com/x.png" }
  ],
  "forms": [
    { "id": "f1", "name": "Form1",
      "props": { "caption": "Say \"hi\"", "width": 480, "height": 320, "startX": 100, "borderstyle": 2, "isStartup": true,
                 "hint": null, "zoom": 1.5, "tag": "12", "color": "&HFF00FF" },
      "children": [
        { "name": "Button1", "type": "RButton",
          "props": { "caption": "OK", "left": 10, "top": 20, "text": 5, "enabled": false },
          "code": { "handlers": { "OnClick": "Button1_Click", "OnMouseDown": "" } } },
        { "name": "Label1", "type": "RLabel", "props": { "caption": "1234", "parent": "Panel1" }, "code": { "handlers": {} } }
      ],
      "code": { "handlers": { "OnClose": "Form1_Close" }, "source": "\nSUB Button1_Click\n  ShowMessage \"clicked\"\nEND SUB\n" } },
    { "id": "f2", "name": "About", "props": { "caption": "About", "width": 200 }, "children": [], "code": { "handlers": {}, "source": "" } }
  ],
  "startupForm": "f2",
  "theme": "dark"
}"##;

const WORKSPACE_SEED: &str = r#"
active_document = "main.rr"
watches = ["x", "a(1) + 2"]
dock_layout = "{\"left\":[\"Project\"]}"

[[open_documents]]
path = "main.rr"
caret_line = 4
caret_column = 9
scroll_line = 1

[[breakpoints]]
file = "main.rr"
line = 12
condition = "i > 3"
hit_condition = ">= 10"
log_message = "i = {i}"
enabled = true
"#;

fn project_seeds() -> Vec<String> {
    let mut seeds = Vec::new();
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    let mut stack = vec![fixtures];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rrproj") {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    seeds.push(text);
                }
            }
        }
    }
    for template in TEMPLATES {
        let (project, _) = Project::new_from_template("Demo", template).expect("a template");
        seeds.push(project.to_toml());
    }
    seeds.sort();
    seeds
}

/// Reads one text as a project the way Studio does (`rapidr_project::open`
/// decides by content), and uses what comes out.
/// True when the text was accepted (so a test can tell its inputs get past
/// the first error: a fuzz test whose inputs are all refused tests nothing).
fn read_project(text: &str) -> bool {
    let accepted = Project::from_toml(text);
    let ok = accepted.is_ok();
    if let Ok(p) = accepted {
        // what a project read from a file must survive: saving it back
        let again = p.to_toml();
        let p2 = Project::from_toml(&again).expect("a saved project reads back");
        assert_eq!(p2.name, p.name);
        let _ = (p.file_count(), p.compat_mode(), p.contains("main.rr"), p.files_of_kind(rapidr_project::FileKind::Form).len());
        for i in 0..p.file_count().min(50) {
            let _ = p.file(i);
        }
    }
    let _ = is_v1_json(text);
    ok
}

fn read_v1(text: &str) -> bool {
    let _ = is_v1_json(text);
    let accepted = import_v1(text);
    let ok = accepted.is_ok();
    if let Ok(imported) = accepted {
        for (path, _) in &imported.files {
            assert!(
                !path.is_empty() && !path.starts_with('/') && !path.contains('\\') && !path.contains('\0') && !path.split('/').any(|c| c == ".." || c.is_empty() || c == "."),
                "an imported file names a path outside the project folder: {path:?}"
            );
        }
        let _ = imported.project.to_toml();
        let _ = imported.file_text("main.rr");
    }
    ok
}

/// At least `percent` % of `tried` inputs were accepted (checked only on a
/// real run, not when one seed is replayed).
fn assert_reach(what: &str, accepted: &AtomicUsize, tried: u64, percent: usize) {
    if tried < 1000 {
        return;
    }
    let got = accepted.load(Ordering::Relaxed);
    eprintln!("{what}: {got} of {tried} mutated inputs were accepted ({} %)", got * 100 / tried as usize);
    assert!(got * 100 >= tried as usize * percent, "{what}: only {got} of {tried} mutated inputs were accepted (wanted {percent} %): the edits are too destructive to test the readers");
}

#[test]
fn mutated_rrproj_files_never_panic_or_hang() {
    let seeds = project_seeds();
    assert!(seeds.len() >= TEMPLATES.len());
    for seed in &seeds {
        assert!(Project::from_toml(seed).is_ok(), "a seed reads");
    }
    let accepted = std::sync::Arc::new(AtomicUsize::new(0));
    let counter = accepted.clone();
    run_guarded("rrproj", 20_000, move |rng| {
        let text = mutate(&seeds[rng.below(seeds.len())], rng);
        if read_project(&text) {
            counter.fetch_add(1, Ordering::Relaxed);
        }
    });
    assert_reach("rrproj", &accepted, iterations(20_000), 10);
}

#[test]
fn random_text_as_a_project_never_panics_or_hangs() {
    run_guarded("random rrproj text", 20_000, |rng| {
        let text = random_text(rng);
        read_project(&text);
        let _ = Workspace::from_toml(&text);
    });
}

#[test]
fn mutated_v1_json_projects_never_panic_or_hang() {
    assert!(import_v1(V1_SEED).is_ok(), "the seed imports");
    let accepted = std::sync::Arc::new(AtomicUsize::new(0));
    let counter = accepted.clone();
    run_guarded("v1 json", 20_000, move |rng| {
        let text = mutate(V1_SEED, rng);
        if read_v1(&text) {
            counter.fetch_add(1, Ordering::Relaxed);
        }
    });
    assert_reach("v1 json", &accepted, iterations(20_000), 25);
}

#[test]
fn mutated_workspace_files_never_panic_or_hang() {
    assert!(Workspace::from_toml(WORKSPACE_SEED).is_ok(), "the seed reads");
    run_guarded("workspace", 20_000, |rng| {
        let text = mutate(WORKSPACE_SEED, rng);
        if let Ok(w) = Workspace::from_toml(&text) {
            let again = w.to_toml();
            assert_eq!(Workspace::from_toml(&again).expect("a saved workspace reads back"), w);
        }
    });
}

#[test]
fn strings_a_project_hands_to_other_code_never_panic() {
    run_guarded("project strings", 20_000, |rng| {
        let s = random_text(rng);
        let _ = decode_base64(&s);
        let _ = decode_data_url(&s);
        let n = normalize_path(&s);
        // (`/` separators only, no empty or `.` steps; leading `..` steps
        // stay: a file beside the project)
        assert!(!n.contains('\\') && !n.contains("//") && !n.split('/').any(|c| c == "."), "normalize_path({s:?}) = {n:?}");
        let safe = sanitize_file_name(&s, "fallback");
        assert!(!safe.is_empty() && !safe.contains('/') && !safe.contains('\\') && !safe.contains(".."), "sanitize_file_name({s:?}) = {safe:?}");
        let _ = include_targets(&s);
        let _ = named_files(&s);
        let _ = top_level_forms(&s);
        let _ = defines_form(&s);
        let _ = kind_for_source("x.rr", &s);
        let other = random_text(rng);
        let _ = forms::uses_rapidq_names(&s, "a.bas");
        let _ = forms::line_end(&s);
        let _ = forms::includes(&s, &other);
        let _ = forms::include_insertion(&s, &other);
        let _ = forms::include_rename(&s, &other, "z.rr");
        let _ = forms::include_line(&s, &other);
        let _ = forms::with_include(&s, &other);
        let _ = rapidr_project::ship::place(&s);
        let _ = rapidr_project::ship::is_catalog(&s);
        let _ = rapidr_project::ship::uses_translations(&s);
    });
}
