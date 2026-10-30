//! Syntax from other languages, end to end (docs/manual/diagnostics.md,
//! "Coming from another language?"): a program is analysed as Studio's
//! Problems pane analyses it, and each habit's error carries its hint —
//! an ADDITIONAL line, the compiler's own message unchanged. What RapidQ
//! compiles (`+=`, `++`, `==`, `print(x)`) has no error and no hint; what
//! RC.EXE only warns about (a block never closed) is a warning here, and
//! `&&` and `console.log`, which compile and run wrongly, are warnings with
//! a hint too.

use std::path::Path;

use rapidr_langsvc::{Analysis, FileDiagnostic, Options, Severity};

fn problems(program: &str) -> Vec<FileDiagnostic> {
    let mut a = Analysis::new(Options::default());
    let file = Path::new("/project/main.rr");
    a.update(file, program);
    a.diagnostics(file)
}

/// The first error's (message, hint id).
fn first_error(program: &str) -> (String, Option<&'static str>) {
    let d = problems(program).into_iter().find(|d| d.severity == Severity::Error).unwrap_or_else(|| panic!("no error in {program:?}"));
    (d.message.clone(), d.hint.map(|h| h.id))
}

fn warning_ids(program: &str) -> Vec<(String, Option<&'static str>)> {
    problems(program).into_iter().filter(|d| d.severity == Severity::Warning).map(|d| (d.message, d.hint.map(|h| h.id))).collect()
}

#[test]
fn set_has_a_hint_under_the_compilers_own_message() {
    let (message, hint) = first_error("DIM x AS INTEGER\nSET x = 1\nPRINT x\n");
    assert_eq!(message, "Syntax error in SET statement", "the message is RC.EXE's, unchanged");
    assert_eq!(hint, Some("set"));
    let d = problems("DIM x AS INTEGER\nSET x = 1\n").into_iter().find(|d| d.severity == Severity::Error).unwrap();
    assert_eq!(d.two_lines(), "Syntax error in SET statement\nhint: RapidR has no SET: write x = 1 (the name, =, then the value)");
    assert!(d.one_line().starts_with("Syntax error in SET statement — RapidR has no SET: write x = 1"));
}

#[test]
fn each_habit_gets_its_hint() {
    let cases: &[(&str, &str)] = &[
        ("DIM a AS INTEGER\na = 7 % 3\n", "mod"),
        ("DIM x AS INTEGER\nx %= 3\n", "compound"),
        ("DIM x AS INTEGER\nx ^= 2\n", "compound"),
        ("DIM i AS INTEGER\nFOR i IN range(10)\n  PRINT i\nNEXT\n", "for-foreign"),
        ("DIM i AS INTEGER\nfor (i = 0; i < 10; i++)\n  PRINT i\nNEXT\n", "for-foreign"),
        ("DIM a AS INTEGER\nIF a = 1 THEN\n  PRINT 1\nelif a = 2 THEN\n  PRINT 2\nEND IF\n", "elseif"),
        ("DIM a AS INTEGER\nIF a = 1 {\n  PRINT 1\n}\n", "braces"),
        ("DIM a AS INTEGER\nIF a = 1:\n  PRINT 1\nEND IF\n", "colon"),
        ("def hello():\n  PRINT 1\nEND SUB\n", "function"),
        ("function hello(a) {\n  PRINT 1\n}\n", "function"),
        ("DIM a AS INTEGER\nIF a != 1 THEN PRINT 1\n", "ne"),
        ("DIM a AS INTEGER\nIF a || 1 THEN PRINT 1\n", "or"),
        ("var x = 1\nPRINT x\n", "var"),
        ("let x = 1\nPRINT x\n", "let"),
        ("DIM a AS INTEGER\nIF a = 1 THEN\n  PRINT 1\nENDIF\n", "endif"),
        ("DIM i AS INTEGER\nFOR i = 1 TO 3\n  PRINT i\nEND FOR\n", "end-loop"),
        ("DIM a AS INTEGER\na = 2 ** 3\n", "pow"),
        ("DIM a AS INTEGER\na = 7 // 2\n", "intdiv"),
        ("DIM a AS INTEGER\na = 1;\n", "semicolon"),
        ("DIM a AS INTEGER\nIF a THEN break\n", "break"),
        ("DIM s AS STRING\ns = 'hello'\nPRINT s\n", "squote"),
        ("#include <stdio.h>\nPRINT 1\n", "include"),
        ("DIM a AS INTEGER\nswitch (a) {\n  case 1:\n}\n", "switch"),
        ("DIM s AS STRING\ns = toupper(\"a\")\n", "fn-toupper"),
        ("DIM a AS INTEGER\nIF a = 1 THEN PRINT 1\nELSE PRINT 2\n", "else-line"),
        ("DIM a AS INTEGER\nIF a = 1\n  PRINT 1\nEND IF\n", "if-then"),
    ];
    for (program, want) in cases {
        let (message, hint) = first_error(program);
        assert_eq!(hint, Some(*want), "{program:?}: the error {message:?}");
    }
}

