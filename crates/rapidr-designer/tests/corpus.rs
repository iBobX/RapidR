//! The designer on real programs (docs/ide-plan.md I4 acceptance, "round
//! trip on the corpus"): every program of the repo's `examples/` and of the
//! RapidQ corpus (`$RAPIDQ_DIR`, default `~/Downloads/Rapidq`, read only)
//! with a CREATE block opens in the designer, and
//!
//! * saved without a change, gives the file's bytes back;
//! * every property set to the value it already has changes nothing;
//! * a component moved 8 px changes exactly its Left's value (one line,
//!   nothing else), the text read back is the designer's model, the
//!   program parses with the same diagnostics, and undo gives the exact
//!   bytes back.
//!
//! `RAPIDR_DESIGNER_REPORT=1` prints the counts.

use std::fs;
use std::path::{Path, PathBuf};

use rapidr_designer::{Command, Document};
use rapidr_preprocessor::PreprocessOptions;

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|e| e.path());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if path.extension().and_then(|e| e.to_str()).is_some_and(|e| ["bas", "rr"].contains(&e.to_ascii_lowercase().as_str())) {
            out.push(path);
        }
    }
}

fn rapidq_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("RAPIDQ_DIR").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join("Downloads/Rapidq")))?;
    dir.join("examples").is_dir().then_some(dir)
}

#[derive(Default, Debug)]
struct Counts {
    programs: usize,
    with_forms: usize,
    forms: usize,
    components: usize,
    properties: usize,
    moved: usize,
    failures: Vec<String>,
}

fn check(path: &Path, options: &PreprocessOptions, c: &mut Counts) {
    let Ok(bytes) = fs::read(path) else { return };
    c.programs += 1;
    let mut doc = Document::open_bytes(&bytes, Some(path), options.clone());
    let name = path.display().to_string();
    if doc.bytes() != bytes {
        c.failures.push(format!("{name}: opening changed the bytes"));
        return;
    }
    if doc.forms().is_empty() {
        return;
    }
    c.with_forms += 1;
    c.forms += doc.forms().len();
    let diagnostics = doc.diagnostics().to_vec();
    // Every property set to its own value: nothing to write.
    for fi in 0..doc.forms().len() {
        let des = doc.designer(fi).unwrap();
        let mut cmds = Vec::new();
        for id in des.design.ids() {
            c.components += 1;
            for p in des.design.node(id).unwrap().props() {
                c.properties += 1;
                cmds.push(Command::SetProp { node: id, name: p.name.clone(), value: Some(des.design.node(id).unwrap().prop(&p.name).unwrap().to_string()) });
            }
        }
        des.execute(Command::Batch(cmds)).unwrap();
        let patches = doc.sync();
        if !patches.is_empty() || doc.bytes() != bytes {
            c.failures.push(format!("{name}: setting values to themselves wrote {patches:?}"));
            return;
        }
    }
    // One component per form moved 8 px: one value changes.
    for fi in 0..doc.forms().len() {
        let des = &doc.forms()[fi].designer;
        let target = des.design.ids().into_iter().skip(1).find(|&id| des.design.node(id).is_some_and(|n| n.int("Left").is_some() && n.prop("Left").is_some_and(|v| v.trim().parse::<i64>().is_ok())));
        let Some(id) = target else { continue };
        let left = des.design.node(id).unwrap().int("Left").unwrap();
        let before = doc.text().to_string();
        let des = doc.designer(fi).unwrap();
        des.execute(Command::SetProp { node: id, name: "Left".into(), value: Some((left + 8).to_string()) }).unwrap();
        let expected = des.design.clone();
        let patches = doc.sync();
        c.moved += 1;
        let after = doc.text().to_string();
        let changed: Vec<(&str, &str)> = before.split('\n').zip(after.split('\n')).filter(|(a, b)| a != b).collect();
        if patches.len() != 1 || changed.len() != 1 || before.split('\n').count() != after.split('\n').count() {
            c.failures.push(format!("{name}: moving {id} changed {} lines with {patches:?}", changed.len()));
        } else {
            let (a, b) = changed[0];
            let (old, new) = (left.to_string(), (left + 8).to_string());
            if a.replacen(&old, &new, 1) != b && a.matches(&old).count() == 1 {
                c.failures.push(format!("{name}: the moved line `{a}` became `{b}`"));
            }
        }
        if doc.forms()[fi].designer.design != expected {
            c.failures.push(format!("{name}: the text read back isn't the designer's model"));
        }
        if doc.diagnostics() != diagnostics.as_slice() {
            c.failures.push(format!("{name}: the move changed the diagnostics: {:?}", doc.diagnostics()));
        }
        doc.undo();
        if doc.bytes() != bytes {
            c.failures.push(format!("{name}: undo didn't give the bytes back"));
        }
    }
}

#[test]
fn every_form_round_trips() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    walk(&root.join("examples"), &mut files);
    let mut repo = Counts::default();
    for f in &files {
        check(f, &PreprocessOptions::default(), &mut repo);
    }
    let mut corpus = Counts::default();
    if let Some(dir) = rapidq_dir() {
        let mut files = Vec::new();
        walk(&dir.join("examples"), &mut files);
        let options = PreprocessOptions { include_dirs: vec![dir.join("include")], ..PreprocessOptions::default() };
        for f in &files {
            check(f, &options, &mut corpus);
        }
    }
    if std::env::var_os("RAPIDR_DESIGNER_REPORT").is_some() {
        eprintln!("examples/: {repo:?}");
        eprintln!("RapidQ corpus: {corpus:?}");
    }
    let failures: Vec<&String> = repo.failures.iter().chain(&corpus.failures).collect();
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.iter().take(40).map(|s| s.as_str()).collect::<Vec<_>>().join("\n"));
    assert!(repo.with_forms >= 15, "examples/ has forms: {repo:?}");
}
