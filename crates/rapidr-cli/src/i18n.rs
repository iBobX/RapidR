//! `rapidr i18n …`: the tools translators use (rapidr_i18n::tools), for any
//! program's catalogs and for RapidR's own — RapidR Studio's text
//! (`ide/lang`), the compiler's messages (crates/rapidr-diagnostics/lang),
//! the manual (docs/manual/<language>/) and the registry's docs
//! (crates/rapidr-lang/data/i18n/<language>/).
//!
//! ```text
//! rapidr i18n extract <file|folder…> [-o FILE.pot] [--project NAME] [--keyword Name:N]
//!                         a template: every TR$ / TRN$ text, and the keywords' arguments
//! rapidr i18n pseudo <template.pot> [-o xx.po]   the pseudo-language's catalog
//! rapidr i18n update <template.pot> <lang.po…>   each language's file brought up to date
//! rapidr i18n check <lang.po…>                   translations keep {name}, %1, `code`, & …
//! rapidr i18n studio [--check]                   Studio's template, xx.po, checks (from the checkout)
//! rapidr i18n compiler [--check]                 the compiler messages' template
//! rapidr i18n runtime [--check]                  RapidR's own text's template (every tr_ui in the sources)
//! rapidr i18n coverage [LANG…]                   how much of RapidR each language has
//! rapidr i18n manual-stamp <LANG> <page.md>      a translated manual page records the English it translates
//! ```
//!
//! `--check`: write nothing; exit 1 when a generated file is out of date or
//! a translation loses a marker.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rapidr_i18n::extract::Keyword;
use rapidr_i18n::po::{self, Message};
use rapidr_i18n::tools;

const USAGE: &str = "usage: rapidr i18n extract <file|folder…> [-o FILE.pot] [--project NAME] [--keyword Name:N]\n\
                     \x20      rapidr i18n pseudo <template.pot> [-o xx.po]\n\
                     \x20      rapidr i18n update <template.pot> <lang.po…>\n\
                     \x20      rapidr i18n check <lang.po…>\n\
                     \x20      rapidr i18n studio [--check] | compiler [--check] | runtime [--check] | coverage [LANG…]";