#[test]
fn what_rapidq_compiles_has_no_error_and_no_hint() {
    for program in [
        "DIM x AS INTEGER\nx = 1\nx += 1\nx -= 1\nx *= 2\nx /= 2\nx++\nPRINT x\n",
        "DIM a AS INTEGER\na = 1\nIF a == 1 THEN PRINT 1\n",
        "print(\"hello\")\n",
        "DIM a AS INTEGER\na = 7 MOD 3\nPRINT a\n",
        "DIM i AS INTEGER\nFOR i = 1 TO 3 STEP 1\n  PRINT i\nNEXT i\n",
    ] {
        let d = problems(program);
        assert!(d.iter().all(|d| d.severity != Severity::Error), "{program:?}: {d:?}");
        assert!(d.iter().all(|d| d.hint.is_none()), "{program:?}: {d:?}");
    }
}

#[test]
fn the_manual_lists_every_hint_by_its_id() {
    // (docs/manual/language.md, "Coming from another language?": the table
    // is the hints' documentation — a hint without a row is a drift)
    let page = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/manual/language.md")).unwrap();
    let missing: Vec<&str> = rapidr_langsvc::foreign::HINTS.iter().map(|h| h.id).filter(|id| !page.contains(&format!("`hint.{id}`"))).collect();
    assert!(missing.is_empty(), "language.md has no row for: {missing:?}");
    assert!(page.contains("hint.fn-str"), "the functions' row names the pattern");
}

#[test]
fn strings_and_comments_hold_no_habits() {
    let d = problems("DIM s AS STRING\ns = \"a != b\" ' x != y\nPRINT s\n");
    assert!(d.is_empty(), "{d:?}");
}

#[test]
fn and_and_compiles_so_it_is_a_warning_with_the_hint() {
    // (RapidQ takes && as two & — text joining —: it compiles, the test is
    // never true. Compatibility kept; the warning and its fix added)
    let w = warning_ids("DIM x AS INTEGER\nx = 3\nIF x = 3 && x = 3 THEN PRINT \"yes\" ELSE PRINT \"no\"\n");
    assert_eq!(w.len(), 1, "{w:?}");
    assert_eq!(w[0].1, Some("and-and"));
    assert!(w[0].0.starts_with("&& is not a logical AND"), "{w:?}");
    assert!(rapidr_langsvc::foreign::hint("and-and").unwrap().text().starts_with("RapidR uses AND and OR"));
    // (in a string, a comment, or before a hex number: not one)
    assert!(warning_ids("DIM s AS STRING\ns = \"Q&&A\" ' a && b\n").is_empty());
    assert!(warning_ids("DIM n AS INTEGER\nn = 1 &&HFF\n").is_empty());
}

