//! Importing a RapidQ program, folder or project into a converted copy,
//! and upgrading one of RapidR's own files in place.
//!
//! The copy is the program **and every file it `$INCLUDE`s**, however deep,
//! found where the compiler finds them ([`rapidr_preprocessor::include_search`]:
//! the program's folder, the include folders, the including file's …):
//! those under the imported folder keep their place in it, those of an
//! include folder go to the copy's folder as they were in that include
//! folder (`GL\QGL.INC` → `GL/QGL.INC`: found there first, as RC.EXE finds
//! a program's own folder's files before its include folder's), any other
//! by its name. An `$INCLUDE` line that wouldn't find its file's copy from
//! the copy (`..\shared\x.inc`, `C:\rapidq\inc\x.inc`) is rewritten to the
//! copy's path. Includes RapidR supplies built in (RAPIDQ.INC, QMidi.inc …)
//! are never copied — a user's own file of that name neither — and an
//! include found nowhere is reported with what to do.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use crate::convert::{plan_program, plan_program_with, same_file, Change, Edit, FilePlan, Note, Options};
use crate::files::{Disk, Files};
use crate::includes::{lexical, relative, same, written_like, IncludeRef, Place};
use crate::verify;

/// Whether a copy was proved to compile as its original.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verification {
    /// The same bytecode, byte for byte.
    Identical,
    /// Different bytecode (the copy keeps the original's meaning in every
    /// name it changed, so this is a bug to report).
    Differs,
    /// The original doesn't compile in RapidR today: nothing to compare.
    OriginalFails(String),
    /// The original compiles, its copy doesn't (or it reads an original
    /// file instead of that file's copy).
    CopyFails(String),
}

/// One program of an import.
#[derive(Debug, Clone)]
pub struct ProgramReport {
    /// The main file, in the original.
    pub entry: PathBuf,
    /// The main file in the copy.
    pub copy: PathBuf,
    pub verification: Option<Verification>,
}

/// One file written into the copy.
#[derive(Debug, Clone)]
pub struct FileReport {
    pub source: PathBuf,
    pub dest: PathBuf,
    pub changes: Vec<Change>,
    /// A source file (converted) or another file (copied as it is).
    pub is_source: bool,
}

/// One `$INCLUDE` of the imported programs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeReport {
    /// The file it is in (the original) and its line.
    pub from: PathBuf,
    pub line: usize,
    /// The file's name as written.
    pub written: String,
    /// Where it was found.
    pub place: Place,
    /// The file it read (the original).
    pub file: Option<PathBuf>,
    /// That file's copy (`None`: not copied — RapidR supplies it, or it
    /// wasn't found).
    pub copy: Option<PathBuf>,
    /// The copy's line names it so now (its copy is elsewhere than the
    /// original's path leads).
    pub rewritten: Option<String>,
}

/// What an import did.
#[derive(Debug, Clone, Default)]
pub struct ImportReport {
    pub input: PathBuf,
    pub out_dir: PathBuf,
    pub programs: Vec<ProgramReport>,
    pub files: Vec<FileReport>,
    pub notes: Vec<Note>,
    /// Every `$INCLUDE` the programs follow, once each.
    pub includes: Vec<IncludeReport>,
    /// The include folders searched (the import's, then the user's).
    pub include_dirs: Vec<PathBuf>,
    /// Files of the names RapidR supplies built in (RapidQ's RAPIDQ.INC,
    /// QMidi.inc …: the user's own copies) the programs read or the folder
    /// has: never copied.
    pub builtin_left_out: Vec<PathBuf>,
    /// Programs that `$INCLUDE "RAPIDQ.INC"` (the line kept).
    pub rapidq_inc_includers: usize,
}

impl ImportReport {
    pub fn changes(&self) -> usize {
        self.files.iter().map(|f| f.changes.len()).sum()
    }

    pub fn count(&self, v: fn(&Verification) -> bool) -> usize {
        self.programs.iter().filter(|p| p.verification.as_ref().is_some_and(v)).count()
    }

    /// The includes found nowhere.
    pub fn missing(&self) -> impl Iterator<Item = &IncludeReport> {
        self.includes.iter().filter(|i| i.place == Place::Missing)
    }
}

