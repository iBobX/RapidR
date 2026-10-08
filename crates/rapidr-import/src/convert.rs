//! One program's type names, planned as edits of its files.

use std::path::{Path, PathBuf};

use rapidr_ast::{Program, Statement};
use rapidr_lang::Component;
use rapidr_lexer::{Lexer, TokenType};
use rapidr_parser::{parse_file_for_tools, ToolsParse};
use rapidr_preprocessor::{FileId, LineKind, PreprocessOptions, SourceEncoding};

/// How a program is read.
#[derive(Debug, Clone)]
pub struct Options {
    /// Where `$INCLUDE` looks after the including file's folder (RapidQ's
    /// `include\` folder). `RAPIDR_INCLUDE_PATH` is searched too.
    pub include_dirs: Vec<PathBuf>,
    /// Also write RapidR's names that are spelled otherwise
    /// (`RBUTTON`, `rbutton` → `RButton`).
    pub normalize_case: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options { include_dirs: Vec::new(), normalize_case: true }
    }
}

/// A replacement of bytes `start..end` of a file's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// One change, as the report lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    /// 1-based line and column (in characters) of the file.
    pub line: usize,
    pub column: usize,
    pub from: String,
    pub to: String,
}

/// Something the importer left as it was, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub path: Option<PathBuf>,
    /// 1-based; 0 when it isn't about a line.
    pub line: usize,
    pub message: String,
}

/// A file of the program and what changes in it.
#[derive(Debug, Clone)]
pub struct FilePlan {
    pub path: PathBuf,
    /// The text as RapidR reads it (decoded).
    pub text: String,
    pub encoding: SourceEncoding,
    /// Sorted, not overlapping.
    pub edits: Vec<Edit>,
    pub changes: Vec<Change>,
    /// RapidQ's `RAPIDQ.INC` (found as a file).
    pub is_rapidq_inc: bool,
    /// How many of its type names name a component RapidQ has by RapidQ's
    /// name (`QBUTTON`) and by RapidR's (`RButton`): its [`NameStyle`].
    pub rapidq_names: usize,
    pub rapidr_names: usize,
}

/// How a file writes the names of the components RapidQ has too: what
/// RapidR Studio's designer and completion follow, so a file never mixes
/// the two (docs/ide-plan.md, R-NAMES phase 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameStyle {
    /// RapidQ's names only (`QBUTTON`): a RapidQ program.
    RapidQ,
    /// RapidR's names only (`RButton`), or none yet: RapidR's default.
    RapidR,
    /// Both.
    Mixed,
}

impl FilePlan {
    /// How the file writes component names (before the conversion).
    pub fn style(&self) -> NameStyle {
        match (self.rapidq_names, self.rapidr_names) {
            (0, _) => NameStyle::RapidR,
            (_, 0) => NameStyle::RapidQ,
            _ => NameStyle::Mixed,
        }
    }

    /// The file's text with the edits made.
    pub fn converted(&self) -> String {
        apply(&self.text, &self.edits)
    }

    /// The converted text as bytes, in the file's own encoding.
    pub fn converted_bytes(&self) -> Vec<u8> {
        rapidr_preprocessor::encode_source(&self.converted(), self.encoding)
    }
}

/// What converting one program means: its files (the main file first) and
/// their edits.
#[derive(Debug, Clone)]
pub struct ProgramPlan {
    pub entry: PathBuf,
    pub files: Vec<FilePlan>,
    pub notes: Vec<Note>,
    /// The files its `$RESOURCE`s (and `$OPTION ICON`) name, found.
    pub resources: Vec<PathBuf>,
    /// Whether RapidQ's RAPIDQ.INC file (when the program reads one) must
    /// go into the copy, and why ([`crate::rapidq_inc`]).
    pub carry_rapidq_inc: Option<String>,
    /// The parser's and preprocessor's errors (the program may not compile).
    pub errors: usize,
}

/// `text` with `edits` (sorted, not overlapping) made.
pub fn apply(text: &str, edits: &[Edit]) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    let mut at = 0;
    for e in edits {
        out.push_str(&text[at..e.start]);
        out.push_str(&e.text);
        at = e.end;
    }
    out.push_str(&text[at..]);
    out
}

/// Plans the conversion of the program whose main file is `entry`.
pub fn plan_program(entry: &Path, options: &Options) -> Result<ProgramPlan, String> {
    let pp = PreprocessOptions { include_dirs: options.include_dirs.clone(), ..Default::default() };
    let tp = parse_file_for_tools(entry, pp).map_err(|d| d.to_string())?;
    Ok(plan_parsed(entry, &tp, options))
}

