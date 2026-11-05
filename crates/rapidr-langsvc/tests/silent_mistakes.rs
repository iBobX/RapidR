//! The mistakes that used to pass in silence (the ERRORS lane, 2026-11-05):
//! each is an error or a warning now, with its file and line and the fix,
//! the same in `rapidr check`, Studio's Problems and the AI tools (all of
//! them ask the language service).
//!
//! * A SUB that ends with a bare `END` in an `$INCLUDE`d file took the
//!   including file's lines after the `$INCLUDE` into its body: the program
//!   ran, printed "start" and ended (ShopInvoices' `SUB StartApp`).
//! * `CREATE Timer AS RTimer` made every TIMER of the program the component
//!   (RapidR Web Server's issue 11).
//! * A file with Windows-1252 bytes beside UTF-8 text was read whole as
//!   Windows-1252: its Chinese showed as `ä¸­æ–‡`.
//!
//! What RapidQ's RC.EXE does with each, in the Windows VM, is kept for
//! classic programs: its own words first (`Missing END SUB or END
//! FUNCTION`, `Identifier TIMER already used, try another name`).

use std::fs;
use std::path::{Path, PathBuf};

use rapidr_diagnostics::Severity;
use rapidr_langsvc::{Analysis, FileDiagnostic, LineIndex, Options};

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rapidr-langsvc-silent-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

/// The program `main` (its files written to a scratch folder first): each
/// diagnostic as (file name, line, column, severity, message).
fn check(name: &str, files: &[(&str, &[u8])]) -> Vec<(String, usize, usize, Severity, String)> {
    let dir = scratch(name);
    for (f, bytes) in files {
        fs::write(dir.join(f), bytes).unwrap();
    }
    let main = dir.join(files[0].0);
    let mut a = Analysis::new(Options::default());
    a.update(main.clone(), rapidr_preprocessor::read_source(&main).unwrap());
    let out = a
        .diagnostics(&main)
        .iter()
        .map(|d: &FileDiagnostic| {
            let text = rapidr_preprocessor::read_source(&d.file).unwrap();
            let (l, c) = LineIndex::new(&text).line_col(d.start);
            (d.file.file_name().unwrap().to_string_lossy().to_string(), l + 1, c + 1, d.severity, d.message.clone())
        })
        .collect();
    let _ = fs::remove_dir_all(&dir);
    out
}

fn errors(d: &[(String, usize, usize, Severity, String)]) -> Vec<(String, usize, String)> {
    d.iter().filter(|x| x.3 == Severity::Error).map(|x| (x.0.clone(), x.1, x.4.clone())).collect()
}

#[test]
fn a_sub_ending_with_end_in_an_include_is_an_error_at_its_line() {
    let b: &[u8] = b"DECLARE SUB Go\nSUB Go\n  PRINT \"in Go\"\nEND\n";
    let m: &[u8] = b"PRINT \"start\"\n$INCLUDE \"b.rr\"\nGo\nPRINT \"after Go\"\n";
    let d = check("include-end", &[("m.rr", m), ("b.rr", b)]);
    assert_eq!(
        errors(&d),
        [("b.rr".to_string(), 2, "SUB Go (b.rr:2) has no END SUB: a bare END (b.rr:4) ends the program; write END SUB there".to_string())],
        "the one error, in b.rr at the SUB; m.rr's lines after the $INCLUDE are its own code again: {d:?}"
    );
    // (the classic program: RC.EXE's "! WARNING: Missing END SUB or END
    // FUNCTION", which builds no program — so an error — then RapidR's words)
    let d = check("include-end-bas", &[("m.bas", b"PRINT \"start\"\n$INCLUDE \"b.inc\"\nGo\nPRINT \"after Go\"\n"), ("b.inc", b)]);
    assert_eq!(
        errors(&d),
        [("b.inc".to_string(), 2, "Missing END SUB or END FUNCTION — SUB Go (b.inc:2) has no END SUB: a bare END (b.inc:4) ends the program; write END SUB there".to_string())],
        "{d:?}"
    );
    // (the same SUB closed: nothing at all)
    let d = check("include-end-ok", &[("m.rr", m), ("b.rr", b"DECLARE SUB Go\nSUB Go\n  PRINT \"in Go\"\nEND SUB\n")]);
    assert!(errors(&d).is_empty(), "{d:?}");
}

