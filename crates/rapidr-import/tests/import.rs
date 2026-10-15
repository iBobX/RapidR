//! The RapidQ importer on small programs: what it changes, what it leaves,
//! and the proof that the copy compiles to the same bytecode.

use std::fs;
use std::path::{Path, PathBuf};

use rapidr_import::{import, import_with, plan_program, report, upgrade_file, Files, Memory, NameStyle, Options, Place, Verification};

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rapidr-import-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn write(d: &Path, name: &str, text: &str) -> PathBuf {
    let p = d.join(name);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&p, text).unwrap();
    p
}

const PROGRAM: &str = "' A QFORM with a QBUTTON (a comment: not changed)
$INCLUDE \"RAPIDQ.INC\"
$INCLUDE \"lib.rqb\"
DIM QButtonCount AS INTEGER
DIM Pb AS QGAUGE
DIM Grid AS qstringgrid, Old AS RBUTTON
TYPE TMyForm EXTENDS QFORM
    Extra AS QLABEL
END TYPE
SUB Clicked (Sender AS QBUTTON)
    QButtonCount = QButtonCount + 1
    PRINT \"QBUTTON clicked\", clRed
END SUB
FUNCTION MakeFont AS QFONT
    DIM f AS QFONT
    MakeFont = f
END FUNCTION
CREATE Form AS QFORM
    Caption = \"QFORM\"
    CREATE Button AS QBUTTON
        OnClick = Clicked
    END CREATE
END CREATE
$IFDEF NOT_DEFINED
DIM Never AS QEDIT
$ENDIF
";

#[test]
fn converts_type_names_only() {
    let d = dir("names");
    let main = write(&d, "main.bas", PROGRAM);
    write(&d, "lib.rqb", "SUB Helper (L AS QLISTBOX)\nEND SUB\n");
    let plan = plan_program(&main, &Options::default()).unwrap();
    let f = &plan.files[0];
    let out = f.converted();
    let expected = PROGRAM
        .replace("DIM Pb AS QGAUGE", "DIM Pb AS RProgressBar")
        .replace("AS qstringgrid, Old AS RBUTTON", "AS RStringGrid, Old AS RButton")
        .replace("EXTENDS QFORM", "EXTENDS RForm")
        .replace("Extra AS QLABEL", "Extra AS RLabel")
        .replace("(Sender AS QBUTTON)", "(Sender AS RButton)")
        .replace("MakeFont AS QFONT", "MakeFont AS RFont")
        .replace("f AS QFONT", "f AS RFont")
        .replace("Form AS QFORM", "Form AS RForm")
        .replace("Button AS QBUTTON", "Button AS RButton");
    assert_eq!(out, expected);
    // (the comment, the strings, QButtonCount and the $IFDEF branch stay)
    assert!(out.contains("' A QFORM with a QBUTTON") && out.contains("\"QBUTTON clicked\"") && out.contains("Never AS QEDIT"));
    assert!(plan.notes.iter().any(|n| n.message.contains("`QEDIT` is in an $IFDEF branch")), "{:?}", plan.notes);
    let lib = plan.files.iter().find(|f| f.path.ends_with("lib.rqb")).unwrap();
    assert_eq!(lib.converted(), "SUB Helper (L AS RListBox)\nEND SUB\n");
    assert_eq!(f.style(), NameStyle::Mixed, "QBUTTON … and RBUTTON");
    assert_eq!(lib.style(), NameStyle::RapidQ);
    let c = &f.changes[0];
    assert_eq!((c.line, c.column, c.from.as_str(), c.to.as_str()), (5, 11, "QGAUGE", "RProgressBar"));
}

#[test]
fn program_types_and_defines_stay() {
    let d = dir("own");
    // (QBevel.inc's own TYPE QBEVEL: the program's, not RapidR's built-in)
    let main = write(&d, "main.bas", "TYPE QBEVEL EXTENDS QPANEL\n    Depth AS INTEGER\nEND TYPE\nDIM B AS QBEVEL\n$DEFINE MyForm QFORM\nDIM F AS MyForm\nTYPE T EXTENDS QOBJECT\nEND TYPE\n");
    let plan = plan_program(&main, &Options::default()).unwrap();
    assert_eq!(plan.files[0].converted(), "TYPE QBEVEL EXTENDS RPanel\n    Depth AS INTEGER\nEND TYPE\nDIM B AS QBEVEL\n$DEFINE MyForm QFORM\nDIM F AS MyForm\nTYPE T EXTENDS RObject\nEND TYPE\n");
    assert!(plan.notes.iter().any(|n| n.message.contains("$DEFINE MyForm QFORM")), "{:?}", plan.notes);
}