pub fn command(args: &[String]) -> ExitCode {
    let rest = args.get(1..).unwrap_or(&[]);
    let r = match args.first().map(String::as_str) {
        Some("extract") => extract_cmd(rest),
        Some("pseudo") => pseudo_cmd(rest),
        Some("update") => update_cmd(rest),
        Some("check") => check_cmd(rest),
        Some("studio") => studio_cmd(rest),
        Some("compiler") => compiler_cmd(rest),
        Some("runtime") => runtime_cmd(rest),
        Some("coverage") => coverage_cmd(rest),
        Some("manual-stamp") => stamp_cmd(rest),
        _ => Err(USAGE.to_string()),
    };
    match r {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}

/// A flag's value (`-o FILE`), taken out of `args`.
fn take(args: &mut Vec<String>, names: &[&str]) -> Option<String> {
    let i = args.iter().position(|a| names.contains(&a.as_str()))?;
    args.remove(i);
    (i < args.len()).then(|| args.remove(i))
}

fn take_flag(args: &mut Vec<String>, name: &str) -> bool {
    let before = args.len();
    args.retain(|a| a != name);
    args.len() != before
}

fn read(path: &Path) -> Result<String, String> {
    fs::read(path).map(|b| String::from_utf8_lossy(&b).into_owned()).map_err(|e| format!("{}: {e}", path.display()))
}

fn write(path: &Path, text: &str) -> Result<(), String> {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// The program sources under `paths` (files as given; folders: their
/// `.rr`, `.bas`, `.inc`, `.rqw` files, sorted), relative to `base` for the
/// references.
fn sources(paths: &[String], base: &Path) -> Result<Vec<(String, String)>, String> {
    let mut files: Vec<PathBuf> = Vec::new();
    for p in paths {
        let path = PathBuf::from(p);
        if path.is_dir() {
            let mut found: Vec<PathBuf> = fs::read_dir(&path)
                .map_err(|e| format!("{p}: {e}"))?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|f| f.extension().and_then(|e| e.to_str()).is_some_and(|e| ["rr", "bas", "inc", "rqw"].contains(&e.to_ascii_lowercase().as_str())))
                .collect();
            found.sort();
            files.extend(found);
        } else {
            files.push(path);
        }
    }
    files
        .iter()
        .map(|f| {
            let name = f.strip_prefix(base).unwrap_or(f).to_string_lossy().replace('\\', "/");
            read(f).map(|t| (name, t))
        })
        .collect()
}

fn keywords(args: &mut Vec<String>) -> Vec<Keyword> {
    let mut out = Vec::new();
    while let Some(spec) = take(args, &["--keyword", "-k"]) {
        out.extend(Keyword::parse(&spec));
    }
    out
}

fn extract_cmd(args: &[String]) -> Result<ExitCode, String> {
    let mut args = args.to_vec();
    let out = take(&mut args, &["-o", "--output"]);
    let project = take(&mut args, &["--project"]).unwrap_or_else(|| "PROJECT VERSION".to_string());
    let kw = keywords(&mut args);
    if args.is_empty() {
        return Err(USAGE.to_string());
    }
    let base = std::env::current_dir().unwrap_or_default();
    let template = tools::template(&project, &sources(&args, &base)?, &kw);
    let text = tools::write(&template);
    match out {
        Some(o) => {
            write(Path::new(&o), &text)?;
            println!("{o}: {} texts", template.len() - 1);
        }
        None => print!("{text}"),
    }
    Ok(ExitCode::SUCCESS)
}

fn pseudo_cmd(args: &[String]) -> Result<ExitCode, String> {
    let mut args = args.to_vec();
    let out = take(&mut args, &["-o", "--output"]);
    let pot = args.first().ok_or(USAGE)?;
    let template = po::read_messages(&read(Path::new(pot))?);
    let text = tools::write(&tools::pseudo_catalog(&template, "pseudo"));
    match out {
        Some(o) => write(Path::new(&o), &text)?,
        None => print!("{text}"),
    }
    Ok(ExitCode::SUCCESS)
}

/// A language's tag from its file's name (`es.po`, `pt-BR.po`).
fn tag_of(path: &Path) -> String {
    rapidr_i18n::normalize_tag(path.file_stem().and_then(|s| s.to_str()).unwrap_or(""))
}

fn update_cmd(args: &[String]) -> Result<ExitCode, String> {
    let pot = args.first().ok_or(USAGE)?;
    let template = po::read_messages(&read(Path::new(pot))?);
    let project = template.iter().find(|m| m.is_header()).and_then(|h| po::header_value(h, "project-id-version")).unwrap_or_default();
    for f in &args[1..] {
        let path = Path::new(f);
        let tag = tag_of(path);
        let file = if path.exists() { po::read_messages(&read(path)?) } else { Vec::new() };
        let (new, stats) = tools::update(&template, &file, &project, &tag, "nplurals=2; plural=(n != 1);");
        write(path, &tools::write(&new))?;
        println!("{f}: {} kept, {} new{}", stats.kept, stats.added, if stats.dropped.is_empty() { String::new() } else { format!(", {} no longer used: {}", stats.dropped.len(), stats.dropped.join(" | ")) });
    }
    Ok(ExitCode::SUCCESS)
}

/// Each problem of `file` (language `tag`) printed; how many.
fn report_problems(name: &str, tag: &str, file: &[Message]) -> usize {
    let problems = tools::check(tag, file);
    for (id, ps) in &problems {
        for p in ps {
            println!("{name}: {id:?}: {p}");
        }
    }
    problems.len()
}

fn check_cmd(args: &[String]) -> Result<ExitCode, String> {
    if args.is_empty() {
        return Err(USAGE.to_string());
    }
    let mut bad = 0;
    for f in args {
        let path = Path::new(f);
        bad += report_problems(f, &tag_of(path), &po::read_messages(&read(path)?));
    }
    println!("{} file(s) checked, {bad} translation(s) to fix", args.len());
    Ok(if bad == 0 { ExitCode::SUCCESS } else { ExitCode::from(1) })
}

// ------------------------------------------------------------ RapidR's own --

/// The checkout's root: the folder with `ide/studio.rr` (RapidR's home,
/// else the current folder or one above it).
fn root() -> Result<PathBuf, String> {
    let mut cands: Vec<PathBuf> = crate::home::Home::find().map(|h| h.root).into_iter().collect();
    if let Ok(cwd) = std::env::current_dir() {
        cands.extend(cwd.ancestors().map(Path::to_path_buf));
    }
    cands.into_iter().find(|r| r.join("ide/studio.rr").is_file() && r.join("crates").is_dir()).ok_or_else(|| "rapidr i18n: run it in RapidR's checkout (no ide/studio.rr here)".to_string())
}

/// RapidR Studio's sources: `ide/studio.rr` and every `ide/*.inc`, sorted.
pub fn studio_sources(root: &Path) -> Result<Vec<(String, String)>, String> {
    let ide = root.join("ide");
    let mut names: Vec<String> = fs::read_dir(&ide).map_err(|e| format!("{}: {e}", ide.display()))?.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.ends_with(".inc") || n == "studio.rr").collect();
    names.sort();
    names.iter().map(|n| read(&ide.join(n)).map(|t| (format!("ide/{n}"), t))).collect()
}

