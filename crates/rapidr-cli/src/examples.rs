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
    /// `basics/hello.rr` (any case, `\` or `/`).
    fn is(&self, wanted: &str) -> bool {
        let wanted = wanted.replace('\\', "/").to_lowercase();
        let name = self.name().to_lowercase();
        let no_ext = name.rsplit_once('.').map_or(name.clone(), |(n, _)| n.to_string());
        [name.as_str(), no_ext.as_str()].contains(&wanted.as_str())
            || wanted == self.stem().to_lowercase()
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
                println!("  {:width$}  {}", e.name(), summary(&dir.join(&e.rel)));
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

/// One example into `to`: the program, and the files beside it that it
/// names (its data: `$RESOURCE … AS "sprites.dxg"`, "staff.csv").
fn copy_one(dir: &Path, e: &Example, to: &Path) -> Result<Vec<PathBuf>, String> {
    let src = dir.join(&e.rel);
    let text = fs::read_to_string(&src).map_err(|err| format!("{}: {err}", src.display()))?;
    let mut files = vec![src.clone()];
    if let Some(parent) = src.parent() {
        let mut data: Vec<PathBuf> = fs::read_dir(parent)
            .map_err(|err| format!("{}: {err}", parent.display()))?
            .flatten()
            .map(|d| d.path())
            .filter(|p| p.is_file() && !is_program(p))
            .filter(|p| p.file_name().is_some_and(|f| text.contains(&*f.to_string_lossy())))
            .collect();
        data.sort();
        files.extend(data);
    }
    copy_files(&files, |f| to.join(f.file_name().unwrap_or_default()))
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
    copy_files(&files, |f| to.join(f.strip_prefix(dir).unwrap_or(f)))
}

/// `files` to where `dest` puts each — none of them over a file that's
/// already there (then nothing is copied).
fn copy_files(files: &[PathBuf], dest: impl Fn(&Path) -> PathBuf) -> Result<Vec<PathBuf>, String> {
    let pairs: Vec<(PathBuf, PathBuf)> = files.iter().map(|f| (f.clone(), dest(f))).collect();
    let there: Vec<String> = pairs.iter().filter(|(_, d)| d.exists()).map(|(_, d)| d.display().to_string()).collect();
    if !there.is_empty() {
        return Err(format!("already there, not overwritten (nothing was copied): {}", there.join(", ")));
    }
    for (src, dst) in &pairs {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).map_err(|err| format!("{}: {err}", parent.display()))?;
        }
        fs::copy(src, dst).map_err(|err| format!("{} to {}: {err}", src.display(), dst.display()))?;
    }
    Ok(pairs.into_iter().map(|(_, d)| d).collect())
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
        fs::write(dir.join("media/song.rr"), "' song.rr: a song with QMIDI. More here.\n$RESOURCE T AS \"tune.mid\"\n").unwrap();
        fs::write(dir.join("media/tune.mid"), "x").unwrap();
        fs::write(dir.join("media/other.wav"), "y").unwrap();
        fs::write(dir.join("README.md"), "# index").unwrap();
        assert_eq!(summary(&dir.join("media/song.rr")), "a song with QMIDI");
        let all = list(&dir);
        assert_eq!(all.iter().map(Example::name).collect::<Vec<_>>(), ["media/song.rr"]);
        // one: the program and the data it names, not the folder's others
        let out = dir.join("out");
        let copied = copy_one(&dir, &all[0], &out).unwrap();
        assert_eq!(copied, [out.join("song.rr"), out.join("tune.mid")]);
        // never over a file that's there
        assert!(copy_one(&dir, &all[0], &out).unwrap_err().contains("already there"));
        // all: by topic, with the index
        let every = dir.join("every");
        copy_all(&dir, &all, &every).unwrap();
        assert!(every.join("README.md").is_file() && every.join("media/other.wav").is_file() && every.join("media/song.rr").is_file());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_checkouts_examples_are_found() {
        let dir = folder().unwrap();
        let all = list(&dir);
        assert!(all.iter().any(|e| e.name() == "basics/hello.rr"));
        assert!(all.iter().all(|e| !summary(&dir.join(&e.rel)).is_empty()), "every example's first comment line says what it is");
    }
}
