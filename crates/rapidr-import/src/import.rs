//! Importing a RapidQ program, folder or project into a converted copy,
//! and upgrading one of RapidR's own files in place.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::convert::{plan_program, same_file, Change, Edit, FilePlan, Note, Options};
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
    /// The original compiles, its copy doesn't.
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

/// What an import did.
#[derive(Debug, Clone, Default)]
pub struct ImportReport {
    pub input: PathBuf,
    pub out_dir: PathBuf,
    pub programs: Vec<ProgramReport>,
    pub files: Vec<FileReport>,
    pub notes: Vec<Note>,
    /// RapidQ's RAPIDQ.INC files read and left out of the copy (RapidR
    /// supplies their constants).
    pub rapidq_inc_left_out: Vec<PathBuf>,
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

fn real(p: &Path) -> PathBuf {
    fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

fn key(p: &Path) -> PathBuf {
    fold(&real(p))
}

fn ext(p: &Path) -> String {
    p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default()
}

/// Files `rapidr import-rapidq` never copies: RapidQ IDE templates
/// (RapidQ's own material).
fn skipped(p: &Path) -> bool {
    ext(p) == "tpl"
}

/// Main programs first (`.bas`, `.rqw`, `.rr`), then libraries (`.rqb`,
/// `.rq`), then the rest (`.inc` …), each by path.
fn entry_rank(p: &Path) -> u8 {
    match ext(p).as_str() {
        "bas" | "rqw" | "rr" => 0,
        "rqb" | "rq" => 1,
        _ => 2,
    }
}

fn walk(dir: &Path, skip: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for p in entries {
        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if name.starts_with('.') || same_file(&p, skip) {
            continue;
        }
        if p.is_dir() {
            walk(&p, skip, out);
        } else if p.is_file() {
            out.push(p);
        }
    }
}

/// Imports `input` — a program (`.bas`, `.rqw`, `.rqb`, `.rq`, `.inc`,
/// `.rr`), a folder, or a project (`.rrproj`: its folder) — into `out_dir`
/// with RapidR's names, and checks each program against its original
/// (when `verify`). Never writes over a file it reads.
pub fn import(input: &Path, out_dir: &Path, options: &Options, verify: bool) -> Result<ImportReport, String> {
    let input = fs::canonicalize(input).map_err(|e| format!("{}: {e}", input.display()))?;
    let is_project = ext(&input) == "rrproj";
    let root = if input.is_dir() { input.clone() } else { input.parent().map(Path::to_path_buf).unwrap_or_default() };
    fs::create_dir_all(out_dir).map_err(|e| format!("{}: {e}", out_dir.display()))?;
    let out_dir = fs::canonicalize(out_dir).map_err(|e| format!("{}: {e}", out_dir.display()))?;
    if out_dir == root || (input.is_dir() && root.starts_with(&out_dir)) {
        return Err(format!("{}: the copy must go to another folder than the original's", out_dir.display()));
    }

    // The programs and the other files.
    let mut all: Vec<PathBuf> = Vec::new();
    let mut entries: Vec<PathBuf> = Vec::new();
    if input.is_dir() || is_project {
        walk(&root, &out_dir, &mut all);
        // (RapidQ's RAPIDQ.INC is never a program of its own: RapidR supplies it)
        entries = all.iter().filter(|p| rapidr_preprocessor::is_source_path(p) && !crate::convert::is_rapidq_inc_name(&p.to_string_lossy())).cloned().collect();
        if is_project {
            if let Ok(opened) = rapidr_project::open(&input) {
                let main = root.join(&opened.project().main);
                if let Some(i) = entries.iter().position(|e| same_file(e, &main)) {
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

    let mut report = ImportReport { input: input.clone(), out_dir: out_dir.clone(), ..Default::default() };
    // Every source file reached: its plan (edits merged across programs).
    let mut files: BTreeMap<PathBuf, FilePlan> = BTreeMap::new();
    let mut carried_inc: Vec<PathBuf> = Vec::new();
    let mut left_out_shown: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut extra: Vec<PathBuf> = Vec::new();
    let mut reached: Vec<PathBuf> = Vec::new();
    // (and whether its RAPIDQ.INC file is left out of the copy)
    let mut program_entries: Vec<(PathBuf, bool)> = Vec::new();
    for entry in &entries {
        // (a file another program includes is converted with it)
        if entry_rank(entry) == 2 && reached.iter().any(|r| same_file(r, entry)) {
            continue;
        }
        let plan = match plan_program(entry, options) {
            Ok(p) => p,
            Err(e) => {
                report.notes.push(Note { path: Some(entry.clone()), line: 0, message: format!("not read: {e}") });
                continue;
            }
        };
        if entry_rank(entry) < 2 {
            let left_out = plan.carry_rapidq_inc.is_none() && plan.files.iter().any(|f| f.is_rapidq_inc);
            program_entries.push((entry.clone(), left_out));
        }
        if plan.files.iter().skip(1).any(|f| f.is_rapidq_inc) || plan.files.iter().any(|f| has_rapidq_inc_line(&f.text)) {
            report.rapidq_inc_includers += 1;
        }
        report.notes.extend(plan.notes.iter().cloned());
        if let Some(why) = &plan.carry_rapidq_inc {
            report.notes.push(Note { path: Some(entry.clone()), line: 0, message: why.clone() });
        }
        for mut f in plan.files {
            let key = key(&f.path);
            f.path = real(&f.path);
            if f.is_rapidq_inc {
                if plan.carry_rapidq_inc.is_some() {
                    carried_inc.push(key.clone());
                } else if !report.rapidq_inc_left_out.contains(&key) {
                    report.rapidq_inc_left_out.push(key.clone());
                    left_out_shown.push((key.clone(), f.path.clone()));
                }
            }
            reached.push(key.clone());
            match files.get_mut(&key) {
                None => {
                    files.insert(key, f);
                }
                Some(have) => merge(have, f, &mut report.notes),
            }
        }
        for r in plan.resources {
            extra.push(real(&r));
        }
    }
    report.rapidq_inc_left_out.retain(|p| !carried_inc.contains(p));

    // Where each file goes: under the root as it was; from elsewhere
    // (RapidQ's include folder) into the copy's top folder.
    let dest_of = |p: &Path, notes: &mut Vec<Note>| -> PathBuf {
        match p.strip_prefix(&root) {
            Ok(rel) => out_dir.join(rel),
            Err(_) => {
                let name = p.file_name().map(PathBuf::from).unwrap_or_default();
                notes.push(Note { path: Some(p.to_path_buf()), line: 0, message: format!("outside the program's folder: copied to the copy's top folder as {}", name.display()) });
                out_dir.join(name)
            }
        }
    };
    let sources: Vec<PathBuf> = files.values().map(|f| f.path.clone()).collect();
    let guard = |dest: &Path| -> Result<(), String> {
        if sources.iter().chain(all.iter()).any(|s| same_file(s, dest)) {
            return Err(format!("{}: would write over an original", dest.display()));
        }
        Ok(())
    };
    let mut written: Vec<PathBuf> = Vec::new();
    for (k, plan) in &files {
        let path = &plan.path;
        if plan.is_rapidq_inc && !carried_inc.contains(k) {
            continue;
        }
        let dest = dest_of(path, &mut report.notes);
        if written.contains(&fold(&dest)) {
            report.notes.push(Note { path: Some(path.clone()), line: 0, message: format!("another file is copied to {} already: not copied", dest.display()) });
            continue;
        }
        guard(&dest)?;
        write(&dest, &plan.converted_bytes())?;
        written.push(fold(&dest));
        report.files.push(FileReport { source: path.clone(), dest, changes: plan.changes.clone(), is_source: true });
    }
    // Everything else of a folder (and a program's resources), as it is.
    let mut others: Vec<PathBuf> = all.iter().map(|p| real(p)).filter(|p| !files.contains_key(&fold(p))).collect();
    others.extend(extra.into_iter().filter(|p| !files.contains_key(&fold(p))));
    others.sort();
    others.dedup();
    for p in others {
        if skipped(&p) {
            report.notes.push(Note { path: Some(p.clone()), line: 0, message: "a RapidQ IDE template (.tpl): not copied".into() });
            continue;
        }
        if report.rapidq_inc_left_out.contains(&fold(&p)) {
            continue;
        }
        let dest = dest_of(&p, &mut report.notes);
        if written.contains(&fold(&dest)) {
            continue;
        }
        guard(&dest)?;
        let bytes = fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        write(&dest, &bytes)?;
        written.push(fold(&dest));
        report.files.push(FileReport { source: p, dest, changes: Vec::new(), is_source: false });
    }

    report.rapidq_inc_left_out = left_out_shown.into_iter().filter(|(k, _)| report.rapidq_inc_left_out.contains(k)).map(|(_, p)| p).collect();

    // Notes about a line another program converted after all (an `$IFDEF`
    // branch one program compiles, another doesn't) are dropped.
    let converted: Vec<(PathBuf, usize)> = report.files.iter().flat_map(|f| f.changes.iter().map(move |c| (fold(&f.source), c.line))).collect();
    report.notes.retain(|n| !(n.message.contains("$IFDEF branch") && n.path.as_ref().is_some_and(|p| converted.contains(&(key(p), n.line)))));
    let mut seen: Vec<(Option<PathBuf>, usize, String)> = Vec::new();
    report.notes.retain(|n| {
        let k = (n.path.as_ref().map(|p| key(p)), n.line, n.message.clone());
        if seen.contains(&k) {
            false
        } else {
            seen.push(k);
            true
        }
    });

    // The proof, program by program.
    let mut copy_dirs = vec![out_dir.clone()];
    copy_dirs.extend(options.include_dirs.iter().cloned());
    copy_dirs.extend(env_include_dirs());
    let mut orig_dirs = options.include_dirs.clone();
    orig_dirs.extend(env_include_dirs());
    for (entry, builtin_inc) in program_entries {
        let k = key(&entry);
        let copy = report.files.iter().find(|f| fold(&f.source) == k).map(|f| f.dest.clone()).unwrap_or_default();
        let verification = verify.then(|| match verify::bytecode(&entry, &orig_dirs, None, builtin_inc) {
            Err(e) => Verification::OriginalFails(first_line(&e)),
            Ok(a) => match verify::bytecode(&copy, &copy_dirs, None, builtin_inc) {
                Err(e) => Verification::CopyFails(first_line(&e)),
                Ok(b) if a == b => Verification::Identical,
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

/// `RAPIDR_INCLUDE_PATH`'s folders (what `rapidr build-bc` searches too).
pub fn env_include_dirs() -> Vec<PathBuf> {
    std::env::var_os("RAPIDR_INCLUDE_PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default()
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

fn write(dest: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    fs::write(dest, bytes).map_err(|e| format!("{}: {e}", dest.display()))
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
    let file = plan.files.into_iter().find(|f| same_file(&f.path, path)).ok_or_else(|| format!("{}: not read", path.display()))?;
    let converted = file.converted();
    let mut dirs = options.include_dirs.clone();
    dirs.extend(env_include_dirs());
    let verified = match verify::bytecode(path, &dirs, None, false) {
        Err(_) => None,
        Ok(a) => Some(verify::bytecode(path, &dirs, Some(&converted), false).is_ok_and(|b| a == b)),
    };
    let notes = plan.notes.into_iter().filter(|n| n.path.as_deref().is_some_and(|p| same_file(p, path))).collect();
    Ok(Upgrade { path: path.to_path_buf(), bytes: file.converted_bytes(), original: file.text, converted, changes: file.changes, notes, verified })
}
