//! The import of a program that includes a classic WIN32API.INC: RWin32.inc
//! offered with the differences listed, the copy switched only when asked,
//! and the switched copy compiling (tests/fixtures/rwin32/import).

use std::fs;
use std::path::{Path, PathBuf};

use rapidr_import::{import, report, Options, Verification};

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for e in fs::read_dir(from).unwrap() {
        let p = e.unwrap().path();
        let dest = to.join(p.file_name().unwrap());
        if p.is_dir() {
            copy_tree(&p, &dest);
        } else {
            fs::copy(&p, &dest).unwrap();
        }
    }
}

fn fixture(name: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!("rapidr-import-rwin32-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    copy_tree(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/rwin32/import"), &base.join("src"));
    base
}

#[test]
fn win32api_is_offered_and_switched_only_when_asked() {
    let base = fixture("offer");
    let out = base.join("copy");
    let r = import(&base.join("src/prog.bas"), &out, &Options::default(), true).unwrap();
    // the copy as always: the user's file, proved the same as the original
    assert_eq!(r.programs[0].verification, Some(Verification::Identical));
    let o = r.rwin32.clone().expect("RWin32.inc offered");
    assert!(o.switched.is_none());
    assert!(fs::read_to_string(out.join("prog.bas")).unwrap().contains("include\\WIN32API.INC"), "nothing switched unasked");
    let all = o.differences.join("\n");
    assert!(all.contains("$DEFINE INTEGER SHORT"), "{all}");
    assert!(all.contains("GetCurrentDirectory: RWin32.inc calls GetCurrentDirectoryA in kernel32"), "{all}");
    assert!(all.contains("CONST MY_FLAG (7): not in RWin32.inc"), "{all}");
    assert!(!all.contains("WM_CLOSE") && !all.contains("FindWindow") && !all.contains("MB_ICONINFORMATION"), "the same: {all}");
    let md = report::markdown(&r);
    assert!(md.contains("## Windows API declarations: WIN32API.INC and RWin32.inc") && md.contains("Nothing changes unless you want it to"), "{md}");
    assert!(report::summary(&r).contains("RWin32.inc offered (3 difference(s)"), "{}", report::summary(&r));
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn the_switched_copy_names_rwin32_and_compiles() {
    let base = fixture("switch");
    let out = base.join("copy");
    let mut r = import(&base.join("src/prog.bas"), &out, &Options::default(), true).unwrap();
    // (the program's own constant the new file lacks: kept in the program,
    // as the report says to)
    let n = rapidr_import::rwin32::switch_copy(&rapidr_import::Disk, &mut r).unwrap();
    assert_eq!(n, 1);
    let text = fs::read_to_string(out.join("prog.bas")).unwrap();
    assert!(text.contains("$INCLUDE \"RWin32.inc\"") && !text.contains("include\\WIN32API.INC"), "{text}");
    // MY_FLAG isn't in RWin32.inc: the switched copy reads it as an
    // undeclared variable (0), and compiles; the report has said so
    assert_eq!(r.rwin32.as_ref().unwrap().switched, Some(Ok(())));
    assert!(report::markdown(&r).contains("The switched copy compiles."));
    assert!(report::summary(&r).contains("the copy uses RWin32.inc now and compiles"));
    // the original is untouched
    assert!(fs::read_to_string(base.join("src/prog.bas")).unwrap().contains("include\\WIN32API.INC"));
    let _ = fs::remove_dir_all(&base);
}