/// What to do about an include found nowhere (`name`: as written).
pub fn missing_fix(name: &str) -> String {
    let n = name.trim().replace('\\', "/");
    let n = n.rsplit('/').next().unwrap_or(&n).to_string();
    format!("add the folder that holds {n} in RapidR Studio > Tools > Settings > Include folders (or `--include-dir <folder>`), or put {n} beside the program (in a browser: import the program's folder, or pick the program and its include files together), and import again")
}

/// A path as the file system compares it: real (links and `..` resolved),
/// and in one case where names aren't case-sensitive (macOS, Windows), so
/// `RapidQ2.inc` and `RAPIDQ2.INC` are one file.
fn fold(p: &Path) -> PathBuf {
    if cfg!(any(target_os = "macos", target_os = "windows")) {
        PathBuf::from(p.to_string_lossy().to_lowercase())
    } else {
        p.to_path_buf()
    }
}

fn real(fs: &dyn Files, p: &Path) -> PathBuf {
    fs.canonical(p).unwrap_or_else(|| p.to_path_buf())
}

fn key(fs: &dyn Files, p: &Path) -> PathBuf {
    fold(&real(fs, p))
}

fn ext(p: &Path) -> String {
    p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default()
}

fn is_builtin_file(p: &Path) -> bool {
    rapidr_preprocessor::is_builtin_include(&p.to_string_lossy())
}

/// Files `rapidr import-rapidq` never copies: RapidQ IDE templates
/// (RapidQ's own material).
fn skipped(p: &Path) -> bool {
    ext(p) == "tpl"
}

/// Programs first (`.bas`, `.rqw`, `.rr`), then RapidQ's other program
/// files (`.rqb`, `.rq`), then the rest (`.inc` …), each by path.
fn entry_rank(p: &Path) -> u8 {
    match ext(p).as_str() {
        "bas" | "rqw" | "rr" => 0,
        "rqb" | "rq" => 1,
        _ => 2,
    }
}

fn walk(fs: &dyn Files, dir: &Path, skip: &Path, out: &mut Vec<PathBuf>) {
    for p in fs.entries(dir) {
        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if name.starts_with('.') || same_file(fs, &p, skip) {
            continue;
        }
        if fs.is_dir(&p) {
            walk(fs, &p, skip, out);
        } else if fs.is_file(&p) {
            out.push(p);
        }
    }
}

/// The include folders of an import: the options', then the user's
/// ([`rapidr_preprocessor::configured_include_dirs`]), each once.
fn include_dirs(options: &Options) -> Vec<PathBuf> {
    let mut dirs = options.include_dirs.clone();
    for d in env_include_dirs() {
        if !dirs.contains(&d) {
            dirs.push(d);
        }
    }
    dirs
}

/// What `$INCLUDE "written"` reads in the copy (a file of `includer_dir` of
/// the program in `main_dir`): the first place of the compiler's search
/// that is one of the copy's files (`planned`: by [`lexical`] path, lower
/// case) or a file there now.
fn copy_resolves(fs: &dyn Files, written: &str, includer_dir: &Path, main_dir: &Path, dirs: &[PathBuf], planned: &HashMap<String, PathBuf>) -> Option<PathBuf> {
    for (root, rel) in rapidr_preprocessor::include_search(written, includer_dir, main_dir, dirs) {
        if let Some(d) = planned.get(&lexical(&root.join(&rel)).to_lowercase()) {
            return Some(d.clone());
        }
        if let Some(found) = fs.find(&root, &rel) {
            return Some(found);
        }
    }
    None
}

/// Imports `input` — a program (`.bas`, `.rqw`, `.rqb`, `.rq`, `.inc`,
/// `.rr`), a folder, or a project (`.rrproj`: its folder) — into `out_dir`
/// with RapidR's names, and checks each program against its original
/// (when `verify`). Never writes over a file it reads.
pub fn import(input: &Path, out_dir: &Path, options: &Options, verify: bool) -> Result<ImportReport, String> {
    import_with(&Disk, input, out_dir, options, verify)
}