/// Studio's template, as `ide/lang/studio.pot` should hold it.
pub fn studio_template(root: &Path) -> Result<Vec<Message>, String> {
    Ok(tools::template("RapidR Studio", &studio_sources(root)?, &[]))
}

/// Writes `text` to `path` unless it holds it already; with `check`,
/// writes nothing and says whether it would. True: up to date.
fn generated(path: &Path, text: &str, check: bool) -> Result<bool, String> {
    let same = fs::read_to_string(path).is_ok_and(|t| t == text);
    if !same && !check {
        write(path, text)?;
        println!("wrote {}", path.display());
    }
    Ok(same || !check)
}

/// The languages a folder of catalogs has (`es.po` …), the pseudo-language
/// aside, sorted.
fn languages_in(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "po"))
        .map(|p| (tag_of(&p), p))
        .filter(|(t, _)| t != rapidr_i18n::pseudo::TAG)
        .collect();
    out.sort();
    out
}

fn studio_cmd(args: &[String]) -> Result<ExitCode, String> {
    let mut args = args.to_vec();
    let check = take_flag(&mut args, "--check");
    let root = root()?;
    let template = studio_template(&root)?;
    let lang = root.join("ide/lang");
    let mut ok = generated(&lang.join("studio.pot"), &tools::write(&template), check)?;
    // (the pseudo-language: generated, never committed — .gitignore)
    if !check {
        generated(&lang.join("xx.po"), &tools::write(&tools::pseudo_catalog(&template, "RapidR Studio")), false)?;
    }
    for (tag, path) in languages_in(&lang) {
        let file = po::read_messages(&read(&path)?);
        let c = tools::coverage(&template, &file);
        println!("{}: {} of {} texts ({:.1}%), {} to review, {} fuzzy", path.display(), c.translated, c.total, c.percent(), c.review, c.fuzzy);
        if report_problems(&path.to_string_lossy(), &tag, &file) > 0 {
            ok = false;
        }
        let clashes = studio_mnemonic_clashes(&root, &file)?;
        for (menu, k, captions) in &clashes {
            println!("{}: the {menu} menu: '{k}' is the mnemonic of {}", path.display(), captions.join(", "));
        }
        if !clashes.is_empty() {
            ok = false;
        }
    }
    println!("{} texts in ide/lang/studio.pot{}", template.len() - 1, if ok { "" } else { " — out of date or translations to fix (above)" });
    Ok(if ok { ExitCode::SUCCESS } else { ExitCode::from(1) })
}

/// The menus' captions in a language (commands.inc's command table: each
/// `AddCmd`'s title under its category, the top menus'), and the letters
/// two items of one menu share.
pub fn studio_mnemonic_clashes(root: &Path, file: &[Message]) -> Result<Vec<(String, char, Vec<String>)>, String> {
    Ok(tools::studio_mnemonic_clashes(&read(&root.join("ide/commands.inc"))?, file))
}