/// The program's own TYPEs (upper case).
fn own_types(program: &Program) -> Vec<String> {
    program.statements.iter().filter_map(|s| if let Statement::Type(t) = s { Some(t.name.to_ascii_uppercase()) } else { None }).collect()
}

/// What a type name means (the compilers' name for it: `RBUTTON`) and
/// RapidR's spelling of it (`RButton`), when the name should be written so:
/// the compiler reads it as one of its components (not the program's own
/// TYPE), and the spelling means the same. RapidQ's empty base object
/// QOBJECT (`TYPE T EXTENDS QOBJECT`: a TYPE with methods) is RapidR's
/// `RObject`.
pub fn target(written: &str, own: &[String], normalize_case: bool) -> Option<(String, String)> {
    if own.iter().any(|t| t.eq_ignore_ascii_case(written)) {
        return None;
    }
    let (resolved, to) = if written.eq_ignore_ascii_case("QOBJECT") || written.eq_ignore_ascii_case("ROBJECT") {
        ("ROBJECT".to_string(), "RObject".to_string())
    } else {
        let resolved = rapidr_ast::component_type_reference(written, own);
        if !rapidr_ast::is_component_type_name(&resolved) {
            return None;
        }
        let c: &Component = rapidr_lang::component(&resolved)?;
        // (the new name is read as the same component)
        if !rapidr_ast::component_type_reference(&c.spelling(), own).eq_ignore_ascii_case(&resolved) {
            return None;
        }
        (resolved.to_ascii_uppercase(), c.spelling())
    };
    if to == written || (!normalize_case && to.eq_ignore_ascii_case(written)) {
        return None;
    }
    Some((resolved, to))
}