/// [`import`] in `fs` (the disk, or files in memory: RapidR Studio on the
/// web).
pub fn import_with(fs: &dyn Files, input: &Path, out_dir: &Path, options: &Options, verify: bool) -> Result<ImportReport, String> {
    let input = fs.canonical(input).ok_or_else(|| format!("{}: no such file or folder", input.display()))?;
    let is_project = ext(&input) == "rrproj";
    let input_is_dir = fs.is_dir(&input);
    let root = if input_is_dir { input.clone() } else { input.parent().map(Path::to_path_buf).unwrap_or_default() };
    fs.make_dir(out_dir)?;
    let out_dir = fs.canonical(out_dir).unwrap_or_else(|| out_dir.to_path_buf());
    if out_dir == root || (input_is_dir && root.starts_with(&out_dir)) {
        return Err(format!("{}: the copy must go to another folder than the original's", out_dir.display()));
    }
    let dirs = include_dirs(options);
    // (each include folder as the files are named: real)
    let real_dirs: Vec<PathBuf> = dirs.iter().filter_map(|d| fs.canonical(d)).collect();

    // The programs and the other files.
    let mut all: Vec<PathBuf> = Vec::new();
    let mut entries: Vec<PathBuf> = Vec::new();
    if input_is_dir || is_project {
        walk(fs, &root, &out_dir, &mut all);
        // (an include RapidR supplies — RapidQ's RAPIDQ.INC — is never a
        // program of its own)
        entries = all.iter().filter(|p| rapidr_preprocessor::is_source_path(p) && !is_builtin_file(p)).cloned().collect();
        if is_project {
            let project = fs.read(&input).ok().and_then(|b| rapidr_project::Project::from_toml(&String::from_utf8_lossy(&b)).ok());
            if let Some(project) = project {
                let main = root.join(&project.main);
                if let Some(i) = entries.iter().position(|e| same_file(fs, e, &main)) {
                    let m = entries.remove(i);
                    entries.insert(0, m);
                }
            }
        }
        entries.sort_by_key(|p| entry_rank(p));
    } else {
        entries.push(input.clone());
        all.push(input.clone());
    }

    let mut report = ImportReport { input: input.clone(), out_dir: out_dir.clone(), include_dirs: dirs.clone(), ..Default::default() };
    // Every source file reached: its plan (edits merged across programs).
    let mut files: BTreeMap<PathBuf, FilePlan> = BTreeMap::new();
    let mut builtin: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut extra: Vec<PathBuf> = Vec::new();
    let mut reached: Vec<PathBuf> = Vec::new();
    // (each program's main file, and the built-in includes its proof reads
    // as RapidR's: the copy can't find the user's own)
    let mut program_entries: Vec<(PathBuf, Vec<String>)> = Vec::new();
    let mut program_includes: Vec<(PathBuf, Vec<IncludeRef>)> = Vec::new();
    for entry in &entries {
        // (a file another program includes is converted with it)
        if entry_rank(entry) == 2 && reached.iter().any(|r| same_file(fs, r, entry)) {
            continue;
        }
        let plan = match plan_program_with(fs, entry, options) {
            Ok(p) => p,
            Err(e) => {
                report.notes.push(Note { path: Some(entry.clone()), line: 0, message: format!("not read: {e}") });
                continue;
            }
        };
        if entry_rank(entry) < 2 {
            // (RAPIDQ.INC as RapidR's but where the program uses more of
            // its own: then the copy needs that one found, and the proof
            // says whether it is)
            let names: Vec<String> = plan
                .files
                .iter()
                .filter(|f| f.builtin && !(plan.own_rapidq_inc.is_some() && crate::convert::is_rapidq_inc_name(&f.path.to_string_lossy())))
                .filter_map(|f| f.path.file_name().map(|n| n.to_string_lossy().into_owned()))
                .collect();
            program_entries.push((entry.clone(), names));
        }
        if plan.files.iter().skip(1).any(|f| f.builtin && crate::convert::is_rapidq_inc_name(&f.path.to_string_lossy())) || plan.files.iter().any(|f| has_rapidq_inc_line(&f.text)) {
            report.rapidq_inc_includers += 1;
        }
        report.notes.extend(plan.notes.iter().cloned());
        if let Some(used) = &plan.own_rapidq_inc {
            let inc = plan.files.iter().find(|f| f.builtin && crate::convert::is_rapidq_inc_name(&f.path.to_string_lossy())).map(|f| real(fs, &f.path)).unwrap_or_default();
            let folder = inc.parent().map(|p| p.display().to_string()).unwrap_or_default();
            report.notes.push(Note {
                path: Some(entry.clone()),
                line: 0,
                message: format!(
                    "uses {used} of your RAPIDQ.INC ({}), which RapidR's built-in RAPIDQ.INC doesn't have; RAPIDQ.INC isn't copied (RapidR supplies it), so the copy needs your file found: keep its folder ({folder}) in the include folders (RapidR Studio: Tools > Settings > Include folders; rapidr: --include-dir)",
                    inc.display()
                ),
            });
        }
        for mut f in plan.files {
            let key = key(fs, &f.path);
            f.path = real(fs, &f.path);
            reached.push(key.clone());
            if f.builtin {
                if !builtin.iter().any(|(k, _)| *k == key) {
                    builtin.push((key, f.path.clone()));
                }
                continue;
            }
            match files.get_mut(&key) {
                None => {
                    files.insert(key, f);
                }
                Some(have) => merge(have, f, &mut report.notes),
            }
        }
        for r in plan.resources {
            extra.push(real(fs, &r));
        }
        program_includes.push((real(fs, entry), plan.includes));
    }

    // Where each file goes: under the imported folder as it was; from an
    // include folder as it was there (the copy's folder is searched first);
    // from elsewhere by its name; a place taken: under `include/`.
    let mut dests: BTreeMap<PathBuf, PathBuf> = BTreeMap::new();
    let mut taken: Vec<PathBuf> = Vec::new();
    let natural = |p: &Path| -> PathBuf {
        if let Ok(rel) = p.strip_prefix(&root) {
            return rel.to_path_buf();
        }
        if let Some(rel) = real_dirs.iter().find_map(|d| p.strip_prefix(d).ok()) {
            return rel.to_path_buf();
        }
        p.file_name().map(PathBuf::from).unwrap_or_default()
    };
    let mut order: Vec<(&PathBuf, &FilePlan)> = files.iter().collect();
    // (the imported folder's own files first: they keep their places)
    order.sort_by_key(|(_, f)| !f.path.starts_with(&root));
    for (k, plan) in order {
        let rel = natural(&plan.path);
        let mut dest = out_dir.join(&rel);
        if taken.contains(&fold(&dest)) {
            dest = out_dir.join("include").join(&rel);
        }
        if taken.contains(&fold(&dest)) {
            report.notes.push(Note { path: Some(plan.path.clone()), line: 0, message: format!("another file is copied to {} already: not copied", dest.display()) });
            continue;
        }
        taken.push(fold(&dest));
        dests.insert(k.clone(), dest);
    }

    // Everything else of a folder (and a program's resources), as it is —
    // but RapidQ's IDE templates and RapidR's built-in includes.
    let mut others: Vec<PathBuf> = all.iter().map(|p| real(fs, p)).filter(|p| !files.contains_key(&fold(p))).collect();
    others.extend(extra.into_iter().filter(|p| !files.contains_key(&fold(p))));
    others.sort();
    others.dedup();
    let mut other_dests: Vec<(PathBuf, PathBuf)> = Vec::new();
    for p in others {
        if skipped(&p) {
            report.notes.push(Note { path: Some(p.clone()), line: 0, message: "a RapidQ IDE template (.tpl): not copied".into() });
            continue;
        }
        if is_builtin_file(&p) {
            let k = fold(&p);
            if !builtin.iter().any(|(b, _)| *b == k) {
                builtin.push((k, p.clone()));
            }
            continue;
        }
        let dest = match p.strip_prefix(&root) {
            Ok(rel) => out_dir.join(rel),
            Err(_) => out_dir.join(p.file_name().map(PathBuf::from).unwrap_or_default()),
        };
        if taken.contains(&fold(&dest)) {
            continue;
        }
        taken.push(fold(&dest));
        other_dests.push((p, dest));
    }
    report.builtin_left_out = builtin.into_iter().map(|(_, p)| p).collect();

    // Each program's includes in the copy: a line that wouldn't find its
    // file's copy names the copy (from the program's folder, searched first).
    let planned: HashMap<String, PathBuf> = dests.values().chain(other_dests.iter().map(|(_, d)| d)).map(|d| (lexical(d).to_lowercase(), d.clone())).collect();
    let mut seen: Vec<(PathBuf, usize, String)> = Vec::new();
    for (entry, refs) in &program_includes {
        let Some(main_dest) = dests.get(&fold(entry)) else { continue };
        let main_dir = main_dest.parent().map(Path::to_path_buf).unwrap_or_default();
        for r in refs {
            let from_key = key(fs, &r.from);
            let target = r.file.as_ref().map(|t| key(fs, t));
            let copy = target.as_ref().and_then(|t| dests.get(t)).cloned();
            let mut rewritten = None;
            if let (Some(copy), Some(from_dest)) = (&copy, dests.get(&from_key)) {
                let includer_dir = from_dest.parent().map(Path::to_path_buf).unwrap_or_default();
                let finds = |written: &str| copy_resolves(fs, written, &includer_dir, &main_dir, &dirs, &planned).is_some_and(|f| same(&f, copy));
                if !finds(&r.written) {
                    let new = written_like(&r.written, &relative(&main_dir, copy));
                    let plan = files.get_mut(&from_key).expect("a copied file has a plan");
                    match plan.edits.iter().find(|e| e.start == r.span.start) {
                        Some(e) if e.text != new => report.notes.push(Note {
                            path: Some(plan.path.clone()),
                            line: r.line,
                            message: format!("`{}` is read by programs in different folders: its copy names `{}`, which one of them doesn't find", r.written, e.text),
                        }),
                        Some(_) => {}
                        None => {
                            if !finds(&new) {
                                report.notes.push(Note { path: Some(plan.path.clone()), line: r.line, message: format!("`{}` names `{new}` in the copy, which may find another file first", r.written) });
                            }
                            let line_start = plan.text[..r.span.start].rfind('\n').map_or(0, |i| i + 1);
                            let column = plan.text[line_start..r.span.start].chars().count() + 1;
                            plan.edits.push(Edit { start: r.span.start, end: r.span.end, text: new.clone() });
                            plan.edits.sort_by_key(|e| e.start);
                            plan.changes.push(Change { line: r.line, column, from: r.written.clone(), to: new.clone() });
                            plan.changes.sort_by_key(|c| (c.line, c.column));
                        }
                    }
                    rewritten = Some(new);
                }
            }
            let k = (from_key.clone(), r.line, r.written.clone());
            if seen.contains(&k) {
                continue;
            }
            seen.push(k);
            report.includes.push(IncludeReport { from: real(fs, &r.from), line: r.line, written: r.written.clone(), place: r.place.clone(), file: r.file.as_ref().map(|f| real(fs, f)), copy, rewritten });
        }
    }

    // The copy, written.
    let sources: Vec<PathBuf> = files.values().map(|f| f.path.clone()).collect();
    let guard = |dest: &Path| -> Result<(), String> {
        if sources.iter().chain(all.iter()).any(|s| same_file(fs, s, dest)) {
            return Err(format!("{}: would write over an original", dest.display()));
        }
        Ok(())
    };
    for (k, dest) in &dests {
        let plan = &files[k];
        guard(dest)?;
        fs.write(dest, &plan.converted_bytes())?;
        report.files.push(FileReport { source: plan.path.clone(), dest: dest.clone(), changes: plan.changes.clone(), is_source: true });
    }
    for (p, dest) in other_dests {
        guard(&dest)?;
        let bytes = fs.read(&p)?;
        fs.write(&dest, &bytes)?;
        report.files.push(FileReport { source: p, dest, changes: Vec::new(), is_source: false });
    }
    // (the program's own files first, as before: by where they come from)
    report.files.sort_by(|a, b| (!a.is_source, &a.source).cmp(&(!b.is_source, &b.source)));

    // Notes about a line another program converted after all (an `$IFDEF`
    // branch one program compiles, another doesn't) are dropped.
    let converted: Vec<(PathBuf, usize)> = report.files.iter().flat_map(|f| f.changes.iter().map(move |c| (fold(&f.source), c.line))).collect();
    report.notes.retain(|n| !(n.message.contains("$IFDEF branch") && n.path.as_ref().is_some_and(|p| converted.contains(&(key(fs, p), n.line)))));
    let mut seen_notes: Vec<(Option<PathBuf>, usize, String)> = Vec::new();
    report.notes.retain(|n| {
        let k = (n.path.as_ref().map(|p| key(fs, p)), n.line, n.message.clone());
        if seen_notes.contains(&k) {
            false
        } else {
            seen_notes.push(k);
            true
        }
    });

    // The proof, program by program: each compiled as the user compiles it
    // (its folder, the include folders), the copy reading only copies.
    for (entry, builtins) in program_entries {
        let k = key(fs, &entry);
        let copy = dests.get(&k).cloned().unwrap_or_default();
        let verification = verify.then(|| match verify::bytecode_with(fs, &entry, &dirs, None, &builtins) {
            Err(e) => Verification::OriginalFails(first_line(&e)),
            Ok((a, _)) => match verify::bytecode_with(fs, &copy, &dirs, None, &builtins) {
                Err(e) => Verification::CopyFails(first_line(&e)),
                Ok((_, read)) if read.iter().any(|f| dests.contains_key(&key(fs, f))) => {
                    let f = read.iter().find(|f| dests.contains_key(&key(fs, f))).cloned().unwrap_or_default();
                    Verification::CopyFails(format!("it reads the original {} instead of its copy", f.display()))
                }
                Ok((b, _)) if a == b => Verification::Identical,
                Ok(_) => Verification::Differs,
            },
        });
        report.programs.push(ProgramReport { entry, copy, verification });
    }
    Ok(report)
}

