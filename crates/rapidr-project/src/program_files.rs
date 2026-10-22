//! A program's own files on disk, as one set for everything that moves a
//! program somewhere else: the program, the files it `$INCLUDE`s from its
//! folder or below it (`lib/geometry.inc`, as the compiler finds them) and
//! the files beside it that its source names (its data: `"staff.csv"`,
//! `$RESOURCE … AS "sprites.dxg"`).
//!
//! - `rapidr examples copy` copies them ([`copy_program`]);
//! - RapidR Studio opens an example installed where it can't be changed
//!   (`/usr/lib/rapidr/examples`, the macOS app, Program Files) as a copy
//!   of them in the user's `Documents/RapidR/Examples/<name>/`
//!   ([`example_to_open`]), and copies them again on request
//!   ([`copy_example_again`]);
//! - `rapidr build` puts the data files ([`data_files_of`]) next to the
//!   program it makes, so the app finds them wherever it's started.
//!
//! The desktop's: the web has no folders to copy between (its page's
//! store keeps what the user saves over the file it opened).

use std::fs;
use std::path::{Path, PathBuf};

/// Whether `path` is a program's source (`.rr`, `.bas`, RapidQ's `.rqw`,
/// `.rqb`, `.rq`): never a data file of another program.
pub fn is_program(path: &Path) -> bool {
    path.extension().is_some_and(|x| ["rr", "bas", "rqw", "rqb", "rq"].iter().any(|e| x.eq_ignore_ascii_case(e)))
}

/// The files program `src` `$INCLUDE`s (however deep) that are in its own
/// folder or below it (`lib/geometry.inc`), as the compiler finds them.
pub fn includes_of(src: &Path) -> Vec<PathBuf> {
    let Some(folder) = src.parent() else { return Vec::new() };
    let Ok(pre) = rapidr_preprocessor::preprocess_file(src, rapidr_preprocessor::PreprocessOptions::default()) else { return Vec::new() };
    let mut out: Vec<PathBuf> = Vec::new();
    for u in &pre.includes {
        if let rapidr_preprocessor::IncludeFound::File(p) = &u.found {
            if p.starts_with(folder) && p.is_file() && !out.contains(p) {
                out.push(p.clone());
            }
        }
    }
    out
}

/// The data files of program `src`: the files beside it (not programs, not
/// what it includes) whose names its source — or a file it includes —
/// has in it (`"staff.csv"`, `$RESOURCE Logo AS "logo.png"`), by name.
pub fn data_files_of(src: &Path) -> Vec<PathBuf> {
    let Some(parent) = src.parent() else { return Vec::new() };
    let parent = if parent.as_os_str().is_empty() { Path::new(".") } else { parent };
    let includes = includes_of(src);
    let text = program_text(src, &includes);
    // (a name counts only as a whole string literal — "staff.csv" — and
    // only a name with an extension: a word the program prints, `PRINT
    // "hello"`, isn't the executable `hello` a build left beside it)
    let quoted = quoted_strings(&text);
    let Ok(entries) = fs::read_dir(parent) else { return Vec::new() };
    let mut data: Vec<PathBuf> = entries
        .flatten()
        .map(|d| d.path())
        .filter(|p| p.is_file() && !is_program(p) && !includes.contains(p) && p.extension().is_some())
        .filter(|p| {
            p.file_name().is_some_and(|f| {
                let f = f.to_string_lossy();
                !f.starts_with('.') && quoted.contains(&f.to_lowercase())
            })
        })
        .collect();
    data.sort();
    data
}

/// The text of program `src` and of the files it includes (`includes`):
/// where its files are named.
fn program_text(src: &Path, includes: &[PathBuf]) -> String {
    let mut text = fs::read_to_string(src).unwrap_or_default();
    for inc in includes {
        text.push('\n');
        text.push_str(&String::from_utf8_lossy(&fs::read(inc).unwrap_or_default()));
    }
    text
}

/// What a program takes with it from the folders below its own
/// ([`carried_of`]; the rules: [`crate::ship`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Carried {
    /// The files in folders below the program's that its source names
    /// (`"art/logo.svg"`, `"data\sample.csv"`): (its path from the
    /// program's folder as on disk, `/`-separated; the file).
    pub files: Vec<(String, PathBuf)>,
    /// Its translations when it is translated — it uses `TR$`, `TRN$` or
    /// `Application.Language`, or has a `lang` folder: each `lang/*.po`
    /// (file name; the file), by name. `None`: it isn't translated.
    pub catalogs: Option<Vec<(String, PathBuf)>>,
    /// The files its source names that are outside its folder
    /// (`"../secret.txt"`, `"/etc/hosts"`, a link leading out), said in
    /// plain words: never taken.
    pub warnings: Vec<String>,
}