/// The compiler catalog's template: the messages, then the hints under
/// them (`hint.<id>`, rapidr_langsvc::foreign) — one file per language
/// translates both.
pub fn compiler_template() -> Vec<Message> {
    let mut template = rapidr_diagnostics::messages::template();
    for (key, en) in rapidr_langsvc::foreign::catalog_entries() {
        let note = if key == "hint.label" { "The word that labels a hint under a compiler message" } else { "A hint under a compiler message: what to write in RapidR instead (code stays as written)" };
        template.push(Message { context: key, id: en, notes: vec![note.to_string()], ..Default::default() });
    }
    template
}

fn compiler_cmd(args: &[String]) -> Result<ExitCode, String> {
    let check = args.iter().any(|a| a == "--check");
    let root = root()?;
    let template = compiler_template();
    let ok = generated(&root.join("crates/rapidr-diagnostics/lang/compiler.pot"), &tools::write(&template), check)?;
    println!("{} compiler messages", template.len() - 1);
    Ok(if ok { ExitCode::SUCCESS } else { ExitCode::from(1) })
}

/// The Rust sources of RapidR's crates (crates/*/src, interpreter/*/src),
/// sorted, as (path from the root, text).
fn rust_sources(root: &Path) -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    for top in ["crates", "interpreter"] {
        for c in fs::read_dir(root.join(top)).into_iter().flatten().flatten() {
            walk(&c.path().join("src"), &mut files);
        }
    }
    files.sort();
    files.into_iter().filter_map(|f| fs::read_to_string(&f).ok().map(|t| (f.strip_prefix(root).unwrap_or(&f).to_string_lossy().replace('\\', "/"), t))).filter(|(_, t)| t.contains("tr_ui")).collect()
}

/// RapidR's own text's template, as crates/rapidr-value/lang/runtime.pot
/// should hold it.
pub fn runtime_template(root: &Path) -> Vec<Message> {
    tools::rust_template("RapidR runtime", &rust_sources(root))
}

fn runtime_cmd(args: &[String]) -> Result<ExitCode, String> {
    let check = args.iter().any(|a| a == "--check");
    let root = root()?;
    let template = runtime_template(&root);
    let mut ok = generated(&root.join("crates/rapidr-value/lang/runtime.pot"), &tools::write(&template), check)?;
    for tag in rapidr_value::i18n::runtime_languages() {
        let file = po::read_messages(rapidr_value::i18n::runtime_catalog_text(tag).unwrap_or(""));
        let c = tools::coverage(&template, &file);
        println!("crates/rapidr-value/lang/{tag}.po: {} of {} texts ({:.1}%), {} to review", c.translated, c.total, c.percent(), c.review);
        if report_problems(&format!("crates/rapidr-value/lang/{tag}.po"), tag, &file) > 0 {
            ok = false;
        }
    }
    println!("{} texts of RapidR's own in crates/rapidr-value/lang/runtime.pot", template.len() - 1);
    Ok(if ok { ExitCode::SUCCESS } else { ExitCode::from(1) })
}

/// `manual-stamp es getting-started.md`: docs/manual/es/getting-started.md's
/// first line says which English it translates (its hash), so coverage
/// can tell when the English changed since.
fn stamp_cmd(args: &[String]) -> Result<ExitCode, String> {
    let (Some(lang), Some(page)) = (args.first(), args.get(1)) else { return Err(USAGE.to_string()) };
    let root = root()?;
    let english = read(&root.join("docs/manual").join(page))?;
    let path = root.join("docs/manual").join(lang).join(page);
    let text = read(&path)?;
    let line = format!("<!-- translated-from: {page} {} -->", text_hash(&english));
    let body = if text.starts_with("<!-- translated-from:") { text.split_once('\n').map(|(_, r)| r.to_string()).unwrap_or_default() } else { text };
    write(&path, &format!("{line}\n{body}"))?;
    println!("{}: {line}", path.display());
    Ok(ExitCode::SUCCESS)
}