#[test]
fn console_log_compiles_so_it_is_a_warning_with_the_hint() {
    let w = warning_ids("console.log(\"hi\")\n");
    assert_eq!(w.len(), 1, "{w:?}");
    assert_eq!(w[0].1, Some("console-log"));
    assert!(w[0].0.starts_with("console.log is not RapidR"), "{w:?}");
    assert!(rapidr_langsvc::foreign::hint("console-log").unwrap().text().starts_with("use PRINT"));
    assert_eq!(warning_ids("System.out.println(\"hi\")\n").len(), 1);
    // (a program with a name of its own: no quoting of JavaScript)
    assert!(warning_ids("DIM console AS INTEGER\nconsole.log(\"x\")\n").iter().all(|w| w.1 != Some("console-log")));
}

#[test]
fn a_block_never_closed_is_rc_exes_warning_and_the_program_compiles() {
    let cases: &[(&str, &str, &str)] = &[
        ("DIM i AS INTEGER\nFOR i = 1 TO 3\n  PRINT i\n", "Missing NEXT statement", "missing-next"),
        ("DIM i AS INTEGER\nWHILE i < 3\n  i = i + 1\n", "Missing WEND statement", "missing-wend"),
        ("DIM i AS INTEGER\nDO\n  i = i + 1\n", "Missing LOOP statement", "missing-loop"),
        ("DIM a AS INTEGER\nIF a THEN\n  PRINT 1\n", "Missing END IF statement", "missing-end-if"),
        ("DIM a AS INTEGER\nSELECT CASE a\n  CASE 1\n    PRINT 1\n", "Missing END SELECT statement", "missing-end-select"),
        ("DIM a AS INTEGER\nIF a THEN\n  PRINT 1\nELSE IF a = 2 THEN\n  PRINT 2\nEND IF\n", "Missing END IF statement", "else-if"),
    ];
    for (program, message, hint) in cases {
        let d = problems(program);
        assert!(d.iter().all(|d| d.severity != Severity::Error), "{program:?} compiles (RC.EXE only warns): {d:?}");
        let w = d.iter().find(|d| d.message == *message).unwrap_or_else(|| panic!("{program:?}: {d:?}"));
        assert_eq!(w.severity, Severity::Warning);
        assert_eq!(w.hint.map(|h| h.id), Some(*hint), "{program:?}");
    }
    // (closed blocks, one-line IFs and one-line loops: nothing)
    let ok = "DIM i AS INTEGER\nFOR i = 1 TO 3 : PRINT i : NEXT i\nIF i THEN PRINT 1 ELSE PRINT 2\nWHILE i < 5 : i = i + 1 : WEND\nSELECT CASE i\n  CASE 5\n    PRINT 5\nEND SELECT\n";
    assert!(problems(ok).is_empty(), "{:?}", problems(ok));
}

#[test]
fn the_examples_and_conformance_programs_have_no_missing_warnings() {
    // (a block the parser sees as never closed in a program that runs would
    // be a false warning: the repository's own programs are the check)
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut bad: Vec<String> = Vec::new();
    for dir in ["examples", "tests/conformance/cases"] {
        let mut stack = vec![root.join(dir)];
        while let Some(d) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "rr" || x == "bas") {
                    let Ok(text) = std::fs::read_to_string(&p) else { continue };
                    // (a case that must not compile is allowed its warnings)
                    if p.with_extension("expected-error").exists() {
                        continue;
                    }
                    let mut a = Analysis::new(Options::default());
                    a.update(p.clone(), text);
                    for d in a.diagnostics(&p) {
                        if d.message.starts_with("Missing ") && d.message.ends_with(" statement") && d.file == p {
                            bad.push(format!("{}: {}", p.display(), d.message));
                        }
                    }
                }
            }
        }
    }
    assert!(bad.is_empty(), "false 'Missing … statement' warnings:\n{}", bad.join("\n"));
}
