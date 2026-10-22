//! `rapidr examples`: the example programs RapidR comes with — listed, and
//! copied into a folder of the user's to run and change.
//!
//! They are in RapidR's home (home.rs' rule): a checkout's `examples/`, an
//! SDK install's `lib/rapidr/examples/` (`tools/release/stage.py` puts them
//! there), a topic a folder (`basics/hello.rr`). The index is
//! `examples/README.md`; each program's first comment line says what it is.
//!
//!   rapidr examples                       the list
//!   rapidr examples copy <name> [folder]  one example (and the data files
//!                                         it names) into the folder
//!   rapidr examples copy all [folder]     every example, by topic
//!
//! The folder is `rapidr-examples` in the current one unless given; files
//! already there are never overwritten.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rapidr_project::program_files::{self, Existing};

use crate::home::Home;

/// An example: its path under the examples folder (`basics/hello.rr`).
struct Example {
    rel: PathBuf,
}

impl Example {
    /// `basics/hello.rr`, with `/` on every system.
    fn name(&self) -> String {
        self.rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/")
    }

    /// `hello`.
    fn stem(&self) -> String {
        self.rel.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
    }

    /// Whether `wanted` names it: `hello`, `hello.rr`, `basics/hello` or
    /// `basics/hello.rr` — and one in a folder of its own by its topic
    /// (`rapidq/shapes` for `rapidq/shapes/shapes.bas`) — any case, `\` or
    /// `/`.
    fn is(&self, wanted: &str) -> bool {
        let wanted = wanted.replace('\\', "/").trim_end_matches('/').to_lowercase();
        let name = self.name().to_lowercase();
        let no_ext = name.rsplit_once('.').map_or(name.clone(), |(n, _)| n.to_string());
        let stem = self.stem().to_lowercase();
        let topic = name.split('/').next().unwrap_or("").to_string();
        let topic_form = format!("{topic}/{stem}");
        [name.as_str(), no_ext.as_str(), stem.as_str()].contains(&wanted.as_str())
            || (name.contains('/') && wanted == topic_form)
            || self.rel.file_name().is_some_and(|f| f.to_string_lossy().to_lowercase() == wanted)
    }
}

/// The examples folder: the home's `examples/`.
fn folder() -> Result<PathBuf, String> {
    let home = Home::find().ok_or("RapidR's home was not found (set RAPIDR_HOME)")?;
    let dir = home.root.join("examples");
    if dir.is_dir() {
        Ok(dir)
    } else {
        Err(format!("{}: not found — the examples come with the RapidR SDK, not the Runtime", dir.display()))
    }
}