#[test]
fn import_copies_proves_and_reports() {
    let d = dir("import");
    let src = d.join("prog");
    let main = write(&src, "main.rqw", &PROGRAM.replace('\n', "\r\n"));
    write(&src, "lib.rqb", "SUB Helper (L AS QLISTBOX)\r\nEND SUB\r\n");
    write(&src, "data/notes.txt", "QFORM in a data file\n");
    write(&src, "Window.tpl", "a RapidQ IDE template\n");
    let before = fs::read(&main).unwrap();
    let out = d.join("copy");
    let r = import(&src, &out, &Options::default(), true).unwrap();
    assert_eq!(fs::read(&main).unwrap(), before, "the original is untouched");
    let copy = fs::read_to_string(out.join("main.rqw")).unwrap();
    assert!(copy.contains("CREATE Form AS RForm\r\n"), "CRLF and the extension kept: {copy}");
    assert_eq!(fs::read_to_string(out.join("lib.rqb")).unwrap(), "SUB Helper (L AS RListBox)\r\nEND SUB\r\n");
    assert_eq!(fs::read_to_string(out.join("data/notes.txt")).unwrap(), "QFORM in a data file\n");
    assert!(!out.join("Window.tpl").exists(), "RapidQ's templates aren't copied");
    // (the $INCLUDE "RAPIDQ.INC" line is kept: its constants come with it)
    assert!(copy.contains("$INCLUDE \"RAPIDQ.INC\""));
    let main_report = r.programs.iter().find(|p| p.entry.ends_with("main.rqw")).unwrap();
    assert_eq!(main_report.verification, Some(Verification::Identical));
    let md = report::markdown(&r);
    assert!(md.contains("line 5, column 11: `QGAUGE` → `RProgressBar`"), "{md}");
    assert!(md.contains("`.rqw`") && md.contains("RAPIDQ.INC"), "{md}");
    assert!(md.contains("RapidQ IDE template"), "{md}");
    // Importing the copy again changes nothing: it already has RapidR's names.
    let again = import(&out, &d.join("copy2"), &Options::default(), true).unwrap();
    assert_eq!(again.changes(), 0, "{}", report::markdown(&again));
    // Never over the original.
    assert!(import(&src, &src, &Options::default(), false).is_err());
}

/// The same import in memory (RapidR Studio on the web: the page's store)
/// writes the same copy as on the disk, byte for byte — an include found by
/// its path (`include\\Shapes.inc`) before another file of its name, a
/// Windows-1252 text kept in its encoding — and proves it the same way.
#[test]
fn import_in_memory_writes_what_the_disk_does() {
    let d = dir("memory");
    let src = d.join("prog");
    let mut latin = PROGRAM.replace("QBUTTON clicked", "QBUTTON cliqu\u{e9}").replace('\n', "\r\n").into_bytes();
    // (é in Windows-1252, as RapidQ's editor saved it)
    let at = latin.windows(2).position(|w| w == "\u{e9}".as_bytes()).unwrap();
    latin.splice(at..at + 2, [0xE9]);
    let files: Vec<(&str, Vec<u8>)> = vec![
        ("main.bas", [b"$INCLUDE \"include\\Shapes.inc\"\r\n".as_slice(), &latin].concat()),
        ("lib.rqb", b"SUB Helper (L AS QLISTBOX)\r\nEND SUB\r\n".to_vec()),
        ("include/Shapes.inc", b"DIM Box AS QPANEL\r\n".to_vec()),
        ("Shapes.inc", b"DIM Wrong AS QEDIT\r\n".to_vec()),
        ("data/notes.txt", b"QFORM in a data file\n".to_vec()),
    ];
    for (name, bytes) in &files {
        let p = src.join(name);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, bytes).unwrap();
    }
    let on_disk = import(&src, &d.join("copy"), &Options::default(), true).unwrap();
    let mem = Memory::new(files.iter().map(|(n, b)| (PathBuf::from("/store/prog").join(n), b.clone())));
    let in_mem = import_with(&mem, Path::new("/store/prog"), Path::new("/store/prog-rapidr"), &Options::default(), true).unwrap();
    let written = mem.written();
    assert_eq!(written.len(), on_disk.files.len(), "{written:?}");
    for f in &on_disk.files {
        let rel = f.dest.strip_prefix(fs::canonicalize(d.join("copy")).unwrap()).unwrap();
        let want = fs::read(&f.dest).unwrap();
        let got = mem.read(&Path::new("/store/prog-rapidr").join(rel)).unwrap();
        assert_eq!(got, want, "{}", rel.display());
    }
    let main = mem.read(Path::new("/store/prog-rapidr/main.bas")).unwrap();
    assert!(main.contains(&0xE9) && !String::from_utf8_lossy(&main).contains("\u{e9}"), "Windows-1252 kept");
    assert_eq!(mem.read(Path::new("/store/prog-rapidr/include/Shapes.inc")).unwrap(), b"DIM Box AS RPanel\r\n");
    assert_eq!(on_disk.changes(), in_mem.changes());
    let ok = |r: &rapidr_import::ImportReport| r.programs.iter().filter(|p| p.verification == Some(Verification::Identical)).count();
    assert_eq!(ok(&in_mem), ok(&on_disk));
    assert!(ok(&in_mem) >= 1, "{}", report::markdown(&in_mem));
    // (nothing written over what was read)
    assert!(import_with(&mem, Path::new("/store/prog"), Path::new("/store/prog"), &Options::default(), false).is_err());
}