fn is_identifier(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_') && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Whether a file is one of RapidR's built-in libraries (no file on disk).
fn is_builtin(path: &Path) -> bool {
    path.to_string_lossy().starts_with("<RapidR>")
}

/// Whether `path` names RapidQ's RAPIDQ.INC.
pub fn is_rapidq_inc_name(path: &str) -> bool {
    path.rsplit(['/', '\\']).next().is_some_and(|n| n.trim().eq_ignore_ascii_case("RAPIDQ.INC"))
}

pub(crate) fn plan_parsed(entry: &Path, tp: &ToolsParse, options: &Options) -> ProgramPlan {
    let own = own_types(&tp.program);
    let files = &tp.preprocessed.origins.files;
    let mut plans: Vec<Option<FilePlan>> = files
        .iter()
        .map(|f| {
            let path = f.path.clone()?;
            if is_builtin(&path) {
                return None;
            }
            let is_rapidq_inc = is_rapidq_inc_name(&path.to_string_lossy());
            Some(FilePlan { path, text: f.text.clone(), encoding: f.encoding, edits: Vec::new(), changes: Vec::new(), is_rapidq_inc, rapidq_names: 0, rapidr_names: 0 })
        })
        .collect();
    let mut notes = Vec::new();
    let note = |notes: &mut Vec<Note>, fid: Option<FileId>, line: usize, message: String| {
        let path = fid.and_then(|f| files[f].path.clone());
        if !notes.iter().any(|n: &Note| n.path == path && n.line == line && n.message == message) {
            notes.push(Note { path, line, message });
        }
    };

    for span in &tp.type_names {
        let Some(lexeme) = tp.preprocessed.source.get(span.start..span.end) else { continue };
        count_style(tp, *span, lexeme, &own, &mut plans);
        let Some((meaning, to)) = target(lexeme, &own, options.normalize_case) else { continue };
        let Some(loc) = tp.locate(*span) else { continue };
        let file = &files[loc.file];
        let Some(plan) = plans[loc.file].as_mut() else { continue };
        if plan.is_rapidq_inc {
            continue;
        }
        let src = &file.text[loc.start..loc.end];
        let ok = if loc.exact && src.eq_ignore_ascii_case(lexeme) {
            true
        } else if is_identifier(src) && target(src, &own, options.normalize_case).is_some_and(|(m, _)| m == meaning) {
            // (a `$DEFINE` naming one of RapidQ's names: RAPIDQ2.INC's
            // QCOMPORT is COMPORT; written as RapidR's name it means the same)
            true
        } else {
            let (line, _) = file.line_col(loc.start);
            note(&mut notes, Some(loc.file), line, format!("`{src}` becomes {lexeme} through a $DEFINE or $MACRO: not converted"));
            false
        };
        if !ok || plan.edits.iter().any(|e| e.start == loc.start) {
            continue;
        }
        let (line, _) = file.line_col(loc.start);
        let column = file.text[file.line_starts()[line - 1]..loc.start].chars().count() + 1;
        plan.edits.push(Edit { start: loc.start, end: loc.end, text: to.clone() });
        plan.changes.push(Change { line, column, from: src.to_string(), to });
    }

    // (an `$INCLUDE` not found: its file isn't in the copy)
    for (loc, d) in tp.diagnostic_locations().into_iter().take(tp.preprocessor_diagnostics) {
        let line = loc.as_ref().map_or(0, |l| l.line);
        note(&mut notes, loc.map(|l| l.file), line, d.message.clone());
    }

    // What the parse didn't see: `$IFDEF` branches left out of this build,
    // `$DEFINE`s that name a component.
    for (fid, f) in files.iter().enumerate() {
        if plans[fid].as_ref().is_none_or(|p| p.is_rapidq_inc) {
            continue;
        }
        for (i, (line, kind)) in f.text.split('\n').zip(f.lines.iter()).enumerate() {
            match kind {
                LineKind::Inactive => {
                    for name in types_in_line(line) {
                        if target(&name, &own, options.normalize_case).is_some() {
                            note(&mut notes, Some(fid), i + 1, format!("`{name}` is in an $IFDEF branch this build doesn't compile: not converted"));
                        }
                    }
                }
                LineKind::Directive => {
                    let t = line.trim_start();
                    let Some(rest) = t.get(..7).filter(|d| d.eq_ignore_ascii_case("$DEFINE")).map(|_| &t[7..]) else { continue };
                    let mut words = rest.split_whitespace();
                    let (Some(name), Some(value)) = (words.next(), words.next()) else { continue };
                    if target(value, &own, options.normalize_case).is_some() && !rapidr_lang::component(name).is_some() {
                        note(&mut notes, Some(fid), i + 1, format!("$DEFINE {name} {value}: a $DEFINE's text isn't converted"));
                    }
                }
                _ => {}
            }
        }
    }

    for p in plans.iter_mut().flatten() {
        p.edits.sort_by_key(|e| e.start);
        p.changes.sort_by_key(|c| (c.line, c.column));
    }
    let mut out: Vec<FilePlan> = Vec::new();
    // (the main file first)
    let main = plans.iter().position(|p| p.as_ref().is_some_and(|p| same_file(&p.path, entry)));
    if let Some(m) = main {
        out.extend(plans[m].take());
    }
    out.extend(plans.into_iter().flatten());

    let resources = tp.preprocessed.resources.iter().filter_map(|r| r.path.clone()).collect();
    let errors = tp.diagnostics.iter().filter(|d| d.severity == rapidr_diagnostics::Severity::Error).count();
    let carry_rapidq_inc = crate::rapidq_inc::must_carry(tp, &out);
    let mut plan = ProgramPlan { entry: entry.to_path_buf(), files: out, notes, resources, carry_rapidq_inc, errors };
    crate::rapidq_inc::note_include_lines(&mut plan);
    plan
}

/// Counts a type name of a component RapidQ has in its file's style.
fn count_style(tp: &ToolsParse, span: rapidr_diagnostics::TextSpan, lexeme: &str, own: &[String], plans: &mut [Option<FilePlan>]) {
    if own.iter().any(|t| t.eq_ignore_ascii_case(lexeme)) {
        return;
    }
    let resolved = rapidr_ast::component_type_reference(lexeme, own);
    let Some(c) = rapidr_lang::component(&resolved).filter(|c| c.rapidq.is_some() && c.kind == rapidr_lang::Kind::Component) else { return };
    let Some(loc) = tp.locate(span).filter(|l| l.exact) else { return };
    let Some(plan) = plans.get_mut(loc.file).and_then(Option::as_mut) else { return };
    if lexeme.eq_ignore_ascii_case(c.name) {
        plan.rapidr_names += 1;
    } else {
        plan.rapidq_names += 1;
    }
}

pub(crate) fn same_file(a: &Path, b: &Path) -> bool {
    a == b || matches!((std::fs::canonicalize(a), std::fs::canonicalize(b)), (Ok(x), Ok(y)) if x == y)
}

/// The names after `AS` / `EXTENDS` on one line, as the lexer reads it.
fn types_in_line(line: &str) -> Vec<String> {
    let (tokens, _) = Lexer::new(line, None).tokenize_recovering();
    tokens
        .windows(2)
        .filter(|w| matches!(w[0].kind, TokenType::As | TokenType::Extends) && w[1].kind == TokenType::Identifier)
        .map(|w| w[1].lexeme.clone())
        .collect()
}