/// What program `src` takes with it from below its folder: the files there
/// its source (or a file it includes) names, its translations; and what it
/// names outside its folder, as warnings. The files right beside it are
/// [`data_files_of`]'s; programs and its includes are never data.
pub fn carried_of(src: &Path) -> Carried {
    let mut out = Carried::default();
    let Some(folder) = src.parent() else { return out };
    let folder = if folder.as_os_str().is_empty() { Path::new(".") } else { folder };
    let includes = includes_of(src);
    let text = program_text(src, &includes);
    let outside = |name: &str| format!("\"{name}\" is outside the program's folder: not copied (a build takes only files from the program's folder and the folders below it)");
    for (name, _) in crate::named_paths(&text) {
        match crate::ship::place(&name) {
            crate::ship::Place::Beside => {}
            crate::ship::Place::Below(rel) => {
                let Some(found) = rapidr_preprocessor::find_file(folder, &rel) else { continue };
                if is_program(&found) || includes.iter().any(|i| same_path(i, &found)) {
                    continue;
                }
                // (a link inside the folder that leads out of it: the same)
                if !is_inside(&found, folder) {
                    out.warnings.push(outside(&name));
                    continue;
                }
                let to = found.strip_prefix(folder).map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or(rel);
                if !out.files.iter().any(|(t, _)| t.eq_ignore_ascii_case(&to)) {
                    out.files.push((to, found));
                }
            }
            crate::ship::Place::Outside => {
                let unix = name.replace('\\', "/");
                let at = if crate::implicit::is_absolute(&unix) { PathBuf::from(&unix) } else { folder.join(&unix) };
                if at.is_file() && !out.warnings.contains(&outside(&name)) {
                    out.warnings.push(outside(&name));
                }
            }
        }
    }
    // (its translations: the lang folder in any case, its .po files)
    let lang = fs::read_dir(folder).ok().and_then(|entries| {
        entries.flatten().map(|e| e.path()).find(|p| p.is_dir() && p.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(crate::ship::LANG_FOLDER)))
    });
    if lang.is_some() || crate::ship::uses_translations(&text) {
        let mut catalogs: Vec<(String, PathBuf)> = Vec::new();
        for entry in lang.and_then(|l| fs::read_dir(l).ok()).into_iter().flat_map(|d| d.flatten()) {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if !path.is_file() || !crate::ship::is_catalog(&name) {
                continue;
            }
            if !is_inside(&path, folder) {
                out.warnings.push(outside(&format!("{}/{name}", crate::ship::LANG_FOLDER)));
                continue;
            }
            catalogs.push((name, path));
        }
        catalogs.sort();
        out.catalogs = Some(catalogs);
    }
    out
}

/// The program's string literals ("…" on one line), lowercased: the file
/// names it can name.
fn quoted_strings(text: &str) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split('"').collect();
        // (the odd pieces are inside quotes; a last one with no closing quote isn't)
        for i in (1..parts.len().saturating_sub(1)).step_by(2) {
            out.insert(parts[i].trim().to_lowercase());
        }
    }
    out
}

/// Every file of program `src`: itself, what it includes from its folder,
/// its data files ([`data_files_of`]), and what it takes from below its
/// folder ([`carried_of`]: `art/logo.svg`, its `lang/*.po`).
pub fn files_of(src: &Path) -> Result<Vec<PathBuf>, String> {
    if !src.is_file() {
        return Err(format!("{}: no such file", src.display()));
    }
    let mut files = vec![src.to_path_buf()];
    let carried = carried_of(src);
    let below = carried.files.into_iter().chain(carried.catalogs.unwrap_or_default()).map(|(_, f)| f);
    for f in includes_of(src).into_iter().chain(data_files_of(src)).chain(below) {
        if !files.contains(&f) {
            files.push(f);
        }
    }
    Ok(files)
}

/// What a copy does with a file that's there already.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Existing {
    /// Nothing is copied: an error names them (`rapidr examples copy`).
    Refuse,
    /// Those stay as they are; the others are copied.
    Keep,
    /// Copied over (asked for: Copy Again from the Original).
    Replace,
}