#[test]
fn rapidq_inc_file_left_out_unless_used() {
    let d = dir("rqinc");
    let src = d.join("prog");
    // (a RAPIDQ.INC of the program's own folder: constants RapidR has, and
    // RapidQ's QBColor array, which RapidR's built-in constants don't have)
    write(&src, "RAPIDQ.INC", "$IFNDEF __RQINC\n$DEFINE __RQINC\nCONST clRed = &HFF\nCONST MB_OK = 0\nDIM QBColor(0 TO 15) AS INTEGER\n$ENDIF\n");
    write(&src, "a.bas", "$INCLUDE \"RAPIDQ.INC\"\nPRINT clRed\nDIM F AS QFORM\n");
    let out = d.join("copy");
    let r = import(&src, &out, &Options::default(), true).unwrap();
    assert!(!out.join("RAPIDQ.INC").exists(), "RapidR supplies RAPIDQ.INC");
    assert_eq!(r.builtin_left_out.len(), 1);
    assert!(r.programs.iter().all(|p| p.verification == Some(Verification::Identical)), "{:?}", r.programs);
    assert!(report::markdown(&r).contains("Your own copies of includes RapidR supplies built in aren't copied"));

    // (QBColor is the user's file's own: still not copied — the report says
    // the copy needs that file found, in an include folder)
    write(&src, "a.bas", "$INCLUDE \"RAPIDQ.INC\"\nPRINT QBColor(4)\n");
    let out = d.join("copy2");
    let r = import(&src, &out, &Options::default(), true).unwrap();
    assert!(!out.join("RAPIDQ.INC").exists(), "never copied");
    assert!(r.notes.iter().any(|n| n.message.contains("QBCOLOR") && n.message.contains("include folders")), "{:?}", r.notes);
    assert_ne!(r.programs[0].verification, Some(Verification::Identical), "the copy alone finds RapidR's RAPIDQ.INC");
    // …found there, the copy compiles as the original
    let out = d.join("copy3");
    let r = import(&src, &out, &Options { include_dirs: vec![src.clone()], ..Options::default() }, true).unwrap();
    assert!(!out.join("RAPIDQ.INC").exists());
    assert_eq!(r.programs[0].verification, Some(Verification::Identical), "{}", report::markdown(&r));
}