fn first_line(e: &str) -> String {
    e.lines().next().unwrap_or("").to_string()
}

/// The user's include folders: RapidR Studio's Settings > Include folders,
/// `--include-dir`, `RAPIDR_INCLUDE_PATH`'s — what `rapidr build-bc`
/// searches too ([`rapidr_preprocessor::configured_include_dirs`]).
pub fn env_include_dirs() -> Vec<PathBuf> {
    rapidr_preprocessor::configured_include_dirs()
}

fn has_rapidq_inc_line(text: &str) -> bool {
    text.lines().any(|l| {
        let t = l.trim_start();
        t.get(..8).is_some_and(|d| d.eq_ignore_ascii_case("$INCLUDE")) && crate::convert::is_rapidq_inc_name(t[8..].trim().trim_matches(['"', '<', '>', '\'', '\r']).trim())
    })
}

/// Edits of the same file planned by two programs: the union (the same
/// name is converted the same way by every program that reads the file).
fn merge(have: &mut FilePlan, other: FilePlan, notes: &mut Vec<Note>) {
    for (e, c) in other.edits.into_iter().zip(other.changes) {
        match have.edits.iter().find(|h| h.start == e.start) {
            Some(h) if h.text != e.text || h.end != e.end => notes.push(Note {
                path: Some(have.path.clone()),
                line: c.line,
                message: format!("two programs read `{}` differently ({} / {}): left as {}", c.from, h.text, e.text, h.text),
            }),
            Some(_) => {}
            None if have.edits.iter().any(|h| h.start < e.end && e.start < h.end) => {}
            None => {
                have.edits.push(e);
                have.changes.push(c);
            }
        }
    }
    have.edits.sort_by_key(|e: &Edit| e.start);
    have.changes.sort_by_key(|c| (c.line, c.column));
}