#[test]
fn every_block_left_open_says_which_where_and_the_fix() {
    let cases: &[(&str, &str)] = &[
        ("FOR i = 1 TO 2\n  PRINT i\n", "FOR i (main.rr:1) has no NEXT: write NEXT where it ends"),
        ("WHILE 0\n  PRINT 1\n", "WHILE (main.rr:1) has no WEND: write WEND where it ends"),
        ("DO\n  EXIT DO\n", "DO (main.rr:1) has no LOOP: write LOOP where it ends"),
        ("IF 1 THEN\n  PRINT 1\n", "IF (main.rr:1) has no END IF: write END IF where it ends"),
        ("SELECT CASE 1\n  CASE 1\n    PRINT 1\n", "SELECT CASE (main.rr:1) has no END SELECT: write END SELECT where it ends"),
        ("FUNCTION F AS INTEGER\n  Result = 1\n", "FUNCTION F (main.rr:1) has no END FUNCTION: write END FUNCTION where it ends"),
        ("CREATE T AS RTimer\n  Interval = 100\n", "CREATE T (main.rr:1) has no END CREATE: write END CREATE where it ends"),
        ("TYPE TP\n  x AS INTEGER\n", "TYPE TP (main.rr:1) has no END TYPE: write END TYPE where it ends"),
        ("WITH Screen\n  PRINT .Width\n", "WITH Screen (main.rr:1) has no END WITH: write END WITH where it ends"),
        ("SUB A\n  PRINT 1\nSUB B\n  PRINT 2\nEND SUB\n", "SUB A (main.rr:1) has no END SUB before SUB B (main.rr:3): write END SUB above it"),
    ];
    for (program, want) in cases {
        let d = check("blocks", &[("main.rr", program.as_bytes())]);
        assert_eq!(errors(&d), [("main.rr".to_string(), 1, want.to_string())], "{program:?}: {d:?}");
    }
    // (a block opened in an $INCLUDEd file ends there; the file that
    // includes it goes on as its own code)
    let d = check("blocks-include", &[("main.rr", b"PRINT \"start\"\n$INCLUDE \"part.rr\"\nPRINT \"after\"\n"), ("part.rr", b"IF 1 THEN\n  PRINT \"in IF\"\n")]);
    assert_eq!(errors(&d), [("part.rr".to_string(), 1, "IF (part.rr:1) has no END IF in its file: a block ends in the file it starts in — write END IF there".to_string())], "{d:?}");
    // (an $INCLUDE inside a block is part of it)
    let d = check("blocks-inside", &[("main.rr", b"SUB Main\n  $INCLUDE \"body.rr\"\nEND SUB\nMain\n"), ("body.rr", b"  PRINT \"body\"\n")]);
    assert!(errors(&d).is_empty(), "{d:?}");
    // (RC.EXE takes a WITH or a CREATE left open in a classic program, and
    // runs it: so does RapidR)
    let d = check("blocks-classic", &[("main.bas", b"CREATE T AS QTIMER\n  Interval = 100\n")]);
    assert!(errors(&d).is_empty(), "{d:?}");
}

