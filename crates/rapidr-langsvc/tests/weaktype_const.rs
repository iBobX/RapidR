//! `$OPTION WEAKTYPE ON` and a CONST written again (a classic program, as
//! RC.EXE has it; the WEAKTYPE-CONST lane, 2026-11-07).
//!
//! RC.EXE, probed in the Windows VM: with the option on at the line of the
//! second CONST there is no error and the last value wins; with it off
//! (the default) any second CONST of a name is `CONST variable A already
//! defined`, in the same file or another, `Public Const` and `Private
//! Const` too. A VB port such as WIN32API.INC starts with the
//! option on and ends with it off, and repeats some of RAPIDQ.INC's
//! constants: `$INCLUDE "rapidq.inc"` then the port compiles, the other
//! order stops at RAPIDQ.INC's first repeated name. The same answers in
//! `rapidr check`, Studio's Problems and the AI tools (the language
//! service); the run itself is tests/conformance/cases/weaktype_*.

use std::fs;
use std::path::PathBuf;

use rapidr_diagnostics::Severity;
use rapidr_langsvc::{Analysis, LineIndex, Options};

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rapidr-langsvc-weaktype-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

/// The program `files[0]` (all the files written to a scratch folder
/// first): each error as (file name, line, message).
fn errors(name: &str, files: &[(&str, &str)]) -> Vec<(String, usize, String)> {
    let dir = scratch(name);
    for (f, text) in files {
        fs::write(dir.join(f), text).unwrap();
    }
    let main = dir.join(files[0].0);
    let mut a = Analysis::new(Options::default());
    a.update(main.clone(), rapidr_preprocessor::read_source(&main).unwrap());
    let out = a
        .diagnostics(&main)
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| {
            let text = rapidr_preprocessor::read_source(&d.file).unwrap();
            let (l, _) = LineIndex::new(&text).line_col(d.start);
            (d.file.file_name().unwrap().to_string_lossy().to_string(), l + 1, d.message.clone())
        })
        .collect();
    let _ = fs::remove_dir_all(&dir);
    out
}

/// The errors' first words (the message goes on with where the first one
/// is and how to fix it).
fn heads(e: &[(String, usize, String)]) -> Vec<(String, usize, String)> {
    e.iter().map(|(f, l, m)| (f.clone(), *l, m.split(" — ").next().unwrap_or(m).to_string())).collect()
}

const RQ: &str = "'-- constants like RAPIDQ.INC's\n'--\n\nCONST SND_SYNC = 0\nCONST SND_ASYNC = 1\nConst SND_NODEFAULT = &H2\nCONST ANSI_CHARSET = 0\n";
const WIN: &str = "$IFNDEF __WIN\n$DEFINE __WIN\n$OPTION WEAKTYPE ON\n$OPTION VBDLL    ON\nPublic Const ANSI_CHARSET = 0\nPublic Const SND_SYNC = &H0\nPublic Const SND_ASYNC = &H1\nPublic Const WM_USER = &H400\n$OPTION WEAKTYPE OFF\n$OPTION VBDLL    OFF\n$ENDIF '__WIN\n";

#[test]
fn a_const_written_again_while_weaktype_is_on_is_no_error() {
    for (what, program) in [
        ("on, twice", "$OPTION WEAKTYPE ON\nCONST A = 1\nCONST A = 2\nPRINT A\n"),
        ("on, another type", "$OPTION WEAKTYPE ON\nCONST A = 1\nCONST A = \"x\"\nPRINT A\n"),
        ("on at the second only", "CONST A = 1\n$OPTION WEAKTYPE ON\nCONST A = 2\nPRINT A\n"),
        ("Public and Private, any capitals", "$option weaktype on\nCONST A = 1\nPublic Const a = 2\nPrivate Const A = 3\nPRINT A\n"),
        ("on, off, on", "$OPTION WEAKTYPE ON\nCONST A = 1\n$OPTION WEAKTYPE OFF\nCONST B = 1\n$OPTION WEAKTYPE ON\nCONST A = 2\n"),
        ("a comment after it", "$OPTION WEAKTYPE ON ' repeats\nCONST A = 1\nCONST A = 2\n"),
        ("in an IF", "$OPTION WEAKTYPE ON\nCONST A = 1\nIF A = 1 THEN\n  CONST A = 2\nEND IF\n"),
        ("after a DIM", "$OPTION WEAKTYPE ON\nDIM A AS INTEGER\nCONST A = 2\n"),
        ("after a SUB", "$OPTION WEAKTYPE ON\nSUB A\n  PRINT 1\nEND SUB\nCONST A = 2\n"),
    ] {
        let e = errors("on", &[("main.bas", program)]);
        assert!(e.is_empty(), "{what}: {e:?}");
    }
}