/// One of RapidR's own files, upgraded to RapidR's names in place.
#[derive(Debug, Clone)]
pub struct Upgrade {
    pub path: PathBuf,
    pub original: String,
    pub converted: String,
    pub changes: Vec<Change>,
    pub notes: Vec<Note>,
    /// The file with and without the changes compiled to the same bytecode
    /// (None: the program doesn't compile today, so it wasn't checked).
    pub verified: Option<bool>,
    /// Bytes to write (the file's own encoding).
    pub bytes: Vec<u8>,
}

/// Plans upgrading `path` (as a program: its includes are read, not
/// changed) and checks it compiles as before.
pub fn upgrade_file(path: &Path, options: &Options) -> Result<Upgrade, String> {
    let plan = plan_program(path, options)?;
    let file = plan.files.into_iter().find(|f| same_file(&Disk, &f.path, path)).ok_or_else(|| format!("{}: not read", path.display()))?;
    let converted = file.converted();
    let dirs = include_dirs(options);
    let verified = match verify::bytecode(path, &dirs, None) {
        Err(_) => None,
        Ok(a) => Some(verify::bytecode(path, &dirs, Some(&converted)).is_ok_and(|b| a == b)),
    };
    let notes = plan.notes.into_iter().filter(|n| n.path.as_deref().is_some_and(|p| same_file(&Disk, p, path))).collect();
    Ok(Upgrade { path: path.to_path_buf(), bytes: file.converted_bytes(), original: file.text, converted, changes: file.changes, notes, verified })
}
