//! The files generated from the registry are current: the VS Code
//! extension's and the web IDE's language data, the manual's reference
//! pages. When this fails: `rapidr lang export --all` (from the repository's
//! root) — or `RAPIDR_LANG_BLESS=1 cargo test -p rapidr-lang --test
//! generated`, which writes them — and commit what it wrote.

#[test]
fn generated_files_are_current() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    if std::env::var_os("RAPIDR_LANG_BLESS").is_some() {
        for (path, text) in rapidr_lang::export::generated_files() {
            std::fs::write(root.join(&path), text).unwrap_or_else(|e| panic!("{path}: {e}"));
        }
    }
    let stale: Vec<String> = rapidr_lang::export::generated_files()
        .into_iter()
        .filter(|(path, text)| std::fs::read_to_string(root.join(path)).ok().as_deref() != Some(text.as_str()))
        .map(|(path, _)| path)
        .collect();
    assert!(stale.is_empty(), "out of date (run `rapidr lang export --all` from the repository's root): {}", stale.join(", "));
}