#[test]
fn a_global_named_like_a_builtin_is_an_error() {
    let form = b"CREATE F AS RForm\n    CREATE Timer AS RTimer: Interval = 100: END CREATE\nEND CREATE\nDIM t AS DOUBLE\nt = TIMER\nPRINT t; TIMER\n";
    let d = check("timer-rr", &[("main.rr", form)]);
    assert_eq!(errors(&d), [("main.rr".to_string(), 2, "Timer is the name of the builtin TIMER: the program can't tell them apart, so name the component PollTimer".to_string())], "{d:?}");
    // (a classic program: RC.EXE's words — it refuses the name — then RapidR's)
    let d = check("timer-bas", &[("main.bas", b"CREATE Timer AS QTIMER\n  Interval = 100\nEND CREATE\nPRINT TIMER\n")]);
    assert_eq!(errors(&d), [("main.bas".to_string(), 1, "Identifier TIMER already used, try another name — Timer is the name of the builtin TIMER: the program can't tell them apart, so name the component PollTimer".to_string())], "{d:?}");
    let d = check("left-sub", &[("main.bas", b"SUB Left$ (s AS STRING)\n  PRINT s\nEND SUB\nPRINT LEFT$(\"abc\", 1)\n")]);
    assert!(errors(&d).iter().any(|e| e.2.starts_with("LEFT$ identifier already in use, try another name — ")), "{d:?}");
    let d = check("len-const", &[("main.bas", b"CONST Len = 3\nPRINT Len\n")]);
    assert!(errors(&d).iter().any(|e| e.2.starts_with("CONST variable LEN already defined — ")), "{d:?}");
    // (a variable: when the builtin is read through it)
    let d = check("len-var", &[("main.rr", b"DIM Len AS INTEGER\nLen = 5\nPRINT LEN(\"abc\")\n")]);
    assert_eq!(errors(&d), [("main.rr".to_string(), 1, "Len is the name of the builtin LEN: the program can't tell them apart, so name the variable Length".to_string())], "{d:?}");
    // (and not otherwise; nor the names RC.EXE takes: LEFT, RND, BEEP; nor a CONST PI)
    for (file, program) in [
        ("main.bas", "DIM Log AS STRING\nLog = \"a\"\nPRINT Log\n"),
        ("main.bas", "DIM Left AS INTEGER\nLeft = 3\nPRINT Left\n"),
        ("main.bas", "DIM Rnd AS INTEGER\nRnd = 3\n"),
        ("main.bas", "SUB Beep\n  PRINT \"mine\"\nEND SUB\nBeep\n"),
        ("main.rr", "CONST PI = 3.14159265\nPRINT PI\n"),
        ("main.rr", "DIM e AS INTEGER\ne = 2\nPRINT e\n"),
        ("main.rr", "SUB S(Len AS INTEGER)\n  PRINT Len\nEND SUB\nS 1\n"),
    ] {
        let d = check("names-ok", &[(file, program.as_bytes())]);
        assert!(errors(&d).is_empty(), "{file} {program:?}: {d:?}");
    }
}

#[test]
fn a_file_that_isnt_utf8_is_said_with_its_line() {
    // (ShopInvoices' ui_logic.rr: "Español" in Latin-1 on one line, "中文" in
    // UTF-8 on another — read whole as Windows-1252, the Chinese shows wrong)
    let mixed: &[u8] = b"PRINT \"one\"\nPRINT \"Espa\xf1ol\"\nPRINT \"\xe4\xb8\xad\xe6\x96\x87\"\n";
    let d = check("mixed", &[("main.rr", mixed)]);
    let w: Vec<_> = d.iter().filter(|x| x.3 == Severity::Warning).collect();
    assert_eq!(w.len(), 1, "{d:?}");
    assert_eq!((w[0].0.as_str(), w[0].1), ("main.rr", 2));
    assert_eq!(w[0].4, "main.rr mixes two encodings: byte 0xF1 on line 2 isn't UTF-8 (Windows-1252 reads it as ñ), so the whole file is read as Windows-1252 and its UTF-8 characters show wrong — retype that character and save the file as UTF-8");
    // (the UTF-8 before the byte: each of its bytes is a character of the
    // text read as Windows-1252 — the place is still the byte's)
    let d = check("mixed-after", &[("main.rr", b"PRINT \"\xe4\xb8\xad\xe6\x96\x87\"\nPRINT \"Espa\xf1ol\"\n")]);
    assert!(d.iter().any(|x| x.3 == Severity::Warning && (x.1, x.2) == (2, 12) && x.4.contains("byte 0xF1 on line 2 isn't UTF-8 (Windows-1252 reads it as ñ)")), "{d:?}");
    // (a .rr file in Windows-1252 only: said too; a classic one is RapidQ's
    // ANSI, as it always was — unless it mixes the two)
    let d = check("ansi-rr", &[("main.rr", b"PRINT \"Espa\xf1ol\"\n")]);
    assert!(d.iter().any(|x| x.3 == Severity::Warning && x.4 == "main.rr isn't UTF-8: byte 0xF1 on line 1 is read as Windows-1252 (ñ) — save the file as UTF-8 (in a text editor: Save with Encoding, UTF-8)"), "{d:?}");
    let d = check("ansi-bas", &[("main.bas", b"PRINT \"Espa\xf1ol\"\n")]);
    assert!(d.is_empty(), "{d:?}");
    let d = check("mixed-inc", &[("main.bas", b"$INCLUDE \"t.inc\"\nPRINT T$\n"), ("t.inc", b"' \xe4\xb8\xad\nCONST T$ = \"Espa\xf1ol\"\n")]);
    assert!(d.iter().any(|x| x.0 == "t.inc" && x.1 == 2 && x.3 == Severity::Warning && x.4.starts_with("t.inc mixes two encodings: byte 0xF1 on line 2")), "{d:?}");
}