/// RapidQ code written on Windows, imported: every include it uses — beside
/// it in another case (`Inc\MyLib.INC`), nested beside its includer, in a
/// folder above (`..\shared`), in the include folder (`<GL\QGL.INC>`), by a
/// Windows path from another computer (`C:\rapidq\include\winlib.inc`) — is
/// copied with its names converted, the lines that wouldn't find their copy
/// rewritten; RAPIDQ.INC stays RapidR's; a missing include is reported with
/// what to do. The copy compiles as the original, and on its own.
fn include_tree(base: &Path) -> (PathBuf, PathBuf) {
    let src = base.join("prog");
    let inc = base.join("rapidq").join("include");
    write(
        &src,
        "main.bas",
        "$INCLUDE \"RAPIDQ.INC\"\r\n$INCLUDE \"Inc\\MyLib.INC\"\r\n$INCLUDE \"..\\shared\\common.inc\"\r\n$INCLUDE \"C:\\rapidq\\include\\winlib.inc\"\r\n$INCLUDE <GL\\QGL.INC>\r\nDIM F AS QFORM\r\nPRINT clRed\r\n",
    );
    write(&src, "inc/mylib.inc", "$INCLUDE \"helper.inc\"\r\nDIM B AS QBUTTON\r\n");
    write(&src, "inc/helper.inc", "DIM H AS QLABEL\r\n");
    write(&src, "broken.bas", "$INCLUDE \"nowhere.inc\"\r\nPRINT 1\r\n");
    write(base, "shared/common.inc", "DIM C AS QEDIT\r\n");
    write(&inc, "winlib.inc", "DIM W AS QPANEL\r\n");
    write(&inc, "GL/qgl.inc", "DIM G AS QCANVAS\r\n");
    // (the user's own RAPIDQ.INC, as RapidQ's include folder has it)
    write(&inc, "RAPIDQ.INC", "CONST clRed = &HFF\r\n");
    (src, inc)
}

#[test]
fn includes_are_copied_found_and_reported() {
    let d = dir("includes");
    let (src, inc) = include_tree(&d);
    let out = d.join("prog-rapidr");
    let options = Options { include_dirs: vec![inc.clone()], ..Options::default() };
    let r = import(&src, &out, &options, true).unwrap();
    let md = report::markdown(&r);
    let read = |p: &str| fs::read_to_string(out.join(p)).unwrap_or_else(|e| panic!("{p}: {e}\n{md}"));
    // (every include copied, names converted)
    assert_eq!(read("inc/mylib.inc"), "$INCLUDE \"helper.inc\"\r\nDIM B AS RButton\r\n");
    assert_eq!(read("inc/helper.inc"), "DIM H AS RLabel\r\n");
    assert_eq!(read("common.inc"), "DIM C AS REdit\r\n");
    assert_eq!(read("winlib.inc"), "DIM W AS RPanel\r\n");
    assert_eq!(read("GL/qgl.inc"), "DIM G AS RCanvas\r\n");
    // (the files' own names, not as the lines wrote them)
    let names = |p: &str| -> Vec<String> { fs::read_dir(out.join(p)).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect() };
    assert!(names("inc").contains(&"mylib.inc".to_string()) && names("GL").contains(&"qgl.inc".to_string()), "{:?} {:?}", names("inc"), names("GL"));
    assert!(!out.join("RAPIDQ.INC").exists(), "RapidR supplies RAPIDQ.INC");
    // (the lines that would find the originals name the copies; the others stay)
    assert_eq!(
        read("main.bas"),
        "$INCLUDE \"RAPIDQ.INC\"\r\n$INCLUDE \"Inc\\MyLib.INC\"\r\n$INCLUDE \"common.inc\"\r\n$INCLUDE \"winlib.inc\"\r\n$INCLUDE <GL\\QGL.INC>\r\nDIM F AS RForm\r\nPRINT clRed\r\n"
    );
    let main = r.programs.iter().find(|p| p.entry.ends_with("main.bas")).unwrap();
    assert_eq!(main.verification, Some(Verification::Identical), "{md}");
    let broken = r.programs.iter().find(|p| p.entry.ends_with("broken.bas")).unwrap();
    assert!(matches!(broken.verification, Some(Verification::OriginalFails(_))), "{md}");
    // (where each was found)
    let place = |w: &str| r.includes.iter().find(|i| i.written == w).map(|i| i.place.clone()).unwrap_or_else(|| panic!("{w}: {md}"));
    let inc_real = fs::canonicalize(&inc).unwrap();
    assert_eq!(place("Inc\\MyLib.INC"), Place::ProgramFolder);
    assert_eq!(place("helper.inc"), Place::BesideIncluder);
    assert_eq!(place("GL\\QGL.INC"), Place::IncludeFolder(inc.clone()));
    assert_eq!(place("C:\\rapidq\\include\\winlib.inc"), Place::ByName(inc.clone()));
    assert_eq!(place("nowhere.inc"), Place::Missing);
    assert_eq!(r.builtin_left_out, vec![inc_real.join("RAPIDQ.INC")]);
    // (the report: each include, the missing one with what to do)
    assert!(md.contains("## Include files") && md.contains("| `Inc\\MyLib.INC` | `main.bas`, line 2 | in the program's folder: `inc/mylib.inc` | `inc/mylib.inc` |"), "{md}");
    assert!(md.contains("`common.inc` (the line now names `common.inc`)"), "{md}");
    assert!(md.contains("### Not found") && md.contains("- `nowhere.inc` (`broken.bas`, line 1): add the folder that holds nowhere.inc in RapidR Studio > Tools > Settings > Include folders"), "{md}");
    assert!(report::summary(&r).ends_with("include files: 5 copied, 1 RapidR's built-in, 1 not found"), "{}", report::summary(&r));
    // The copy on its own (no include folder): the same program.
    let alone = rapidr_import::verify::bytecode(&out.join("main.bas"), &[], None).unwrap();
    let (original, _) = rapidr_import::verify::bytecode_with(&rapidr_import::Disk, &src.join("main.bas"), &[inc.clone()], None, &["RAPIDQ.INC".to_string()]).unwrap();
    assert_eq!(alone, original);
}