#[test]
fn it_counts_where_the_second_const_is_written() {
    // (off, the default: any second CONST, Public Const and Private Const too)
    for (program, line) in [
        ("CONST A = 1\nCONST A = 1\n", 2),
        ("CONST A = 1\nPublic Const A = 2\n", 2),
        ("Public Const A = 1\nCONST A = 1\n", 2),
        ("CONST A = 1\nPrivate Const A = 2\n", 2),
        ("$OPTION WEAKTYPE OFF\nCONST A = 1\nCONST A = 1\n", 3),
        // (on for the first only)
        ("$OPTION WEAKTYPE ON\nCONST A = 1\n$OPTION WEAKTYPE OFF\nCONST A = 1\n", 4),
        // (the other option of the VB ports is not it)
        ("$OPTION VBDLL ON\nCONST A = 1\nCONST A = 1\n", 3),
    ] {
        let e = errors("off", &[("main.bas", program)]);
        assert_eq!(heads(&e), [("main.bas".to_string(), line, "CONST variable A already defined".to_string())], "{program:?}: {e:?}");
    }
}

#[test]
fn only_a_const_may_be_written_again() {
    // (RC.EXE: `Identifier B already used, try another name` for the second DIM, ...)
    let e = errors("dim", &[("main.bas", "$OPTION WEAKTYPE ON\nDIM B AS INTEGER\nDIM B AS INTEGER\n")]);
    assert_eq!(heads(&e), [("main.bas".to_string(), 3, "Identifier B already used, try another name".to_string())], "{e:?}");
    // (... for a DIM after a CONST)
    let e = errors("dim-after-const", &[("main.bas", "$OPTION WEAKTYPE ON\nCONST A = 2\nDIM A AS INTEGER\n")]);
    assert_eq!(heads(&e), [("main.bas".to_string(), 3, "Identifier A already used, try another name".to_string())], "{e:?}");
}

#[test]
fn a_rapidr_program_keeps_its_repeated_const_an_error() {
    // (docs/rr-sane-audit.md: what a .rr should do is Robert's to decide)
    let e = errors("rr", &[("main.rr", "$OPTION WEAKTYPE ON\nCONST A = 1\nCONST A = 2\nPRINT A\n")]);
    assert_eq!(heads(&e), [("main.rr".to_string(), 3, "CONST variable A already defined".to_string())], "{e:?}");
}

#[test]
fn a_vb_port_after_the_classic_constants_and_not_before_them() {
    // RC.EXE: `$INCLUDE` the RAPIDQ.INC-like file, then the port: it compiles ...
    let e = errors("pair", &[("main.bas", "$INCLUDE \"rq.inc\"\n$INCLUDE \"win.inc\"\nPRINT SND_SYNC\n"), ("rq.inc", RQ), ("win.inc", WIN)]);
    assert!(e.is_empty(), "{e:?}");
    // ... the other order stops at the RAPIDQ.INC-like file's first repeated name
    let e = errors("pair-swapped", &[("main.bas", "$INCLUDE \"win.inc\"\n$INCLUDE \"rq.inc\"\nPRINT SND_SYNC\n"), ("rq.inc", RQ), ("win.inc", WIN)]);
    let h = heads(&e);
    assert_eq!(h.first(), Some(&("rq.inc".to_string(), 4, "CONST variable SND_SYNC already defined".to_string())), "{e:?}");
    // (… and only the names both files have: SND_SYNC, SND_ASYNC, ANSI_CHARSET)
    assert_eq!(h.len(), 3, "{e:?}");
}

#[test]
fn the_option_takes_on_or_off_and_nothing_else() {
    // RC.EXE: `Option WEAKTYPE expected ON|OFF but got MAYBE` (upper case), `Expected end-of-line but got JUNK`, and nothing after `got` for no setting
    let e = errors("values", &[("main.bas", "$OPTION WEAKTYPE maybe\n$OPTION WEAKTYPE ON junk\n$OPTION WEAKTYPE\nPRINT 1\n")]);
    let h = heads(&e);
    assert_eq!(h.len(), 3, "{e:?}");
    assert_eq!((h[0].1, h[0].2.as_str()), (1, "Option WEAKTYPE expected ON|OFF but got MAYBE"));
    assert_eq!((h[1].1, h[1].2.as_str()), (2, "Expected end-of-line but got JUNK"));
    assert_eq!(h[2].1, 3);
    assert!(h[2].2.starts_with("Option WEAKTYPE expected ON|OFF but got"), "{e:?}");
    // (a comment after the setting, or touching it, is fine)
    for program in ["$OPTION WEAKTYPE OFF ' why\nPRINT 1\n", "$OPTION WEAKTYPE ON'why not\nPRINT 1\n"] {
        let e = errors("comment", &[("main.bas", program)]);
        assert!(e.is_empty(), "{program:?}: {e:?}");
    }
}