/// `files` to where `dest` puts each; what was copied.
pub fn copy_files(files: &[PathBuf], dest: impl Fn(&Path) -> PathBuf, existing: Existing) -> Result<Vec<PathBuf>, String> {
    let pairs: Vec<(PathBuf, PathBuf)> = files.iter().map(|f| (f.clone(), dest(f))).collect();
    if existing == Existing::Refuse {
        let there: Vec<String> = pairs.iter().filter(|(_, d)| d.exists()).map(|(_, d)| d.display().to_string()).collect();
        if !there.is_empty() {
            return Err(format!("already there, not overwritten (nothing was copied): {}", there.join(", ")));
        }
    }
    let mut copied = Vec::new();
    for (src, dst) in &pairs {
        if existing == Existing::Keep && dst.exists() {
            continue;
        }
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).map_err(|err| format!("{}: {err}", parent.display()))?;
        }
        // (a copy of a read-only install is the user's to change: never
        // read-only itself)
        let bytes = fs::read(src).map_err(|err| format!("{}: {err}", src.display()))?;
        if dst.exists() {
            make_writable(dst);
        }
        fs::write(dst, bytes).map_err(|err| format!("{}: {err}", dst.display()))?;
        copied.push(dst.clone());
    }
    Ok(copied)
}

fn make_writable(path: &Path) {
    if let Ok(meta) = fs::metadata(path) {
        let mut perms = meta.permissions();
        if perms.readonly() {
            #[allow(clippy::permissions_set_readonly_false)]
            perms.set_readonly(false);
            let _ = fs::set_permissions(path, perms);
        }
    }
}

/// Program `src` and its files ([`files_of`]) into folder `to`, as they are
/// beside it (`lib/geometry.inc` stays in `lib/`); what was copied.
pub fn copy_program(src: &Path, to: &Path, existing: Existing) -> Result<Vec<PathBuf>, String> {
    let files = files_of(src)?;
    let parent = src.parent().map(Path::to_path_buf).unwrap_or_default();
    copy_files(&files, |f| to.join(f.strip_prefix(&parent).map(Path::to_path_buf).unwrap_or_else(|_| PathBuf::from(f.file_name().unwrap_or_default()))), existing)
}

/// Whether files can be made in folder `dir` (made if it isn't there): a
/// file written there and taken away again — what the system really allows
/// (permissions, a read-only disk, a signed app's bundle), not what its
/// flags say.
pub fn can_write_in(dir: &Path) -> bool {
    if fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(format!(".rapidr-write-test-{}", std::process::id()));
    let ok = fs::write(&probe, b"").is_ok();
    let _ = fs::remove_file(&probe);
    ok
}

/// The user's Documents folder: `RAPIDR_DOCUMENTS` when it's set (the
/// tests', so nothing lands in a real one), else `Documents` in the home
/// folder. Builds that can't go beside their program go to its
/// `RapidR/Builds/`, examples' copies to its `RapidR/Examples/`.
pub fn documents_folder() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("RAPIDR_DOCUMENTS").filter(|d| !d.is_empty()) {
        return Some(PathBuf::from(d));
    }
    std::env::home_dir().map(|h| h.join("Documents"))
}

/// Where the copy of example `src` goes: `Documents/RapidR/Examples/<its
/// name>/` (`million_rows`, `shapes`).
pub fn example_copy_folder(src: &Path) -> Option<PathBuf> {
    let stem = src.file_stem()?.to_string_lossy().into_owned();
    Some(documents_folder()?.join("RapidR").join("Examples").join(stem))
}

/// What [`example_to_open`] decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExampleOpen {
    /// The example itself (its folder can be written: a checkout).
    InPlace,
    /// Its copy, made now.
    Copied,
    /// Its copy, made before: opened as the user left it.
    Existing,
}