/// The same import in memory (RapidR Studio on the web: the files the user
/// picked) copies the same includes and rewrites the same lines.
#[test]
fn includes_in_memory_as_on_the_disk() {
    let d = dir("includes-mem");
    let (src, inc) = include_tree(&d);
    let options = Options { include_dirs: vec![inc.clone()], ..Options::default() };
    let on_disk = import(&src, &d.join("prog-rapidr"), &options, true).unwrap();
    let mut held = Vec::new();
    for (from, to) in [(&d, "/store")] {
        let mut stack = vec![from.clone()];
        while let Some(p) = stack.pop() {
            for e in fs::read_dir(&p).unwrap().flatten() {
                let path = e.path();
                if path.is_dir() {
                    if !path.ends_with("prog-rapidr") {
                        stack.push(path);
                    }
                } else {
                    held.push((PathBuf::from(to).join(path.strip_prefix(from).unwrap()), fs::read(&path).unwrap()));
                }
            }
        }
    }
    let mem = Memory::new(held);
    let options = Options { include_dirs: vec![PathBuf::from("/store/rapidq/include")], ..Options::default() };
    let in_mem = import_with(&mem, Path::new("/store/prog"), Path::new("/store/prog-rapidr"), &options, true).unwrap();
    for f in &on_disk.files {
        let rel = f.dest.strip_prefix(fs::canonicalize(d.join("prog-rapidr")).unwrap()).unwrap();
        assert_eq!(mem.read(&Path::new("/store/prog-rapidr").join(rel)).unwrap(), fs::read(&f.dest).unwrap(), "{}", rel.display());
    }
    assert_eq!(mem.written().len(), on_disk.files.len());
    let ok = |r: &rapidr_import::ImportReport| r.programs.iter().filter(|p| p.verification == Some(Verification::Identical)).count();
    assert_eq!((ok(&in_mem), ok(&on_disk)), (1, 1), "{}", report::markdown(&in_mem));
    assert_eq!(in_mem.missing().count(), 1);
}

#[test]
fn upgrade_in_place_plan() {
    let d = dir("upgrade");
    let p = write(&d, "app.rr", "CREATE F AS QFORM\n    CREATE B AS rbutton\n    END CREATE\nEND CREATE\nF.ShowModal\n");
    let u = upgrade_file(&p, &Options::default()).unwrap();
    assert_eq!(u.converted, "CREATE F AS RForm\n    CREATE B AS RButton\n    END CREATE\nEND CREATE\nF.ShowModal\n");
    assert_eq!(u.verified, Some(true));
    assert_eq!(u.changes.len(), 2);
    let upgraded = write(&d, "new.rr", &u.converted);
    let plan = plan_program(&upgraded, &Options::default()).unwrap();
    assert_eq!(plan.files[0].style(), NameStyle::RapidR);
    let diff = rapidr_import::diff::unified("app.rr", &u.original, &u.converted, 1);
    assert!(diff.contains("-CREATE F AS QFORM\n+CREATE F AS RForm\n"), "{diff}");
}