#[test]
fn the_new_messages_are_in_spanish_too() {
    use rapidr_diagnostics::messages::localize;
    assert_eq!(
        localize("SUB Go (b.rr:2) has no END SUB: a bare END (b.rr:4) ends the program; write END SUB there", "es"),
        "SUB Go (b.rr:2) no tiene END SUB: un END solo (b.rr:4) termina el programa; escribe END SUB ahí"
    );
    assert_eq!(
        localize("Identifier TIMER already used, try another name — Timer is the name of the builtin TIMER: the program can't tell them apart, so name the component PollTimer", "es"),
        "El identificador TIMER ya se usó; prueba con otro nombre — Timer es el nombre de TIMER, que RapidR ya trae: el programa no puede distinguirlos, así que llama al componente PollTimer"
    );
    assert!(localize("main.rr isn't UTF-8: byte 0xF1 on line 1 is read as Windows-1252 (ñ) — save the file as UTF-8 (in a text editor: Save with Encoding, UTF-8)", "es").starts_with("main.rr no es UTF-8: el byte 0xF1 de la línea 1"));
}

/// RapidQ's examples (`$RAPIDQ_DIR`, else `~/Downloads/Rapidq`; skipped
/// when there is none): none gets one of the new errors but the two whose
/// block RapidR found open before (a warning then): Math/quest.bas, which
/// RC.EXE refuses too (`Expected AS but got ,`), and mysql/rqlibsql/
/// mySQL_API.bas, which already had errors here.
#[test]
fn rapidq_examples_get_no_new_errors() {
    let Some(dir) = std::env::var_os("RAPIDQ_DIR").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join("Downloads/Rapidq"))) else { return };
    let examples = dir.join("examples");
    if !examples.is_dir() {
        return;
    }
    fn programs(d: &Path, out: &mut Vec<PathBuf>) {
        for e in fs::read_dir(d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                programs(&p, out);
            } else if p.extension().and_then(|x| x.to_str()).is_some_and(|x| ["bas", "rqb", "rqw"].iter().any(|e| e.eq_ignore_ascii_case(x))) {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    programs(&examples, &mut files);
    files.sort();
    let new = |m: &str| m.starts_with("Missing ") || m.contains(") has no ") || m.contains(" is the name of the builtin ");
    let mut bad = Vec::new();
    for p in &files {
        let Ok(text) = rapidr_preprocessor::read_source(p) else { continue };
        let mut a = Analysis::new(Options::default());
        a.update(p.clone(), text);
        for d in a.diagnostics(p).iter().filter(|d| d.severity == Severity::Error && new(&d.message)) {
            let rel = p.strip_prefix(&examples).unwrap_or(p).to_string_lossy().replace('\\', "/");
            if !["Math/quest.bas", "mysql/rqlibsql/mySQL_API.bas"].contains(&rel.as_str()) {
                bad.push(format!("{rel}: {}", d.message));
            }
        }
    }
    assert!(files.len() > 300, "only {} examples in {}", files.len(), examples.display());
    assert!(bad.is_empty(), "new errors in RapidQ's examples:\n{}", bad.join("\n"));
}