/// Every program under `dir` (`.rr`, `.bas`), sorted by path.
fn list(dir: &Path) -> Vec<Example> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<Example>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for e in entries.flatten() {
            let path = e.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else if is_program(&path) {
                if let Ok(rel) = path.strip_prefix(root) {
                    out.push(Example { rel: rel.to_path_buf() });
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    // (by topic, then name; the ones at the top — the IDE — last)
    out.sort_by_key(|e| (e.rel.components().count() == 1, e.name()));
    out
}

fn is_program(path: &Path) -> bool {
    path.extension().is_some_and(|x| x.eq_ignore_ascii_case("rr") || x.eq_ignore_ascii_case("bas"))
}

/// What the program's opening comment says first, after its name
/// (`' hello.rr: the first RapidR program, in a console.` → "the first
/// RapidR program, in a console"): up to the end of its first sentence or
/// clause.
fn summary(path: &Path) -> String {
    let text = fs::read_to_string(path).unwrap_or_default();
    // (the first paragraph of comment lines: until an empty one)
    let words: Vec<&str> = text
        .lines()
        .map(str::trim)
        .skip_while(|l| !l.starts_with('\'') || l.trim_start_matches(['\'', '=', ' ']).is_empty())
        .take_while(|l| l.starts_with('\'') && !l.trim_start_matches(['\'', ' ']).is_empty())
        .map(|l| l.trim_start_matches('\'').trim())
        .collect();
    let para = words.join(" ");
    let file = path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
    let para = para.strip_prefix(&format!("{file}:")).unwrap_or(&para).trim();
    let end = [". ", "; ", " — ", ": "].iter().filter_map(|s| para.find(s)).min().unwrap_or(para.len());
    para[..end].trim_end_matches(['.', ',', ' ']).to_string()
}

/// `text` with each `docs/manual/<page>` it names made the manual's page
/// where it is: an install's (`<prefix>/share/doc/rapidr/manual/`, beside
/// its `lib/rapidr/examples`), a checkout's, else on the web.
fn manual_pages(text: &str, examples: &Path) -> String {
    const NAMED: &str = "docs/manual/";
    let Some(at) = text.find(NAMED) else { return text.to_string() };
    let rest = &text[at + NAMED.len()..];
    let len = rest.find(|c: char| !(c.is_ascii_alphanumeric() || "-_./".contains(c))).unwrap_or(rest.len());
    let page = rest[..len].trim_end_matches('.');
    let home = examples.parent().unwrap_or(Path::new("."));
    let installed = home.parent().and_then(Path::parent).map(|prefix| prefix.join("share").join("doc").join("rapidr").join("manual").join(page));
    let place = match installed.into_iter().chain([home.join("docs").join("manual").join(page)]).find(|p| p.exists()) {
        Some(p) => p.display().to_string(),
        None => format!("https://github.com/iBobX/RapidR/blob/main/docs/manual/{page}"),
    };
    format!("{}{place}{}", &text[..at], manual_pages(&rest[page.len()..], examples))
}

pub fn command(args: &[String]) -> ExitCode {
    let dir = match folder() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    let examples = list(&dir);
    match args.first().map(String::as_str) {
        None | Some("list") => {
            let width = examples.iter().map(|e| e.name().len()).max().unwrap_or(0);
            for e in &examples {
                println!("  {:width$}  {}", e.name(), manual_pages(&summary(&dir.join(&e.rel)), &dir));
            }
            println!();
            println!("{} examples in {} (examples/README.md: what each shows, where it runs).", examples.len(), dir.display());
            println!("  rapidr examples copy <name> [folder]    copy one, e.g. `rapidr examples copy hello`");
            println!("  rapidr examples copy all [folder]       copy them all (default folder: ./rapidr-examples)");
            ExitCode::SUCCESS
        }
        Some("copy") if args.len() >= 2 && args.len() <= 3 => {
            let to = PathBuf::from(args.get(2).map_or("rapidr-examples", String::as_str));
            let result = if args[1].eq_ignore_ascii_case("all") {
                copy_all(&dir, &examples, &to)
            } else {
                match examples.iter().find(|e| e.is(&args[1])) {
                    Some(e) => copy_one(&dir, e, &to),
                    None => Err(format!("no example '{}' (rapidr examples lists them)", args[1])),
                }
            };
            match result {
                Ok(copied) => {
                    for f in &copied {
                        println!("copied {}", f.display());
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::from(1)
                }
            }
        }
        _ => {
            eprintln!("usage: rapidr examples [list]");
            eprintln!("       rapidr examples copy <name|all> [folder]");
            ExitCode::from(2)
        }
    }
}

/// One example into `to`: the program, the files it includes from its
/// folder (as they are there: `lib/geometry.inc`), and the files beside it
/// that it names (its data: `$RESOURCE … AS "sprites.dxg"`, "staff.csv") —
/// the set RapidR Studio copies too (`rapidr_project::program_files`).
fn copy_one(dir: &Path, e: &Example, to: &Path) -> Result<Vec<PathBuf>, String> {
    program_files::copy_program(&dir.join(&e.rel), to, Existing::Refuse)
}

/// Every example, by topic folder, and the index.
fn copy_all(dir: &Path, examples: &[Example], to: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files: Vec<PathBuf> = Vec::new();
    let mut folders: Vec<PathBuf> = vec![dir.to_path_buf()];
    for e in examples {
        if let Some(parent) = dir.join(&e.rel).parent() {
            if !folders.iter().any(|f| f == parent) {
                folders.push(parent.to_path_buf());
            }
        }
    }
    for folder in &folders {
        let mut here: Vec<PathBuf> = fs::read_dir(folder).map_err(|err| format!("{}: {err}", folder.display()))?.flatten().map(|d| d.path()).filter(|p| p.is_file()).collect();
        here.sort();
        files.extend(here);
    }
    // (and what each takes from below its folder: what it includes,
    // rapidq/shapes/lib/…, and the files it names there, gui/icons/*.svg —
    // all of `files_of`, as `copy <name>` copies)
    for e in examples {
        for f in program_files::files_of(&dir.join(&e.rel)).unwrap_or_default() {
            if !files.contains(&f) {
                files.push(f);
            }
        }
    }
    program_files::copy_files(&files, |f| to.join(f.strip_prefix(dir).unwrap_or(f)), Existing::Refuse)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_summaries() {
        let e = Example { rel: PathBuf::from("basics").join("hello.rr") };
        assert_eq!(e.name(), "basics/hello.rr");
        assert!(e.is("hello") && e.is("HELLO.rr") && e.is("basics/hello") && e.is("basics\\hello.rr"));
        assert!(!e.is("hell") && !e.is("gui/hello"));
        let dir = std::env::temp_dir().join(format!("rapidr-examples-test-{}", std::process::id()));
        fs::create_dir_all(dir.join("media")).unwrap();
        fs::write(dir.join("media/song.rr"), "' song.rr: a song with QMIDI. More here.\n$RESOURCE T AS \"tune.mid\"\n$RESOURCE ICON AS \"art/note.svg\"\n").unwrap();
        fs::create_dir_all(dir.join("media/art")).unwrap();
        fs::write(dir.join("media/art/note.svg"), "<svg/>").unwrap();
        fs::write(dir.join("media/tune.mid"), "x").unwrap();
        fs::write(dir.join("media/other.wav"), "y").unwrap();
        fs::write(dir.join("README.md"), "# index").unwrap();
        assert_eq!(summary(&dir.join("media/song.rr")), "a song with QMIDI");
        let all = list(&dir);
        assert_eq!(all.iter().map(Example::name).collect::<Vec<_>>(), ["media/song.rr"]);
        // one: the program and the data it names, not the folder's others
        let out = dir.join("out");
        let copied = copy_one(&dir, &all[0], &out).unwrap();
        assert_eq!(copied, [out.join("song.rr"), out.join("tune.mid"), out.join("art/note.svg")]);
        // never over a file that's there
        assert!(copy_one(&dir, &all[0], &out).unwrap_err().contains("already there"));
        // all: by topic, with the index
        let every = dir.join("every");
        copy_all(&dir, &all, &every).unwrap();
        assert!(every.join("README.md").is_file() && every.join("media/other.wav").is_file() && every.join("media/song.rr").is_file());
        // (and the files it names below its folder: its $RESOURCE's picture)
        assert!(every.join("media/art/note.svg").is_file());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_checkouts_examples_are_found() {
        let dir = folder().unwrap();
        let all = list(&dir);
        assert!(all.iter().any(|e| e.name() == "basics/hello.rr"));
        assert!(all.iter().all(|e| !summary(&dir.join(&e.rel)).is_empty()), "every example's first comment line says what it is");
        // (an example in a folder of its own, by its topic too)
        let shapes = all.iter().find(|e| e.name() == "rapidq/shapes/shapes.bas").unwrap();
        assert!(shapes.is("rapidq/shapes") && shapes.is("rapidq/shapes/") && shapes.is("shapes") && shapes.is("RapidQ\\Shapes"));
        assert!(!shapes.is("gui/shapes"));
        // (the manual's pages where they are: this checkout's)
        let shop = manual_pages(&summary(&dir.join("debugging/shop.rr")), &dir);
        assert!(!shop.contains(" (docs/manual/") && shop.contains(&dir.parent().unwrap().join("docs").join("manual").join("debugging.md").display().to_string()), "{shop}");
        let elsewhere = manual_pages("see docs/manual/nowhere.md.", Path::new("/no/such/examples"));
        assert_eq!(elsewhere, "see https://github.com/iBobX/RapidR/blob/main/docs/manual/nowhere.md.");
    }
}
