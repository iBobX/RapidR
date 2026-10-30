//! RapidR's own catalogs are whole and sound (lane I18N-STUDIO): RapidR
//! Studio's template (ide/lang/studio.pot) lists exactly the texts its
//! sources translate — every TR$ / TRN$ and keyword argument in ide/*.inc,
//! no text missing, none left over —, RapidR's own (the runtime's
//! crates/rapidr-value/lang/runtime.pot) every tr_ui in the Rust sources,
//! and every translation keeps its markers ({name}, %1, `code`, one &, a
//! menu's "...") and makes no menu two items with one mnemonic.
//!
//! Out of date: `rapidr i18n studio` / `rapidr i18n runtime` write them.

use std::fs;
use std::path::{Path, PathBuf};

use rapidr_i18n::po::{self, Message};
use rapidr_i18n::tools;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn studio_sources() -> Vec<(String, String)> {
    let ide = root().join("ide");
    let mut names: Vec<String> = fs::read_dir(&ide).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.ends_with(".inc") || n == "studio.rr").collect();
    names.sort();
    names.iter().map(|n| (format!("ide/{n}"), fs::read_to_string(ide.join(n)).unwrap())).collect()
}

fn keys(messages: &[Message]) -> Vec<(String, String)> {
    let mut k: Vec<(String, String)> = messages.iter().filter(|m| !m.is_header()).map(|m| (m.context.clone(), m.id.clone())).collect();
    k.sort();
    k
}

#[test]
fn studio_template_lists_every_text_its_sources_translate() {
    let made = tools::template("RapidR Studio", &studio_sources(), &[]);
    let file = po::read_messages(&fs::read_to_string(root().join("ide/lang/studio.pot")).expect("ide/lang/studio.pot"));
    let (made_keys, file_keys) = (keys(&made), keys(&file));
    let missing: Vec<_> = made_keys.iter().filter(|k| !file_keys.contains(k)).collect();
    let orphans: Vec<_> = file_keys.iter().filter(|k| !made_keys.contains(k)).collect();
    assert!(missing.is_empty() && orphans.is_empty(), "ide/lang/studio.pot is out of date (rapidr i18n studio): missing {missing:?}, no longer used {orphans:?}");
    // (the same text the tool writes: references too)
    assert_eq!(tools::write(&made), fs::read_to_string(root().join("ide/lang/studio.pot")).unwrap(), "ide/lang/studio.pot: rapidr i18n studio writes it");
    assert!(made_keys.len() > 1000, "{} texts", made_keys.len());
}

#[test]
fn runtime_template_lists_every_tr_ui() {
    let mut files = Vec::new();
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    for top in ["crates", "interpreter"] {
        for c in fs::read_dir(root().join(top)).unwrap().flatten() {
            walk(&c.path().join("src"), &mut files);
        }
    }
    files.sort();
    let root = root().canonicalize().unwrap();
    let sources: Vec<(String, String)> = files
        .into_iter()
        .filter_map(|f| {
            let text = fs::read_to_string(&f).ok()?;
            text.contains("tr_ui").then(|| (f.canonicalize().unwrap().strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/"), text))
        })
        .collect();
    let made = tools::rust_template("RapidR runtime", &sources);
    let file = fs::read_to_string(root.join("crates/rapidr-value/lang/runtime.pot")).expect("runtime.pot");
    assert_eq!(keys(&made), keys(&po::read_messages(&file)), "crates/rapidr-value/lang/runtime.pot is out of date (rapidr i18n runtime)");
}

#[test]
fn translations_keep_their_markers_and_menus_their_keys() {
    let commands = fs::read_to_string(root().join("ide/commands.inc")).unwrap();
    let mut files: Vec<(PathBuf, String)> = Vec::new();
    for dir in ["ide/lang", "crates/rapidr-value/lang", "crates/rapidr-diagnostics/lang"] {
        for e in fs::read_dir(root().join(dir)).unwrap().flatten() {
            let n = e.file_name().to_string_lossy().into_owned();
            if n.ends_with(".po") && n != "xx.po" {
                files.push((e.path(), n.trim_end_matches(".po").to_string()));
            }
        }
    }
    assert!(!files.is_empty());
    for (path, tag) in files {
        let file = po::read_messages(&fs::read_to_string(&path).unwrap());
        let problems = tools::check(&tag, &file);
        assert!(problems.is_empty(), "{}: {problems:?}", path.display());
        if path.ends_with(format!("ide/lang/{tag}.po")) {
            let clashes = tools::studio_mnemonic_clashes(&commands, &file);
            assert!(clashes.is_empty(), "{}: two items of a menu with one key: {clashes:?}", path.display());
        }
    }
}
