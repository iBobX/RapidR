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

/// What the lawyer's review of the Portuguese asked for
/// (docs/legal/review-2026-11-06-pt-br.md) stays: where a key is kept comes
/// with its own preposition (never "em {store}", "em a memória"), the age is
/// said ("quem tem 18 anos ou mais", never "adultos": in Brazil that word is
/// for adult content), "sandbox" promises no isolation, and "nothing waits
/// for you" says whose approval.
#[test]
fn portuguese_keeps_the_legal_reviews_wording() {
    for dir in ["ide/lang", "crates/rapidr-value/lang"] {
        let file = po::read_messages(&fs::read_to_string(root().join(dir).join("pt-BR.po")).unwrap());
        for m in file.iter().filter(|m| !m.is_header() && m.translated()) {
            let pt = m.forms.join(" | ");
            let said = format!("{dir}: {:?} -> {pt:?}", m.id);
            for bad in ["em {store}", "guardada em {place}", "em {source}"," em a memória", " em o Acesso", " em esta aba", " em este navegador", "para adultos", "ambiente isolado", "Nada espera por você", "esperando por você no", "espera por você no", "nem SHELL e RUN"] {
                assert!(!pt.contains(bad), "{bad:?} in {said}");
            }
            // (the English says adults: the Portuguese says the age)
            if m.id.contains("for adults") {
                assert!(pt.contains("18 anos ou mais"), "{said}");
            }
            // (a place of the key, said alone, carries "no / na / nesta / neste / em uma")
            if ["the macOS Keychain", "Windows Credential Manager", "an environment variable", "this browser tab, until it closes", "memory, for this session only"].contains(&m.id.as_str()) {
                assert!(["no ", "na ", "nesta ", "neste ", "em "].iter().any(|p| pt.starts_with(p)), "{said}");
            }
        }
    }
}

/// A language is whole or not there: Studio's, the runtime's and the
/// compiler's catalogs name the same languages (Preview 6: es, pt-BR,
/// zh-Hans), each tag as `normalize_tag` writes it, and the catalog's own
/// header says the language it is and a plural rule that parses.
#[test]
fn every_language_has_its_three_catalogs() {
    let tags = |dir: &str| -> Vec<String> {
        let mut t: Vec<String> = fs::read_dir(root().join(dir)).unwrap().flatten().filter_map(|e| e.file_name().to_string_lossy().strip_suffix(".po").map(String::from)).filter(|t| t != "xx").collect();
        t.sort();
        t
    };
    let studio = tags("ide/lang");
    assert!(studio.len() >= 3, "{studio:?}");
    assert_eq!(studio, tags("crates/rapidr-value/lang"), "Studio's and the runtime's languages");
    assert_eq!(studio, tags("crates/rapidr-diagnostics/lang"), "Studio's and the compiler's languages");
    for dir in ["ide/lang", "crates/rapidr-value/lang", "crates/rapidr-diagnostics/lang"] {
        for tag in &studio {
            assert_eq!(&rapidr_i18n::normalize_tag(tag), tag, "{dir}/{tag}.po: the file name is the tag as RapidR writes it");
            let messages = po::read_messages(&fs::read_to_string(root().join(dir).join(format!("{tag}.po"))).unwrap());
            let head = messages.iter().find(|m| m.is_header()).unwrap_or_else(|| panic!("{dir}/{tag}.po has no header"));
            assert_eq!(po::header_value(head, "language").as_deref(), Some(tag.as_str()), "{dir}/{tag}.po: Language");
            let rule = po::header_value(head, "plural-forms").unwrap_or_else(|| panic!("{dir}/{tag}.po: no Plural-Forms"));
            assert_eq!(rule.trim(), rapidr_i18n::tags::plural_forms(tag), "{dir}/{tag}.po: the language's plural rule");
            assert!(po::Plural::parse(&rule).is_some(), "{dir}/{tag}.po: {rule}");
        }
    }
}