/// The file to open for example `src`: `src` itself where its folder can be
/// written and RapidR isn't `installed` (a checkout's `examples/`), else its
/// copy in `Documents/RapidR/Examples/<name>/` — made now if it isn't there
/// (the program, its includes, its data); a copy that's there is never
/// overwritten (files missing from it are copied again). An install's
/// examples are copied even where the folder could be written (a macOS app
/// the user dragged to Applications: writing inside it would break its
/// signature, and an update would take the changes away).
pub fn example_to_open(src: &Path, installed: bool) -> Result<(PathBuf, ExampleOpen), String> {
    let folder = src.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    if !installed && can_write_in(folder) {
        return Ok((src.to_path_buf(), ExampleOpen::InPlace));
    }
    let to = example_copy_folder(src).ok_or("no Documents folder to copy the example to (no home folder)")?;
    let main = to.join(src.file_name().unwrap_or_default());
    let was_there = main.is_file();
    copy_program(src, &to, Existing::Keep).map_err(|e| format!("can't copy the example to {}: {e}", to.display()))?;
    Ok((main, if was_there { ExampleOpen::Existing } else { ExampleOpen::Copied }))
}

/// Example `src`'s files copied again over its copy (the program, its
/// includes, its data as they come with RapidR; files the user added to
/// the copy stay): the copy's main file.
pub fn copy_example_again(src: &Path) -> Result<PathBuf, String> {
    let to = example_copy_folder(src).ok_or("no Documents folder to copy the example to (no home folder)")?;
    copy_program(src, &to, Existing::Replace).map_err(|e| format!("can't copy the example to {}: {e}", to.display()))?;
    Ok(to.join(src.file_name().unwrap_or_default()))
}

/// The example `copy` is a copy of: a file `Documents/RapidR/Examples/<name>/<file>`
/// whose `<file>` is an example in `examples` (`million_rows.rr` in
/// `data/`). None for any other file.
pub fn example_of_copy(copy: &Path, examples: &Path) -> Option<PathBuf> {
    let file = copy.file_name()?;
    let folder = copy.parent()?;
    let base = documents_folder()?.join("RapidR").join("Examples");
    if !same_path(folder.parent()?, &base) || folder.file_name()? != Path::new(file).file_stem()? {
        return None;
    }
    find_file(examples, file, 0)
}

fn find_file(dir: &Path, name: &std::ffi::OsStr, depth: u8) -> Option<PathBuf> {
    if depth > 4 {
        return None;
    }
    let mut dirs = Vec::new();
    for e in fs::read_dir(dir).ok()?.flatten() {
        let p = e.path();
        if p.is_dir() {
            dirs.push(p);
        } else if p.file_name() == Some(name) {
            return Some(p);
        }
    }
    dirs.sort();
    dirs.iter().find_map(|d| find_file(d, name, depth + 1))
}

/// Whether `a` and `b` are the same folder (each as it is on disk when it's
/// there: links followed, any case where the disk ignores case).
pub fn same_path(a: &Path, b: &Path) -> bool {
    let full = |p: &Path| fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    full(a) == full(b)
}

/// Whether `path` is inside folder `dir` (both as on disk).
pub fn is_inside(path: &Path, dir: &Path) -> bool {
    let full = |p: &Path| fs::canonicalize(p).unwrap_or_else(|_| std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf()));
    full(path).starts_with(full(dir))
}

#[cfg(test)]
mod tests {