/// A short hash of a text (FNV-1a, 64 bits, hex): what a translated
/// manual page records of the English it was made from.
pub fn text_hash(text: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.replace("\r\n", "\n").bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// A translated manual page's record of its English
/// (`<!-- translated-from: getting-started.md 1a2b… -->`): the hash.
fn translated_from(text: &str) -> Option<String> {
    let i = text.find("<!-- translated-from:")?;
    let rest = &text[i + "<!-- translated-from:".len()..];
    let end = rest.find("-->")?;
    rest[..end].split_whitespace().nth(1).map(String::from)
}

fn coverage_cmd(args: &[String]) -> Result<ExitCode, String> {
    let root = root()?;
    let mut langs: Vec<String> = args.iter().map(|a| rapidr_i18n::normalize_tag(a)).collect();
    if langs.is_empty() {
        let mut all: Vec<String> = languages_in(&root.join("ide/lang")).into_iter().map(|(t, _)| t).collect();
        all.extend(rapidr_diagnostics::messages::languages().iter().map(|s| s.to_string()));
        all.extend(rapidr_lang::docs_i18n::doc_languages().iter().map(|s| s.to_string()));
        if let Ok(rd) = fs::read_dir(root.join("docs/manual")) {
            // (a language's folder: it translates a page the manual has)
            let translates = |dir: &Path| fs::read_dir(dir).into_iter().flatten().flatten().any(|f| f.file_name().to_string_lossy().ends_with(".md") && root.join("docs/manual").join(f.file_name()).is_file());
            all.extend(rd.filter_map(|e| e.ok()).filter(|e| e.path().is_dir() && translates(&e.path())).map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.len() <= 7 && n != "reference"));
        }
        all.sort();
        all.dedup();
        langs = all;
    }
    // (Studio's text: its own catalog's and RapidR's own that it shows)
    let studio = studio_template(&root)?;
    let runtime = runtime_template(&root);
    let compiler = compiler_template();
    let manual: Vec<String> = {
        let mut v: Vec<String> = fs::read_dir(root.join("docs/manual")).map_err(|e| e.to_string())?.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.ends_with(".md")).collect();
        v.sort();
        v
    };
    let docs = rapidr_lang::docs_i18n::doc_keys();
    println!("RapidR in every language: what each language has (English is the source)\n");
    println!("{:<8} {:>22} {:>22} {:>22} {:>22}", "", "Studio's text", "compiler messages", "manual pages", "registry docs");
    for tag in &langs {
        let pct = |n: usize, t: usize| if t == 0 { 0.0 } else { n as f64 * 100.0 / t as f64 };
        let cell = |n: usize, t: usize| format!("{n}/{t} {:.1}%", pct(n, t));
        let studio_file = root.join("ide/lang").join(format!("{tag}.po"));
        let mut s = if studio_file.exists() { tools::coverage(&studio, &po::read_messages(&read(&studio_file)?)) } else { tools::Coverage { total: studio.len() - 1, ..Default::default() } };
        let r = tools::coverage(&runtime, &po::read_messages(rapidr_value::i18n::runtime_catalog_text(tag).unwrap_or("")));
        s.total += r.total;
        s.translated += r.translated;
        s.review += r.review;
        let c = rapidr_diagnostics::messages::catalog_text(tag).map(|t| tools::coverage(&compiler, &po::read_messages(t))).unwrap_or(tools::Coverage { total: compiler.len() - 1, ..Default::default() });
        let (mut pages, mut outdated) = (0, Vec::new());
        for page in &manual {
            let tr = root.join("docs/manual").join(tag).join(page);
            if let Ok(text) = fs::read_to_string(&tr) {
                pages += 1;
                let english = fs::read_to_string(root.join("docs/manual").join(page)).unwrap_or_default();
                if translated_from(&text).is_some_and(|h| h != text_hash(&english)) {
                    outdated.push(page.clone());
                }
            }
        }
        let d = docs.iter().filter(|(k, _)| rapidr_lang::docs_i18n::translated_doc(k, tag).is_some()).count();
        println!("{tag:<8} {:>22} {:>22} {:>22} {:>22}", cell(s.translated, s.total), cell(c.translated, c.total), cell(pages, manual.len()), cell(d, docs.len()));
        if s.review + c.review > 0 {
            println!("{:<8} ({} of Studio's and {} of the compiler's still marked to review)", "", s.review, c.review);
        }
        if !outdated.is_empty() {
            println!("{:<8} manual pages whose English changed since they were translated: {}", "", outdated.join(", "));
        }
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_are_stable() {
        assert_eq!(text_hash("a\r\nb"), text_hash("a\nb"));
        assert_eq!(translated_from("x\n<!-- translated-from: getting-started.md 00ff -->\n").as_deref(), Some("00ff"));
    }
}