    /// (the 2.119 release smoke) A word the program prints isn't a data
    /// file: only a whole quoted name with an extension is.
    #[test]
    fn data_files_are_quoted_names_only() {
        let dir = std::env::temp_dir().join(format!("rapidr-datafiles-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("hello.bas"), "PRINT \"hello \"; COMMAND$(1)\nOPEN \"Staff.CSV\" FOR INPUT AS #1\n").unwrap();
        std::fs::write(dir.join("hello"), b"an executable a build left").unwrap();
        std::fs::write(dir.join("staff.csv"), "a,b\n").unwrap();
        std::fs::write(dir.join("notes.txt"), "not named").unwrap();
        let got: Vec<String> = data_files_of(&dir.join("hello.bas")).iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
        assert_eq!(got, vec!["staff.csv".to_string()]);
        let _ = std::fs::remove_dir_all(&dir);
    }
    use super::*;

    /// (T1b) A program takes the files below its folder that it names, its
    /// translations, and nothing outside its folder.
    #[test]
    fn what_a_program_takes_from_below_its_folder() {
        let dir = scratch("carried");
        let app = dir.join("app");
        for d in ["art", "Data", "lang", "lib", "notes"] {
            fs::create_dir_all(app.join(d)).unwrap();
        }
        fs::write(
            app.join("app.rr"),
            "$INCLUDE \"lib/util.inc\"\nLogo.Load \"art/logo.svg\"\nCSV.FileName = \"data\\Sample.csv\"\n\
             x$ = \"../secret.txt\": y$ = \"art/../../secret.txt\"\nz$ = \"art/missing.png\"\nPRINT TR$(\"Hello\")\n\
             ' \"notes/old.txt\" is in a comment\nw$ = \"/no/such/file.txt\"\n",
        )
        .unwrap();
        fs::write(app.join("lib/util.inc"), "' util\nIcon.Load \"art/icon.png\"\nOther.Load \"lib/util.inc\"\n").unwrap();
        fs::write(app.join("art/logo.svg"), "<svg/>").unwrap();
        fs::write(app.join("art/icon.png"), "png").unwrap();
        fs::write(app.join("art/unnamed.png"), "png").unwrap();
        fs::write(app.join("Data/sample.csv"), "a\n1\n").unwrap();
        fs::write(app.join("notes/old.txt"), "old").unwrap();
        fs::write(app.join("lang/es.po"), "msgid \"Hello\"\nmsgstr \"Hola\"\n").unwrap();
        fs::write(app.join("lang/pt-BR.po"), "").unwrap();
        fs::write(app.join("lang/README.txt"), "not a catalog").unwrap();
        fs::write(app.join("lang/.hidden.po"), "").unwrap();
        fs::write(dir.join("secret.txt"), "secret").unwrap();
        let c = carried_of(&app.join("app.rr"));
        let files: Vec<&str> = c.files.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(files, ["art/logo.svg", "Data/sample.csv", "art/icon.png"], "named, found in any case; not the include, not a comment's");
        let catalogs: Vec<&str> = c.catalogs.as_ref().unwrap().iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(catalogs, ["es.po", "pt-BR.po"]);
        assert_eq!(c.warnings.len(), 2, "{:?}", c.warnings);
        assert!(c.warnings[0].starts_with("\"../secret.txt\" is outside the program's folder: not copied"), "{:?}", c.warnings);
        assert!(c.warnings[1].starts_with("\"art/../../secret.txt\""), "{:?}", c.warnings);
        // (a copy of it carries them, as they are below it)
        let out = dir.join("copy");
        copy_program(&app.join("app.rr"), &out, Existing::Refuse).unwrap();
        for f in ["app.rr", "lib/util.inc", "art/logo.svg", "art/icon.png", "Data/sample.csv", "lang/es.po", "lang/pt-BR.po"] {
            assert!(out.join(f).is_file(), "{f}");
        }
        for f in ["art/unnamed.png", "notes/old.txt", "lang/README.txt", "secret.txt"] {
            assert!(!out.join(f).exists(), "{f}");
        }

        // A program that doesn't translate and has no lang folder: none.
        fs::write(dir.join("plain.rr"), "PRINT \"Language: \"; 1\nLanguage = 2\n").unwrap();
        assert_eq!(carried_of(&dir.join("plain.rr")).catalogs, None);
        // One that translates without a lang folder: an empty list.
        let t = dir.join("t");
        fs::create_dir_all(&t).unwrap();
        fs::write(t.join("t.rr"), "Application.Language = \"es\"\n").unwrap();
        assert_eq!(carried_of(&t.join("t.rr")).catalogs, Some(Vec::new()));
        fs::remove_dir_all(&dir).unwrap();
    }

    /// (T1b) A link in the program's folder that leads out of it is never
    /// followed.
    #[cfg(unix)]
    #[test]
    fn a_link_out_of_the_folder_is_not_taken() {
        let dir = scratch("carried-link");
        let app = dir.join("app");
        fs::create_dir_all(app.join("art")).unwrap();
        fs::create_dir_all(app.join("lang")).unwrap();
        fs::write(dir.join("secret.txt"), "secret").unwrap();
        fs::write(dir.join("secret.po"), "secret").unwrap();
        std::os::unix::fs::symlink(dir.join("secret.txt"), app.join("art/link.txt")).unwrap();
        std::os::unix::fs::symlink(dir.join("secret.po"), app.join("lang/es.po")).unwrap();
        fs::write(app.join("app.rr"), "x$ = \"art/link.txt\"\n").unwrap();
        let c = carried_of(&app.join("app.rr"));
        assert!(c.files.is_empty() && c.catalogs == Some(Vec::new()), "{c:?}");
        assert_eq!(c.warnings.len(), 2, "{:?}", c.warnings);
        fs::remove_dir_all(&dir).unwrap();
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rapidr-program-files-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_programs_files() {
        let dir = scratch("set");
        fs::create_dir_all(dir.join("lib")).unwrap();
        fs::write(dir.join("app.rr"), "$INCLUDE \"lib/util.inc\"\nCSV.FileName = \"staff.csv\"\n").unwrap();
        fs::write(dir.join("lib/util.inc"), "' util\nLogo.LoadFromFile \"logo.png\"\n").unwrap();
        fs::write(dir.join("staff.csv"), "a\n1\n").unwrap();
        fs::write(dir.join("logo.png"), "png").unwrap();
        fs::write(dir.join("other.csv"), "b\n").unwrap();
        fs::write(dir.join("other.rr"), "PRINT \"staff.csv\"\n").unwrap();
        let files = files_of(&dir.join("app.rr")).unwrap();
        let names: Vec<String> = files.iter().map(|f| f.strip_prefix(&dir).unwrap().to_string_lossy().replace('\\', "/")).collect();
        assert_eq!(names, ["app.rr", "lib/util.inc", "logo.png", "staff.csv"], "the program, its include, the data both name — not the folder's others");
        assert_eq!(data_files_of(&dir.join("app.rr")).len(), 2);
        let out = dir.join("out");
        copy_program(&dir.join("app.rr"), &out, Existing::Refuse).unwrap();
        assert!(out.join("lib/util.inc").is_file() && out.join("staff.csv").is_file());
        assert!(copy_program(&dir.join("app.rr"), &out, Existing::Refuse).unwrap_err().contains("already there"));
        // (kept: the user's edit stays; replaced: the original again)
        fs::write(out.join("staff.csv"), "edited\n").unwrap();
        fs::remove_file(out.join("logo.png")).unwrap();
        assert_eq!(copy_program(&dir.join("app.rr"), &out, Existing::Keep).unwrap(), [out.join("logo.png")]);
        assert_eq!(fs::read_to_string(out.join("staff.csv")).unwrap(), "edited\n");
        copy_program(&dir.join("app.rr"), &out, Existing::Replace).unwrap();
        assert_eq!(fs::read_to_string(out.join("staff.csv")).unwrap(), "a\n1\n");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn an_installed_example_opens_as_a_copy() {
        use std::os::unix::fs::PermissionsExt;
        let dir = scratch("example");
        let examples = dir.join("install/examples");
        fs::create_dir_all(examples.join("data")).unwrap();
        fs::write(examples.join("data/rows.rr"), "CSV.FileName = \"rows.csv\"\n").unwrap();
        fs::write(examples.join("data/rows.csv"), "x\n1\n").unwrap();
        let src = examples.join("data/rows.rr");
        let docs = dir.join("docs");
        // (RAPIDR_DOCUMENTS is this test's alone: no other test in this
        // crate reads it)
        std::env::set_var("RAPIDR_DOCUMENTS", &docs);
        // (a writable folder: the example itself — but an install's example
        // is copied even there)
        assert_eq!(example_to_open(&src, false).unwrap(), (src.clone(), ExampleOpen::InPlace));
        let copy = docs.join("RapidR/Examples/rows/rows.rr");
        assert_eq!(example_to_open(&src, true).unwrap(), (copy.clone(), ExampleOpen::Copied));
        fs::remove_dir_all(&docs).unwrap();
        fs::set_permissions(examples.join("data"), fs::Permissions::from_mode(0o555)).unwrap();
        if !can_write_in(&examples.join("data")) {
            assert_eq!(example_to_open(&src, false).unwrap(), (copy.clone(), ExampleOpen::Copied));
            assert!(docs.join("RapidR/Examples/rows/rows.csv").is_file());
            fs::write(&copy, "' mine\n").unwrap();
            assert_eq!(example_to_open(&src, false).unwrap(), (copy.clone(), ExampleOpen::Existing));
            assert_eq!(fs::read_to_string(&copy).unwrap(), "' mine\n", "the user's copy is never overwritten");
            assert_eq!(example_of_copy(&copy, &examples), Some(src.clone()));
            assert_eq!(example_of_copy(&src, &examples), None);
            assert_eq!(copy_example_again(&src).unwrap(), copy);
            assert_eq!(fs::read_to_string(&copy).unwrap(), "CSV.FileName = \"rows.csv\"\n");
        }
        fs::set_permissions(examples.join("data"), fs::Permissions::from_mode(0o755)).unwrap();
        std::env::remove_var("RAPIDR_DOCUMENTS");
        fs::remove_dir_all(&dir).unwrap();
    }
}
