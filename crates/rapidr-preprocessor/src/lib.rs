use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use rapidr_diagnostics::{Diagnostic, SourceLocation, TextSpan};

mod origin;
use origin::{MappedText, Output};
pub use origin::{decode_source, encode_source, FileId, LineKind, Origin, OriginMap, OriginSpan, Segment, SourceEncoding, SourceFile};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacroDefinition {
    params: Option<Vec<String>>,
    body: String,
}

impl MacroDefinition {
    fn new(params: Option<Vec<String>>, body: String) -> Self {
        Self { params, body }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PreprocessOptions {
    pub defines: HashMap<String, String>,
    /// Extra directories searched for `$INCLUDE` files after the including
    /// file's own directory (like RapidQ's `include\` folder). Native builds
    /// also search the `RAPIDR_INCLUDE_PATH` environment variable.
    pub include_dirs: Vec<PathBuf>,
    /// Files `$INCLUDE` finds without a file system — the web, where a
    /// project's files are in memory: (name as the project names it, its
    /// text). Looked up first, by the include's path or its last part, in
    /// any case.
    pub virtual_files: Vec<(String, String)>,
}

/// The virtual file `include_file` names ([`PreprocessOptions::virtual_files`]).
fn find_virtual<'a>(files: &'a [(String, String)], include_file: &str) -> Option<&'a (String, String)> {
    let wanted = include_file.trim().replace('\\', "/");
    let base = wanted.rsplit('/').next().unwrap_or(&wanted).to_string();
    files
        .iter()
        .find(|(name, _)| name.replace('\\', "/").eq_ignore_ascii_case(&wanted))
        .or_else(|| files.iter().find(|(name, _)| name.replace('\\', "/").rsplit('/').next().is_some_and(|b| b.eq_ignore_ascii_case(&base))))
}

/// State shared by a file and everything it includes: a `$DEFINE` or `$MACRO`
/// in an include file is visible to the code after the `$INCLUDE`.
struct PpState {
    defines: HashMap<String, String>,
    macros: HashMap<String, MacroDefinition>,
    include_stack: Vec<PathBuf>,
    include_dirs: Vec<PathBuf>,
    virtual_files: Vec<(String, String)>,
    app_type: Option<String>,
    resources: Vec<Resource>,
    /// `$ESCAPECHARS ON` is in effect (it belongs to the file it's in: an
    /// include file starts with it off, and the includer's setting comes back
    /// after the include).
    escape_chars: bool,
    /// RapidR's libraries put before the program (`RAPIDR_LIBRARIES`' file
    /// names, upper case): their RapidQ include files aren't read.
    libraries: Vec<String>,
    /// Every file read so far (the origin map's files).
    files: Vec<SourceFile>,
    /// False when only one text's lines are classified ([`scan_lines`]):
    /// `$INCLUDE`s aren't read.
    follow_includes: bool,
    /// Recovering ([`preprocess_file_recovering`]): errors collected here,
    /// the line in error left out, and preprocessing goes on.
    errors: Option<Vec<PreprocessError>>,
}

impl PpState {
    fn new(options: PreprocessOptions) -> Self {
        let mut defines = options.defines;
        // Built-in definition of the Windows version of RapidQ, which RapidR
        // emulates; programs use `$IFDEF WIN32` around their includes.
        defines.entry("WIN32".to_string()).or_insert_with(|| "WIN32".to_string());
        Self {
            defines,
            macros: HashMap::new(),
            include_stack: Vec::new(),
            include_dirs: options.include_dirs,
            virtual_files: options.virtual_files,
            app_type: None,
            resources: Vec::new(),
            escape_chars: false,
            libraries: Vec::new(),
            files: Vec::new(),
            follow_includes: true,
            errors: None,
        }
    }

    /// The file's id, registering it the first time it's read (`fresh`).
    fn file_id(&mut self, path: &Option<PathBuf>, text: &str, encoding: SourceEncoding) -> (FileId, bool) {
        if path.is_some() {
            if let Some(id) = self.files.iter().position(|f| &f.path == path) {
                return (id, false);
            }
        }
        self.files.push(SourceFile {
            path: path.clone(),
            text: text.to_string(),
            encoding,
            lines: vec![LineKind::Code; text.split('\n').count()],
        });
        (self.files.len() - 1, true)
    }

    /// Value of a VB `#If` condition: True/False, a number, a `#Const` or
    /// `$DEFINE` name, optionally negated with `Not`.
    fn vb_condition(&self, condition: &str) -> bool {
        let condition = condition.trim();
        if let Some(rest) = condition.strip_prefix("NOT ") {
            return !self.vb_condition(rest);
        }
        match condition {
            "TRUE" => true,
            "FALSE" | "" => false,
            _ => {
                if let Ok(n) = condition.parse::<f64>() {
                    return n != 0.0;
                }
                self.defines
                    .iter()
                    .find(|(key, _)| key.eq_ignore_ascii_case(condition))
                    .is_some_and(|(_, value)| {
                        let v = value.trim();
                        !(v.eq_ignore_ascii_case("FALSE") || v == "0")
                    })
            }
        }
    }

    fn is_defined(&self, symbol: &str) -> bool {
        self.defines.keys().any(|key| key.eq_ignore_ascii_case(symbol))
    }
}

/// Reads a source file. RapidQ programs are usually Windows-1252 (ANSI), not
/// UTF-8; bytes that aren't valid UTF-8 are decoded as Windows-1252.
pub fn read_source(path: &Path) -> std::io::Result<String> {
    Ok(read_source_with_encoding(path)?.0)
}

/// [`read_source`], also saying how the file was decoded.
pub fn read_source_with_encoding(path: &Path) -> std::io::Result<(String, SourceEncoding)> {
    Ok(decode_source(&fs::read(path)?))
}

const WINDOWS_1252_HIGH: [char; 32] = [
    '\u{20AC}', '\u{81}', '\u{201A}', '\u{192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{2C6}', '\u{2030}', '\u{160}', '\u{2039}', '\u{152}', '\u{8D}', '\u{17D}', '\u{8F}',
    '\u{90}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{2DC}', '\u{2122}', '\u{161}', '\u{203A}', '\u{153}', '\u{9D}', '\u{17E}', '\u{178}',
];

fn windows_1252_char(byte: u8) -> char {
    match byte {
        0x80..=0x9F => WINDOWS_1252_HIGH[(byte - 0x80) as usize],
        _ => byte as char,
    }
}

/// The Windows-1252 byte of a character (`?` when it has none).
fn windows_1252_byte(c: char) -> u8 {
    match WINDOWS_1252_HIGH.iter().position(|&h| h == c) {
        Some(i) => 0x80 + i as u8,
        None if (c as u32) < 0x80 || (0xA0..=0xFF).contains(&(c as u32)) => c as u32 as u8,
        None => b'?',
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreprocessResult {
    pub source: String,
    /// For each line of `source` (index = line - 1): the file and line it
    /// came from, so errors in `$INCLUDE`d code point at the right place.
    pub line_map: Vec<LineOrigin>,
    /// The value of `$APPTYPE` if present (e.g. "GUI", "CONSOLE", "WEB").
    pub app_type: Option<String>,
    /// `$RESOURCE NAME AS "file"`, in order: the name, the file as written
    /// and where it was found (`None`: not found). Resource `i` has the
    /// handle `RESOURCE_BASE + i`, which the directive defines as `NAME`.
    pub resources: Vec<Resource>,
    /// Where each byte of `source` came from (file and byte offset), and
    /// the text of every file read.
    pub origins: OriginMap,
}

/// One `$RESOURCE` of a program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    pub name: String,
    pub file: String,
    pub path: Option<PathBuf>,
    /// Built in if it's there, empty if not (`$OPTION ICON`'s icon: a
    /// missing one leaves the default icon rather than failing the build).
    pub optional: bool,
}

/// Handle of the first resource (`RESOURCE(0)`); RapidQ's handles are the
/// resource's position in the program, so never 0.
pub const RESOURCE_BASE: i64 = 65536;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreprocessError {
    pub diagnostic: Diagnostic,
}

impl PreprocessError {
    fn new(
        message: impl Into<String>,
        line: usize,
        column: usize,
        file_path: Option<String>,
    ) -> Self {
        Self {
            diagnostic: Diagnostic::error(
                message,
                TextSpan::new(0, 0),
                SourceLocation::new(line, column),
                file_path,
            ),
        }
    }
}

impl fmt::Display for PreprocessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.diagnostic.fmt(f)
    }
}

impl Error for PreprocessError {}

pub fn preprocess_file(
    path: impl AsRef<Path>,
    options: PreprocessOptions,
) -> Result<PreprocessResult, PreprocessError> {
    let path = path.as_ref();
    let mut options = options;
    if let Some(paths) = std::env::var_os("RAPIDR_INCLUDE_PATH") {
        options.include_dirs.extend(std::env::split_paths(&paths));
    }
    let (source, encoding) = read_source_with_encoding(path).map_err(|error| {
        PreprocessError::new(
            format!("Failed to read source file '{}': {error}", path.display()),
            1,
            1,
            Some(path.display().to_string()),
        )
    })?;

    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut state = PpState::new(options);
    let out = preprocess_program(&source, encoding, base_dir, Some(path.to_path_buf()), &mut state)?;
    Ok(finish(out, state))
}

pub fn preprocess_source(
    source: &str,
    base_dir: impl AsRef<Path>,
    file_path: Option<PathBuf>,
    options: PreprocessOptions,
) -> Result<PreprocessResult, PreprocessError> {
    let mut state = PpState::new(options);
    let out = preprocess_program(source, SourceEncoding::Utf8, base_dir.as_ref(), file_path, &mut state)?;
    Ok(finish(out, state))
}

/// [`preprocess_file`] for tools (editors): an `$INCLUDE` that isn't found
/// or can't be read, or a malformed `$INCLUDE` / `$RESOURCE`, is reported
/// and its line left out, and preprocessing goes on (the error's span is
/// that line's bytes in its file). Fails only when `path` can't be read.
pub fn preprocess_file_recovering(path: impl AsRef<Path>, options: PreprocessOptions) -> Result<(PreprocessResult, Vec<PreprocessError>), PreprocessError> {
    let path = path.as_ref();
    let mut options = options;
    if let Some(paths) = std::env::var_os("RAPIDR_INCLUDE_PATH") {
        options.include_dirs.extend(std::env::split_paths(&paths));
    }
    let (source, encoding) = read_source_with_encoding(path).map_err(|error| {
        PreprocessError::new(format!("Failed to read source file '{}': {error}", path.display()), 1, 1, Some(path.display().to_string()))
    })?;
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut state = PpState::new(options);
    state.errors = Some(Vec::new());
    let out = preprocess_program(&source, encoding, base_dir, Some(path.to_path_buf()), &mut state)?;
    let errors = state.errors.take().unwrap_or_default();
    Ok((finish(out, state), errors))
}

/// [`preprocess_source`] for tools: errors reported, never fatal (see
/// [`preprocess_file_recovering`]).
pub fn preprocess_source_recovering(
    source: &str,
    base_dir: impl AsRef<Path>,
    file_path: Option<PathBuf>,
    options: PreprocessOptions,
) -> (PreprocessResult, Vec<PreprocessError>) {
    let mut state = PpState::new(options);
    state.errors = Some(Vec::new());
    match preprocess_program(source, SourceEncoding::Utf8, base_dir.as_ref(), file_path, &mut state) {
        Ok(out) => {
            let errors = state.errors.take().unwrap_or_default();
            (finish(out, state), errors)
        }
        // (recovering never fails; kept total anyway)
        Err(error) => (finish(Output::default(), state), vec![error]),
    }
}

fn finish(out: Output, state: PpState) -> PreprocessResult {
    PreprocessResult {
        source: out.text,
        line_map: out.line_map,
        app_type: state.app_type,
        resources: state.resources,
        origins: OriginMap { files: state.files, segments: out.segs },
    }
}

/// What each line of `source` is to the preprocessor ([`LineKind`]), for
/// one file on its own: its `$INCLUDE`s aren't read (so a symbol only an
/// include file defines counts as undefined in its `$IFDEF`s). Never fails.
pub fn scan_lines(source: &str, options: PreprocessOptions) -> Vec<LineKind> {
    let mut state = PpState::new(options);
    state.follow_includes = false;
    state.errors = Some(Vec::new());
    let file = PathBuf::from("<scanned>");
    match preprocess_program(source, SourceEncoding::Utf8, Path::new("."), Some(file.clone()), &mut state) {
        Ok(_) => state.files.into_iter().find(|f| f.path.as_ref() == Some(&file)).map(|f| f.lines).unwrap_or_default(),
        Err(_) => vec![LineKind::Code; source.split('\n').count()],
    }
}

/// RapidR's own versions of the components RapidQ's include libraries
/// define with Windows calls (docs/desktop-host-plan.md, "RapidQ's remaining
/// UI objects"): (the component's name, the include file it stands for,
/// RapidR's library). A program that names the component gets the library
/// first (its lines come from `<RapidR>/<file>`); a RapidQ library of the
/// same name is never read (QDirListView.inc), or its own definition stays
/// out (RAPIDQ2.INC's QDOCKFORM, by the `__QDF_INC` guard both use).
pub const RAPIDR_LIBRARIES: &[(&str, &str, &str)] = &[
    ("QDOCKFORM", "QDockForm.inc", include_str!("libraries/QDockForm.inc")),
    ("QDIRLISTVIEW", "QDirListView.inc", include_str!("libraries/QDirListView.inc")),
];

/// Whether `source` names `word` (any case) as a whole word.
fn names_word(source: &str, word: &str) -> bool {
    let upper = source.to_ascii_uppercase();
    let bytes = upper.as_bytes();
    let mut from = 0;
    while let Some(i) = upper[from..].find(word) {
        let at = from + i;
        let before = at.checked_sub(1).map(|b| bytes[b]);
        let after = bytes.get(at + word.len()).copied();
        if !before.is_some_and(is_identifier_byte) && !after.is_some_and(is_identifier_byte) {
            return true;
        }
        from = at + word.len();
    }
    false
}

/// The program, after RapidR's libraries for the components it names.
fn preprocess_program(source: &str, encoding: SourceEncoding, base_dir: &Path, file_path: Option<PathBuf>, state: &mut PpState) -> Result<Output, PreprocessError> {
    let mut out = Output::default();
    for (name, file, text) in RAPIDR_LIBRARIES {
        if names_word(source, name) {
            let (lib, lib_id) = preprocess_with_state(text, SourceEncoding::Utf8, base_dir, Some(PathBuf::from(format!("<RapidR>/{file}"))), state)?;
            out.push_output(lib, (lib_id, text.len(), false));
            state.libraries.push(file.to_ascii_uppercase());
        }
    }
    let (main, main_id) = preprocess_with_state(source, encoding, base_dir, file_path, state)?;
    out.push_output(main, (main_id, source.len(), false));
    Ok(out)
}

fn preprocess_with_state(
    source: &str,
    encoding: SourceEncoding,
    base_dir: &Path,
    file_path: Option<PathBuf>,
    state: &mut PpState,
) -> Result<(Output, FileId), PreprocessError> {
    let (file_id, fresh) = state.file_id(&file_path, source, encoding);
    let mut kinds = vec![LineKind::Code; if fresh { source.split('\n').count() } else { 0 }];
    let mut out = Output::default();
    let mut skip_stack: Vec<bool> = Vec::new();
    // One entry per open `#If`: whether one of its branches was taken.
    let mut vb_taken: Vec<bool> = Vec::new();
    let file_label = file_path.as_ref().map(|path| path.display().to_string());
    let mut line_start = 0;

    for (line_index, original_line) in source.split('\n').enumerate() {
        let line_number = line_index + 1;
        let line = original_line.trim();
        let upper_line = line.to_ascii_uppercase();
        // (where this line is in the file, and the `\n` after it)
        let range = line_start..line_start + original_line.len();
        let newline = (file_id, range.end, range.end < source.len());
        line_start = range.end + 1;
        // (one output line for this source line, and what the line was)
        macro_rules! emit {
            ($out:expr, $kind:expr, $text:expr) => {{
                if let Some(k) = kinds.get_mut(line_index) {
                    *k = $kind;
                }
                $out.push_line($text, (file_path.clone(), line_number), newline);
            }};
        }
        let blank = MappedText::default;
        let generated = |text: String| MappedText::generated(text, file_id, range.clone());

        // `#!/usr/bin/env rapidr`: a script's first line, for the shell.
        if line_index == 0 && original_line.starts_with("#!") {
            emit!(out, LineKind::Shebang, blank());
            continue;
        }

        if upper_line.starts_with("$IFDEF") {
            let symbol = line
                .split_once(char::is_whitespace)
                .map(|(_, value)| strip_inline_comment(value).trim().to_string())
                .unwrap_or_default();
            let should_skip = !state.is_defined(&symbol);
            if skip_stack.last().copied().unwrap_or(false) {
                skip_stack.push(true);
            } else {
                skip_stack.push(should_skip);
            }
            emit!(out, LineKind::Directive, blank());
            continue;
        }

        // VB conditional compilation: `#If False Then`, `#If DEBUG Then`,
        // `#ElseIf`, `#Else`, `#End If`, `#Const NAME = value`.
        if let Some(rest) = upper_line.strip_prefix("#IF ") {
            let condition = rest.trim().strip_suffix("THEN").unwrap_or(rest).trim();
            let parent_skip = skip_stack.last().copied().unwrap_or(false);
            skip_stack.push(parent_skip || !state.vb_condition(condition));
            vb_taken.push(!skip_stack.last().copied().unwrap_or(true));
            emit!(out, LineKind::Directive, blank());
            continue;
        }
        if let Some(rest) = upper_line.strip_prefix("#ELSEIF ") {
            if let (Some(taken), Some(_)) = (vb_taken.last().copied(), skip_stack.last()) {
                let parent_skip = skip_stack.len() > 1 && skip_stack[skip_stack.len() - 2];
                let condition = rest.trim().strip_suffix("THEN").unwrap_or(rest).trim();
                let now = !parent_skip && !taken && state.vb_condition(condition);
                let last = skip_stack.len() - 1;
                skip_stack[last] = !now;
                *vb_taken.last_mut().unwrap() = taken || now;
            }
            emit!(out, LineKind::Directive, blank());
            continue;
        }
        if upper_line == "#ELSE" || upper_line.starts_with("#ELSE ") || upper_line.starts_with("#ELSE'") {
            if let Some(taken) = vb_taken.last().copied() {
                let parent_skip = skip_stack.len() > 1 && skip_stack[skip_stack.len() - 2];
                let last = skip_stack.len() - 1;
                skip_stack[last] = parent_skip || taken;
            }
            emit!(out, LineKind::Directive, blank());
            continue;
        }
        if upper_line.starts_with("#END IF") || upper_line.starts_with("#ENDIF") {
            if vb_taken.pop().is_some() {
                skip_stack.pop();
            }
            emit!(out, LineKind::Directive, blank());
            continue;
        }
        if let Some(rest) = upper_line.strip_prefix("#CONST ") {
            if !skip_stack.last().copied().unwrap_or(false) {
                if let Some((name, value)) = line["#CONST ".len()..].split_once('=') {
                    let _ = rest;
                    state.defines.insert(name.trim().to_string(), strip_inline_comment(value).trim().to_string());
                }
            }
            emit!(out, LineKind::Directive, blank());
            continue;
        }

        if upper_line.starts_with("$IFNDEF") {
            let symbol = line
                .split_once(char::is_whitespace)
                .map(|(_, value)| strip_inline_comment(value).trim().to_string())
                .unwrap_or_default();
            let should_skip = state.is_defined(&symbol);
            if skip_stack.last().copied().unwrap_or(false) {
                skip_stack.push(true);
            } else {
                skip_stack.push(should_skip);
            }
            emit!(out, LineKind::Directive, blank());
            continue;
        }

        if upper_line.starts_with("$ELSE") {
            if !skip_stack.is_empty() {
                let parent_skip = if skip_stack.len() > 1 {
                    skip_stack[skip_stack.len() - 2]
                } else {
                    false
                };
                if !parent_skip {
                    let last = skip_stack.len() - 1;
                    skip_stack[last] = !skip_stack[last];
                }
            }
            emit!(out, LineKind::Directive, blank());
            continue;
        }

        if upper_line.starts_with("$ENDIF") {
            if !skip_stack.is_empty() {
                skip_stack.pop();
            }
            emit!(out, LineKind::Directive, blank());
            continue;
        }

        if skip_stack.last().copied().unwrap_or(false) {
            emit!(out, LineKind::Inactive, blank());
            continue;
        }

        if upper_line.starts_with("$DEFINE") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let symbol = parts[1].to_string();
                let value = if parts.len() > 2 {
                    strip_inline_comment(&parts[2..].join(" ")).trim().to_string()
                } else {
                    "1".to_string()
                };
                state.defines.insert(symbol, value);
            }
            emit!(out, LineKind::Directive, blank());
            continue;
        }

        if upper_line.starts_with("$UNDEF") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                state.defines.retain(|key, _| !key.eq_ignore_ascii_case(parts[1]));
            }
            emit!(out, LineKind::Directive, blank());
            continue;
        }

        if upper_line.starts_with("$MACRO") {
            if let Some((name, definition)) = parse_macro_definition(line) {
                state.macros.insert(name, definition);
            }
            emit!(out, LineKind::Directive, blank());
            continue;
        }

        if upper_line.starts_with("$APPTYPE")
            || upper_line.starts_with("$OPTIMIZE")
            || upper_line.starts_with("$ESCAPECHARS")
            || upper_line.starts_with("$THEME")
        {
            if upper_line.starts_with("$ESCAPECHARS") {
                state.escape_chars = strip_inline_comment(line.split_once(char::is_whitespace).map_or("", |(_, v)| v)).trim().eq_ignore_ascii_case("ON");
            }
            // Extract $APPTYPE value
            if upper_line.starts_with("$APPTYPE") {
                if let Some((_, value)) = line.split_once(char::is_whitespace) {
                    let val = strip_inline_comment(value).trim().to_uppercase();
                    if !val.is_empty() {
                        state.app_type = Some(val);
                    }
                }
            }
            emit!(out, LineKind::Code, MappedText::exact(original_line, file_id, range.start));
            continue;
        }

        // `$OPTION ICON "app.ico"`: the program's icon — built in as a
        // resource and made the application's (every form's without its
        // own). An icon that isn't there leaves the default one.
        if upper_line.starts_with("$OPTION") && upper_line["$OPTION".len()..].trim_start().starts_with("ICON") {
            let file = line.find('"').and_then(|a| line[a + 1..].find('"').map(|b| line[a + 1..a + 1 + b].to_string()));
            // (resolved here on the desktop; the web finds it in the project's assets)
            let text = match file {
                Some(file) => {
                    let handle = RESOURCE_BASE + state.resources.len() as i64;
                    let path = resolve_include_path(base_dir, &file, &[]);
                    state.resources.push(Resource { name: "RAPIDR_OPTION_ICON".into(), file, path, optional: true });
                    format!("CONST RAPIDR_OPTION_ICON = {handle} : Application.IcoHandle = RAPIDR_OPTION_ICON")
                }
                None => String::new(),
            };
            emit!(out, LineKind::Directive, generated(text));
            continue;
        }

        // `$RESOURCE NAME AS "file"` (several may share a line, separated
        // by `:`): `NAME` is the resource's handle.
        if upper_line.starts_with("$RESOURCE") {
            let mut consts = Vec::new();
            for part in split_statements(line) {
                let Some((name, file)) = parse_resource(part.trim()) else {
                    let mut error = PreprocessError::new(format!("Invalid $RESOURCE syntax: {}", part.trim()), line_number, 1, file_label.clone());
                    match &mut state.errors {
                        // (recovering: reported, the part left out)
                        Some(errors) => {
                            error.diagnostic.span = TextSpan::new(range.start, range.end);
                            errors.push(error);
                            continue;
                        }
                        None => return Err(error),
                    }
                };
                let handle = RESOURCE_BASE + state.resources.len() as i64;
                let path = resolve_include_path(base_dir, &file, &[]);
                state.resources.push(Resource { name: name.clone(), file, path, optional: false });
                consts.push(format!("CONST {name} = {handle}"));
            }
            emit!(out, LineKind::Directive, generated(consts.join(" : ")));
            continue;
        }

        if upper_line.starts_with("$INCLUDE") {
            // (classifying one file's lines: its includes aren't read)
            if !state.follow_includes {
                emit!(out, LineKind::Directive, blank());
                continue;
            }
            // (an error: the build stops — or, recovering, it's reported and
            // the line left out)
            macro_rules! fail {
                ($message:expr) => {{
                    let mut error = PreprocessError::new($message, line_number, 1, file_label.clone());
                    match &mut state.errors {
                        Some(errors) => {
                            error.diagnostic.span = TextSpan::new(range.start, range.end);
                            errors.push(error);
                            emit!(out, LineKind::Directive, blank());
                            continue;
                        }
                        None => return Err(error),
                    }
                }};
            }
            let Some(include_file) = parse_include_target(line) else {
                fail!(format!("Invalid $INCLUDE syntax: {line}"));
            };
            if let Some(k) = kinds.get_mut(line_index) {
                *k = LineKind::Directive;
            }

            // A RapidQ library RapidR has its own version of: that one, put
            // before the program already (or now).
            let short = Path::new(&include_file.replace('\\', "/")).file_name().and_then(|n| n.to_str()).unwrap_or("").to_ascii_uppercase();
            if let Some((_, file, text)) = RAPIDR_LIBRARIES.iter().find(|(_, f, _)| f.to_ascii_uppercase() == short) {
                if !state.libraries.contains(&short) {
                    state.libraries.push(short);
                    let (lib, _) = preprocess_with_state(text, SourceEncoding::Utf8, base_dir, Some(PathBuf::from(format!("<RapidR>/{file}"))), state)?;
                    out.push_output(lib, newline);
                } else {
                    emit!(out, LineKind::Directive, blank());
                }
                continue;
            }
            let virtual_file = find_virtual(&state.virtual_files, &include_file).cloned();
            let include_path = match virtual_file.as_ref().map(|(name, _)| PathBuf::from(name)).or_else(|| resolve_include_path(base_dir, &include_file, &state.include_dirs)) {
                Some(path) => path,
                None => {
                    // RapidQ programs start with `$INCLUDE "RAPIDQ.INC"`; supply
                    // its constants when the file isn't next to the program.
                    if let Some(builtin) = builtin_include(&include_file) {
                        emit!(out, LineKind::Directive, generated(builtin));
                        continue;
                    }
                    fail!(format!("Include file not found: '{include_file}'"));
                }
            };

            if state.include_stack.iter().any(|entry| entry == &include_path) {
                fail!(format!("Recursive include detected: '{include_file}'"));
            }

            // (an IDE's in-memory file first: PreprocessOptions::virtual_files)
            let read = match virtual_file {
                Some((_, text)) => Ok((text, SourceEncoding::default())),
                None => read_source_with_encoding(&include_path),
            };
            let (include_source, include_encoding) = match read {
                Ok(read) => read,
                Err(error) => fail!(format!("Failed to include '{include_file}': {error}")),
            };

            // The include file has its own $ESCAPECHARS: off to begin with.
            let escape_before = state.escape_chars;
            if escape_before {
                emit!(out, LineKind::Directive, generated("$ESCAPECHARS OFF".to_string()));
                state.escape_chars = false;
            }
            state.include_stack.push(include_path.clone());
            let nested = preprocess_with_state(
                &include_source,
                include_encoding,
                include_path.parent().unwrap_or_else(|| Path::new(".")),
                Some(include_path.clone()),
                state,
            );
            state.include_stack.pop();
            let (nested, _) = nested?;
            out.push_output(nested, newline);
            // …and the includer's comes back.
            if state.escape_chars != escape_before {
                let text = if escape_before { "$ESCAPECHARS ON" } else { "$ESCAPECHARS OFF" };
                emit!(out, LineKind::Directive, generated(text.to_string()));
                state.escape_chars = escape_before;
            }
            continue;
        }

        let mut processed_line = MappedText::exact(original_line, file_id, range.start);

        if !state.macros.is_empty() {
            for (name, definition) in &state.macros {
                expand_macro(&mut processed_line, name, definition);
            }
        }

        if !state.defines.is_empty() && !upper_line.contains('$') {
            substitute_defines_outside_strings(&mut processed_line, &state.defines);
        }

        emit!(out, LineKind::Code, processed_line);
    }

    if fresh {
        state.files[file_id].lines = kinds;
    }
    Ok((out, file_id))
}

impl PreprocessResult {
    /// Rewrites `main:LINE:COL: …` positions in compiler messages (whose
    /// LINE counts preprocessed lines) to the file and line the code really
    /// came from, e.g. `windows.inc:120:5: …` for code in an include.
    pub fn remap_messages(&self, main: &str, text: &str) -> String {
        let prefix = format!("{main}:");
        text.lines()
            .map(|message| {
                let Some(rest) = message.strip_prefix(&prefix) else { return message.to_string() };
                let Some((line, tail)) = rest.split_once(':') else { return message.to_string() };
                let Ok(line) = line.parse::<usize>() else { return message.to_string() };
                match self.line_map.get(line.wrapping_sub(1)) {
                    Some((Some(file), original)) => format!("{}:{original}:{tail}", file.display()),
                    Some((None, original)) => format!("{main}:{original}:{tail}"),
                    None => message.to_string(),
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Where one line of preprocessed source came from.
pub type LineOrigin = (Option<PathBuf>, usize);

/// The constants a RapidQ program gets from `$INCLUDE "RAPIDQ.INC"` when no
/// such file is next to it (RapidR doesn't ship RapidQ's include files).
///
/// Every entry is a name and a number: an interface fact that existing
/// programs depend on, kept so that they run unchanged. Nothing else of
/// RapidQ's file is reproduced — no comments, layout or order. The entries
/// are grouped here by where each number is publicly defined, alphabetically
/// within a group, and each group says its source (docs/legal/rapidq-review.md,
/// "RAPIDQ.INC"). Colours are &HBBGGRR numbers, as RapidR's runtimes use them;
/// a system colour is drawn in the theme's colours
/// (`rapidr_value::objects::color_bgr`). No name appears twice (a test checks),
/// so the order has no effect on any program.
pub const RAPIDQ_INC_CONSTANTS: &[(&str, i64)] = &[
    // ── Windows SDK numbers (Microsoft's public Win32 headers and
    // documentation, learn.microsoft.com). ──
    // MessageBox flags (winuser.h)
    ("MB_ABORTRETRYIGNORE", 2), ("MB_ICONASTERISK", 64), ("MB_ICONERROR", 16),
    ("MB_ICONEXCLAMATION", 48), ("MB_ICONHAND", 16), ("MB_ICONINFORMATION", 64),
    ("MB_ICONQUESTION", 32), ("MB_ICONSTOP", 16), ("MB_ICONWARNING", 48), ("MB_OK", 0),
    ("MB_OKCANCEL", 1), ("MB_RETRYCANCEL", 5), ("MB_YESNO", 4), ("MB_YESNOCANCEL", 3),
    // MessageBox results (winuser.h)
    ("IDABORT", 3), ("IDCANCEL", 2), ("IDIGNORE", 5), ("IDNO", 7), ("IDOK", 1), ("IDRETRY", 4),
    ("IDYES", 6),
    // virtual-key codes (winuser.h)
    ("VK_BACK", 8), ("VK_CONTROL", 17), ("VK_DELETE", 46), ("VK_DOWN", 40), ("VK_END", 35),
    ("VK_ESCAPE", 27), ("VK_F1", 112), ("VK_F10", 121), ("VK_F11", 122), ("VK_F12", 123),
    ("VK_F2", 113), ("VK_F3", 114), ("VK_F4", 115), ("VK_F5", 116), ("VK_F6", 117), ("VK_F7", 118),
    ("VK_F8", 119), ("VK_F9", 120), ("VK_HOME", 36), ("VK_INSERT", 45), ("VK_LEFT", 37),
    ("VK_MENU", 18), ("VK_NEXT", 34), ("VK_PAUSE", 19), ("VK_PRIOR", 33), ("VK_RETURN", 13),
    ("VK_RIGHT", 39), ("VK_SHIFT", 16), ("VK_SPACE", 32), ("VK_TAB", 9), ("VK_UP", 38),
    // PlaySound flags (mmsystem.h)
    ("SND_ASYNC", 1), ("SND_LOOP", 8), ("SND_MEMORY", 4), ("SND_NODEFAULT", 2), ("SND_NOSTOP", 16),
    ("SND_SYNC", 0),
    // font character sets (wingdi.h)
    ("ANSI_CHARSET", 0), ("ARABIC_CHARSET", 178), ("BALTIC_CHARSET", 186),
    ("CHINESEBIG5_CHARSET", 136), ("DEFAULT_CHARSET", 1), ("EASTEUROPE_CHARSET", 238),
    ("GB2312_CHARSET", 134), ("GREEK_CHARSET", 161), ("HANGEUL_CHARSET", 129),
    ("HEBREW_CHARSET", 177), ("JOHAB_CHARSET", 130), ("MAC_CHARSET", 77), ("OEM_CHARSET", 255),
    ("RUSSIAN_CHARSET", 204), ("SHIFTJIS_CHARSET", 128), ("SYMBOL_CHARSET", 2),
    ("THAI_CHARSET", 222), ("TURKISH_CHARSET", 162), ("VIETNAMESE_CHARSET", 163),
    // Windows Sockets protocol, socket type and address family numbers (winsock.h)
    ("AF_APPLETALK", 16), ("AF_BAN", 21), ("AF_CCITT", 10), ("AF_CHAOS", 5), ("AF_DATAKIT", 9),
    ("AF_DECnet", 12), ("AF_DLI", 13), ("AF_ECMA", 8), ("AF_FIREFOX", 19), ("AF_HYLINK", 15),
    ("AF_IMPLINK", 3), ("AF_INET", 2), ("AF_IPX", 6), ("AF_ISO", 7), ("AF_LAT", 14),
    ("AF_NETBIOS", 17), ("AF_NS", 6), ("AF_PUP", 4), ("AF_SNA", 11), ("AF_UNIX", 1),
    ("AF_UNKNOWN1", 20), ("AF_UNSPEC", 0), ("AF_VOICEVIEW", 18), ("IPPROTO_ICMP", 1),
    ("IPPROTO_IDP", 22), ("IPPROTO_IGMP", 2), ("IPPROTO_IP", 0), ("IPPROTO_PUP", 12),
    ("IPPROTO_RAW", 255), ("IPPROTO_TCP", 6), ("IPPROTO_UDP", 17), ("SOCK_DGRAM", 2),
    ("SOCK_RAW", 3), ("SOCK_RDM", 4), ("SOCK_SEQPACKET", 5), ("SOCK_STREAM", 1),
    // raster-operation codes (wingdi.h: BLACKNESS, SRCCOPY, …), under the cm… names of Delphi's TCopyMode
    ("cmBlackness", 0x000042), ("cmDstInvert", 0x550009), ("cmMergeCopy", 0xC000CA),
    ("cmMergePaint", 0xBB0226), ("cmNotSrcCopy", 0x330008), ("cmNotSrcErase", 0x1100A6),
    ("cmPatCopy", 0xF00021), ("cmPatInvert", 0x5A0049), ("cmPatPaint", 0xFB0A09),
    ("cmSrcAnd", 0x8800C6), ("cmSrcCopy", 0xCC0020), ("cmSrcErase", 0x440328),
    ("cmSrcInvert", 0x660046), ("cmSrcPaint", 0xEE0086), ("cmWhiteness", 0xFF0062),
    // colours as COLORREF numbers, &HBBGGRR (wingdi.h RGB)
    // (clGreen &H00FF00 and clPurple &HFF00FF are RapidQ's: programs see
    // those, not the usual &H008000 / &H800080.)
    ("clAqua", 0xFFFF00), ("clBlack", 0x000000), ("clBlue", 0xFF0000), ("clDkGray", 0x808080),
    ("clFuchsia", 0xFF00FF), ("clGray", 0x808080), ("clGreen", 0x00FF00), ("clLime", 0x00FF00),
    ("clLtGray", 0xC0C0C0), ("clMaroon", 0x000080), ("clNavy", 0x800000), ("clOlive", 0x008080),
    ("clPurple", 0xFF00FF), ("clRed", 0x0000FF), ("clSilver", 0xC0C0C0), ("clTeal", 0x808000),
    ("clWhite", 0xFFFFFF), ("clYellow", 0x00FFFF),
    // system colours: &H80000000 + the GetSysColor index (winuser.h COLOR_…), the encoding Delphi uses for TColor
    // (clInfoBk3DDkShadow is index 24, COLOR_INFOBK, under the name RapidQ
    // programs use.)
    ("cl3DDkShadow", -2147483627), ("cl3DLight", -2147483626), ("clActiveBorder", -2147483638),
    ("clActiveCaption", -2147483646), ("clAppWorkSpace", -2147483636),
    ("clBackGround", -2147483647), ("clBtnFace", -2147483633), ("clBtnHighlight", -2147483628),
    ("clBtnShadow", -2147483632), ("clBtnText", -2147483630), ("clCaptionText", -2147483639),
    ("clGrayText", -2147483631), ("clHighlight", -2147483635), ("clHighlightText", -2147483634),
    ("clHilight", -2147483635), ("clHilightText", -2147483634), ("clInActiveBorder", -2147483637),
    ("clInActiveCaption", -2147483645), ("clInActiveCaptionText", -2147483629),
    ("clInfoBk3DDkShadow", -2147483624), ("clInfoText", -2147483625), ("clMenu", -2147483644),
    ("clMenuText", -2147483641), ("clScrollBar", -2147483648), ("clWindow", -2147483643),
    ("clWindowFrame", -2147483642), ("clWindowText", -2147483640),
    // ── Names of the VCL types (Delphi's component library) that RapidQ's
    // components expose, with the numbers RapidQ programs see: an
    // enumeration's ordinal, i.e. its position in the type's declaration as
    // Embarcadero documents it (docwiki.embarcadero.com, under the type's
    // name), or a constant's value. Where RapidQ's number differs from
    // today's Delphi (crSize), RapidQ's is kept: programs depend on it. ──
    // file attributes (SysUtils fa…)
    ("faAnyFile", 63), ("faArchive", 32), ("faDirectory", 16), ("faHidden", 2), ("faReadOnly", 1),
    ("faSysFile", 4), ("faVolumeID", 8),
    // file-open modes (SysUtils)
    ("fmCreate", 65535), ("fmOpenRead", 0), ("fmOpenReadWrite", 2), ("fmOpenWrite", 1),
    // seek origins (Classes)
    ("soFromBeginning", 0), ("soFromCurrent", 1), ("soFromEnd", 2),
    // TAlign
    ("alBottom", 2), ("alClient", 5), ("alLeft", 3), ("alNone", 0), ("alRight", 4), ("alTop", 1),
    // TAlignment
    ("taCenter", 2), ("taLeftJustify", 0), ("taRightJustify", 1),
    // TBevelCut
    ("bvLowered", 1), ("bvNone", 0), ("bvRaised", 2),
    // TBitBtnKind
    ("bkAbort", 7), ("bkAll", 10), ("bkCancel", 2), ("bkClose", 6), ("bkCustom", 0), ("bkHelp", 3),
    ("bkIgnore", 9), ("bkNo", 5), ("bkOK", 1), ("bkRetry", 8), ("bkYes", 4),
    // TBorderIcon
    ("biHelp", 3), ("biMaximize", 2), ("biMinimize", 1), ("biSystemMenu", 0),
    // TCloseAction
    ("caFree", 2), ("caHide", 1), ("caMinimize", 3), ("caNone", 0),
    // TColor clNone / clDefault
    ("clDefault", 536870912), ("clNone", 536870911),
    // TComboBoxStyle
    ("csDropDown", 0), ("csDropDownList", 2), ("csOwnerDrawFixed", 3), ("csOwnerDrawVariable", 4),
    ("csSimple", 1),
    // TCursor (crDefault … crHandPoint)
    ("crAppStart", -19), ("crArrow", -2), ("crCross", -3), ("crDefault", 0), ("crDrag", -12),
    ("crHandPoint", -21), ("crHelp", -20), ("crHourGlass", -11), ("crHSplit", -14),
    ("crIBeam", -4), ("crMultiDrag", -16), ("crNo", -18), ("crNoDrop", -13), ("crNone", -1),
    ("crSize", -5), ("crSizeNESW", -6), ("crSizeNS", -7), ("crSizeNWSE", -8), ("crSizeWE", -9),
    ("crSQLWait", -17), ("crUpArrow", -10), ("crVSplit", -15),
    // TDuplicates
    ("dupAccept", 1), ("dupError", 2), ("dupIgnore", 0),
    // TEditCharCase
    ("ecLowerCase", 2), ("ecNormal", 0), ("ecUpperCase", 1),
    // TFileAttr (FileCtrl)
    ("ftArchive", 5), ("ftDirectory", 4), ("ftHidden", 1), ("ftNormal", 6), ("ftReadOnly", 0),
    ("ftSystem", 2), ("ftVolumeID", 3),
    // TFloatFormat
    ("ffExponent", 1), ("ffFixed", 2), ("ffGeneral", 0), ("ffNumber", 3),
    // TFontDialogOption (fdAnsiOnly … fdApplyButton)
    ("fdAnsiOnly", 0), ("fdApplyButton", 15), ("fdEffects", 2), ("fdFixedPitchOnly", 3),
    ("fdForceFontExist", 4), ("fdLimitSize", 13), ("fdNoFaceSel", 5), ("fdNoOEMFonts", 6),
    ("fdNoSimulations", 7), ("fdNoSizeSel", 8), ("fdNoStyleSel", 9), ("fdNoVectorFonts", 10),
    ("fdScalableOnly", 14), ("fdShowHelp", 11), ("fdTrueTypeOnly", 1), ("fdWysiwyg", 12),
    // TFontPitch
    ("fpDefault", 0), ("fpFixed", 2), ("fpVariable", 1),
    // TFontStyle
    ("fsBold", 0), ("fsItalic", 1), ("fsStrikeOut", 3), ("fsUnderline", 2),
    // TFormBorderStyle
    ("bsDialog", 3), ("bsNone", 0), ("bsSingle", 1), ("bsSizeable", 2), ("bsSizeToolWin", 5),
    ("bsToolWindow", 4),
    // TFormStyle
    ("fsMDIChild", 1), ("fsMDIForm", 2), ("fsNormal", 0), ("fsStayOnTop", 3),
    // TGaugeKind
    ("gkHorizontalBar", 1), ("gkNeedle", 4), ("gkPie", 3), ("gkText", 0), ("gkVerticalBar", 2),
    // TGridOption (goFixedVertLine … goThumbTracking)
    ("goAlwaysShowEditor", 13), ("goColMoving", 9), ("goColSizing", 7), ("goDrawFocusSelected", 5),
    ("goEditing", 10), ("goFixedHorzLine", 1), ("goFixedVertLine", 0), ("goHorzLine", 3),
    ("goRangeSelect", 4), ("goRowMoving", 8), ("goRowSelect", 12), ("goRowSizing", 6),
    ("goTabs", 11), ("goThumbTracking", 14), ("goVertLine", 2),
    // THeaderSectionStyle
    ("hsOwnerDraw", 1), ("hsText", 0),
    // TImageType
    ("itImage", 0), ("itMask", 1),
    // TListBoxStyle
    ("lbOwnerDrawFixed", 1), ("lbOwnerDrawVariable", 2), ("lbStandard", 0),
    // TModalResult
    ("mrAbort", 3), ("mrAll", 8), ("mrCancel", 2), ("mrIgnore", 5), ("mrNo", 7), ("mrNone", 0),
    ("mrNoToAll", 9), ("mrOk", 1), ("mrRetry", 4), ("mrYes", 6), ("mrYesToAll", 10),
    // TMouseButton
    ("mbLeft", 0), ("mbMiddle", 2), ("mbRight", 1),
    // TMsgDlgType
    ("mtConfirmation", 3), ("mtCustom", 4), ("mtError", 1), ("mtInformation", 2), ("mtWarning", 0),
    // TObjectState (OleCtnrs)
    ("osEmpty", 0), ("osInPlaceActive", 4), ("osLoaded", 1), ("osOpen", 3), ("osRunning", 2),
    ("osUIActive", 5),
    // TOutlineOption
    ("ooDrawFocusRect", 1), ("ooDrawStretchBitmaps", 2), ("ooDrawTreeRoot", 0),
    // TOutlineStyle
    ("osPictureText", 2), ("osPlusMinusPictureText", 3), ("osPlusMinusText", 1), ("osText", 0),
    ("osTreePictureText", 5), ("osTreeText", 4),
    // TPixelFormat
    ("pf15bit", 4), ("pf16bit", 5), ("pf1bit", 1), ("pf24bit", 6), ("pf32bit", 7), ("pf4bit", 2),
    ("pf8bit", 3), ("pfDevice", 0),
    // TPrinterOrientation
    ("poLandscape", 1), ("poPortrait", 0),
    // TScrollBarKind
    ("sbHorizontal", 0), ("sbVertical", 1),
    // TScrollCode
    ("scBottom", 7), ("scEndScroll", 8), ("scLineDown", 1), ("scLineUp", 0), ("scPageDown", 3),
    ("scPageUp", 2), ("scPosition", 4), ("scTop", 6), ("scTrack", 5),
    // TScrollStyle
    ("ssBoth", 3), ("ssHorizontal", 1), ("ssNone", 0), ("ssVertical", 2),
    // TSizeMode (OleCtnrs)
    ("smAutoSize", 4), ("smCenter", 1), ("smClip", 0), ("smScale", 2), ("smStretch", 3),
    // TSortType
    ("stNone", 0), ("stText", 2),
    // TTextLayout
    ("tlBottom", 2), ("tlCenter", 1), ("tlTop", 0),
    // TTickMark
    ("tmBoth", 2), ("tmBottomRight", 0), ("tmTopLeft", 1),
    // TTickStyle
    ("tsAuto", 1), ("tsManual", 2), ("tsNone", 0),
    // TTrackBarOrientation
    ("tbHorizontal", 0), ("tbVertical", 1),
    // TViewStyle
    ("vsIcon", 0), ("vsList", 2), ("vsReport", 3), ("vsSmallIcon", 1),
    // TWindowState
    ("wsMaximized", 2), ("wsMinimized", 1), ("wsNormal", 0),
    // ── RapidQ's own numbers, with no outside origin: TRUE is 1 (comparisons
    // themselves give -1 / 0), the Num_… sizes of ReadNum / WriteNum, the
    // shift-state and key-state bits, QComPort's baud / stop-bit / parity
    // codes, the MessageDlg button bits (mbYes 1 … mbAll 256), and the option
    // numbers of a few components. Each is a fact a RapidQ program may use. ──
    ("AltDown", 16), ("blBMPBottom", 3), ("blBMPLeft", 0), ("blBMPRight", 1), ("blBMPTop", 2),
    ("bpNone", 0), ("bpSingle", 1), ("br110", 0), ("br115200", 12), ("br1200", 3), ("br14400", 7),
    ("br19200", 8), ("br2400", 4), ("br300", 1), ("br38400", 9), ("br4800", 5), ("br56000", 10),
    ("br57600", 11), ("br600", 2), ("br9600", 6), ("caClose", 2), ("CtrlDown", 1), ("drtCDRom", 4),
    ("drtFixed", 2), ("drtRamDisk", 5), ("drtRemote", 3), ("drtRemovable", 1), ("drtUnknown", 0),
    ("dsFocused", 0), ("dsNormal", 2), ("dsSelected", 1), ("dsTransparent", 3), ("dtAll", 4),
    ("dtHidden", 1), ("dtNormal", 3), ("dtReadOnly", 0), ("dtSystem", 2), ("False", 0),
    ("gcsEllipsis", 1), ("gcsList", 0), ("gcsNone", 2), ("lsNone", 0), ("lsRaised", 1),
    ("lsRecessed", 2), ("mbAbort", 32), ("mbAll", 256), ("mbCancel", 8), ("mbHelp", 16),
    ("mbIgnore", 128), ("mbNo", 2), ("mbOK", 4), ("mbRetry", 64), ("mbYes", 1), ("Num_BYTE", 1),
    ("Num_DOUBLE", 8), ("Num_DWORD", 5), ("Num_LONG", 4), ("Num_SHORT", 2), ("Num_SINGLE", 6),
    ("Num_WORD", 3), ("prEven", 2), ("prMark", 3), ("prNone", 0), ("prOdd", 1), ("prSpace", 4),
    ("sbOne5StopBits", 1), ("sbOneStopBit", 0), ("sbTwoStopBits", 2), ("ShiftDown", 256),
    ("ssAlt", 1), ("ssCtrl", 16), ("ssShift", 256), ("tmAuto", 0), ("tmFixed", 1), ("True", 1),
];

/// RapidQ's library include files whose object RapidR has built in
/// (rapidr_ast::library), with the constants each defines: what such an
/// `$INCLUDE` gives when the file isn't on disk. Only the names and numbers
/// programs use with those objects (a player's state, a wave's format, the
/// CGI object's input limits) — interface facts, listed in numeric order; no
/// code or text of the libraries is in RapidR (their objects are RapidR's own
/// implementations of the behaviour), and `qcgi.inc`, which is GPL, is not
/// needed (docs/legal/rapidq-review.md, "Library includes").
const LIBRARY_INCLUDES: &[(&str, &[(&str, i64)])] = &[
    ("qcgi.inc", &[("CGI_INPUT_DEFAULT", 32767), ("CGI_INPUT_LARGE", 65535), ("CGI_INPUT_SMALL", 255), ("CGI_MAX_PAIRS", 256)]),
    ("qdownload.inc", &[]),
    ("qmidi.inc", &[("MD_CLOSE", 0), ("MD_PLAY", 1), ("MD_PAUSE", 2), ("MD_STOP", 3)]),
    ("qwave.inc", &[("WV_CLOSE", 0), ("WV_PLAY", 1), ("WV_PAUSE", 2), ("WV_STOP", 3), ("WV_RECORD", 4), ("WV_MONO", 1), ("WV_STEREO", 2), ("WV_BIT8", 8), ("WV_BIT16", 16), ("WV_KHZ8", 8000), ("WV_KHZ11", 11025), ("WV_KHZ22", 22050), ("WV_KHZ44", 44100)]),
    ("qvideo.inc", &[("VD_CLOSE", 0), ("VD_PLAY", 1), ("VD_PAUSE", 2), ("VD_STOP", 3)]),
    ("qcdaudio.inc", &[("CD_CLOSE", 0), ("CD_PLAY", 1), ("CD_PAUSE", 2), ("CD_STOP", 3)]),
];

/// Built-in replacement for an include file that isn't on disk, as a single
/// line (so line numbers after the $INCLUDE stay correct).
fn builtin_include(include_file: &str) -> Option<String> {
    // (RapidQ's include folders use `\`: `Object\QMidi.inc`)
    let name = include_file.rsplit(['/', '\\']).next()?;
    // RapidQ's libraries whose objects RapidR has built in
    // (rapidr_ast::library): their constants, the object being RapidR's.
    if let Some((_, consts)) = LIBRARY_INCLUDES.iter().find(|(file, _)| file.eq_ignore_ascii_case(name)) {
        return Some(consts.iter().map(|(n, v)| format!("CONST {n} = {v}")).collect::<Vec<_>>().join(" : "));
    }
    if !name.eq_ignore_ascii_case("RAPIDQ.INC") {
        return None;
    }
    Some(
        RAPIDQ_INC_CONSTANTS
            .iter()
            .map(|(name, value)| format!("CONST {name} = {value}"))
            .collect::<Vec<_>>()
            .join(" : "),
    )
}

fn strip_inline_comment(input: &str) -> &str {
    input.split('\'').next().unwrap_or(input)
}

fn parse_macro_definition(line: &str) -> Option<(String, MacroDefinition)> {
    let trimmed = line.trim();
    let after_macro = trimmed.strip_prefix("$MACRO")?.trim();
    let (head, body) = after_macro.split_once('=')?;
    let body = strip_inline_comment(body).trim().to_string();
    let head = head.trim();

    if let Some(paren_index) = head.find('(') {
        let close_index = head.rfind(')')?;
        let name = head[..paren_index].trim().to_string();
        let params_text = &head[paren_index + 1..close_index];
        let params = params_text
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        Some((name, MacroDefinition::new(Some(params), body)))
    } else {
        Some((head.to_string(), MacroDefinition::new(None, body)))
    }
}

/// A line's `:`-separated parts (outside quotes), without a trailing
/// comment part (`' …`).
fn split_statements(line: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let (mut start, mut quoted) = (0, false);
    for (i, c) in line.char_indices() {
        match c {
            '"' => quoted = !quoted,
            ':' if !quoted => {
                parts.push(&line[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&line[start..]);
    parts.retain(|p| !p.trim().is_empty() && !p.trim_start().starts_with('\''));
    parts
}

/// `$RESOURCE NAME AS "file"` → (NAME, file).
fn parse_resource(line: &str) -> Option<(String, String)> {
    let rest = line.get("$RESOURCE".len()..)?.trim_start();
    let name: String = rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
    let after = rest[name.len()..].trim_start();
    if name.is_empty() || !after.get(..2).is_some_and(|a| a.eq_ignore_ascii_case("AS")) {
        return None;
    }
    let quoted = after[2..].trim_start().strip_prefix('"')?;
    let file = &quoted[..quoted.find('"')?];
    Some((name, file.to_string()))
}

fn parse_include_target(line: &str) -> Option<String> {
    let start = line.find(['"', '<'])?;
    let end_char = if line.as_bytes().get(start) == Some(&b'"') {
        '"'
    } else {
        '>'
    };
    let tail = &line[start + 1..];
    let end = tail.find(end_char)?;
    Some(tail[..end].to_string())
}

/// Finds an `$INCLUDE` file the way RapidQ on Windows does: `\\` separators
/// and case-insensitive names, looking next to the including file, then in
/// the include directories (and the RapidQ folder above them), then in the
/// current directory. Programs often name their author's absolute path
/// (`c:\\rapidq\\include\\windows.inc`); after the full path fails, ever
/// shorter endings of it are tried (`include/windows.inc`, `windows.inc`).
fn resolve_include_path(base_dir: &Path, include_file: &str, include_dirs: &[PathBuf]) -> Option<PathBuf> {
    let mut relative = include_file.trim().replace('\\', "/");
    if relative.len() >= 2 && relative.as_bytes()[1] == b':' && relative.as_bytes()[0].is_ascii_alphabetic() {
        relative = relative[2..].to_string();
    }
    let mut roots = vec![base_dir.to_path_buf()];
    for dir in include_dirs {
        roots.push(dir.clone());
    }
    for dir in include_dirs {
        if let Some(parent) = dir.parent() {
            roots.push(parent.to_path_buf());
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd);
    }
    if !relative.starts_with('/') {
        if let Some(found) = roots.iter().find_map(|root| find_case_insensitive(root, &relative)) {
            return Some(found);
        }
    }
    let parts: Vec<&str> = relative.split('/').filter(|p| !p.is_empty() && *p != "." && *p != "..").collect();
    (0..parts.len()).find_map(|skip| {
        let tail = parts[skip..].join("/");
        roots.iter().find_map(|root| find_case_insensitive(root, &tail))
    })
}

fn find_case_insensitive(root: &Path, relative: &str) -> Option<PathBuf> {
    let direct = root.join(relative);
    if direct.is_file() {
        return Some(direct);
    }
    let mut current = if relative.starts_with('/') { PathBuf::from("/") } else { root.to_path_buf() };
    for part in relative.split('/').filter(|part| !part.is_empty() && *part != ".") {
        if part == ".." {
            current.push("..");
            continue;
        }
        let exact = current.join(part);
        if exact.exists() {
            current = exact;
            continue;
        }
        let entry = fs::read_dir(&current).ok()?.flatten().find(|entry| {
            entry.file_name().to_str().is_some_and(|name| name.eq_ignore_ascii_case(part))
        })?;
        current = entry.path();
    }
    current.is_file().then_some(current)
}

/// At most this many expansions of one macro on one line: a macro whose
/// body calls itself (`$MACRO F(x) = F(x)`) would otherwise never end.
const MAX_MACRO_EXPANSIONS: usize = 10_000;

fn expand_macro(line: &mut MappedText, name: &str, definition: &MacroDefinition) {
    // (a `$MACRO` without a name matches nothing)
    if name.is_empty() || !line.text.contains(name) {
        return;
    }

    match &definition.params {
        Some(params) => expand_parameterized_macro(line, name, params, &definition.body),
        None => {
            for at in identifier_occurrences(&line.text, name).into_iter().rev() {
                line.replace(at, &definition.body);
            }
        }
    }
}

fn expand_parameterized_macro(result: &mut MappedText, name: &str, params: &[String], body: &str) {
    for _ in 0..MAX_MACRO_EXPANSIONS {
        let Some(call_start) = find_identifier_call(&result.text, name) else {
            break;
        };

        let open_index = call_start + name.len();
        let Some(close_index) = find_matching_paren(&result.text, open_index) else {
            break;
        };

        let args_text = &result.text[open_index + 1..close_index];
        let args = split_macro_args(args_text);
        let mut expanded = body.to_string();
        for (param, arg) in params.iter().zip(args.iter()) {
            expanded = expanded.replace(param, arg);
        }

        result.replace(call_start..close_index + 1, &expanded);
    }
}

fn find_identifier_call(line: &str, name: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let name_bytes = name.as_bytes();

    for start in 0..=bytes.len().saturating_sub(name_bytes.len() + 1) {
        if &bytes[start..start + name_bytes.len()] != name_bytes {
            continue;
        }
        let left_ok = start == 0 || !is_identifier_byte(bytes[start - 1]);
        let paren_index = start + name_bytes.len();
        let right_ok = bytes.get(paren_index) == Some(&b'(');
        if left_ok && right_ok {
            return Some(start);
        }
    }

    None
}

fn find_matching_paren(line: &str, open_index: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, ch) in line[open_index..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open_index + offset);
                }
            }
            _ => {}
        }
    }
    None
}

fn split_macro_args(args: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;

    for ch in args.chars() {
        match ch {
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            ',' if depth == 0 => {
                parts.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() || args.is_empty() {
        parts.push(current.trim().to_string());
    }

    parts
}

/// `$DEFINE`d symbols → their values, outside string literals (each part
/// between `"`s of the line as written, longest symbols first).
fn substitute_defines_outside_strings(line: &mut MappedText, defines: &HashMap<String, String>) {
    let mut sorted = defines.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| right.0.len().cmp(&left.0.len()));

    // The parts outside quotes, as (start, end) of the line as written;
    // done last to first so the earlier ones keep their offsets.
    let mut parts = Vec::new();
    let mut start = 0;
    for (i, part) in line.text.split('"').enumerate() {
        if i % 2 == 0 {
            parts.push((start, start + part.len()));
        }
        start += part.len() + 1;
    }
    for (part_start, part_end) in parts.into_iter().rev() {
        let mut part_end = part_end;
        for (symbol, value) in &sorted {
            let found = identifier_occurrences(&line.text[part_start..part_end], symbol);
            for at in found.iter().rev() {
                line.replace(part_start + at.start..part_start + at.end, value);
            }
            part_end = part_end + found.len() * value.len() - found.len() * symbol.len();
        }
    }
}

/// Where `symbol` stands as a whole identifier in `line` (left to right,
/// a rejected match's bytes are skipped).
fn identifier_occurrences(line: &str, symbol: &str) -> Vec<std::ops::Range<usize>> {
    let mut found = Vec::new();
    if symbol.is_empty() {
        return found;
    }
    let mut cursor = 0usize;
    while let Some(relative) = line[cursor..].find(symbol) {
        let start = cursor + relative;
        let end = start + symbol.len();
        let left_ok = start == 0 || !line[..start].chars().next_back().is_some_and(is_identifier_char);
        let right_ok = end >= line.len() || !line[end..].chars().next().is_some_and(is_identifier_char);
        if left_ok && right_ok {
            found.push(start..end);
        }
        cursor = end;
    }
    found
}

fn is_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{preprocess_file, preprocess_source, PreprocessOptions};

    fn preprocess(src: &str) -> String {
        preprocess_source(src, ".", None, PreprocessOptions::default())
            .unwrap()
            .source
    }

    #[test]
    fn shebang_first_line_is_left_out_lines_kept() {
        let result = preprocess("#!/usr/bin/env rapidr\nPRINT 1\n");
        assert_eq!(result.lines().next(), Some(""), "line 1 stays, empty");
        assert!(result.contains("PRINT 1"));
        // Only the first line: elsewhere `#!` is the program's.
        assert!(preprocess("PRINT 1\n#!x\n").contains("#!x"));
    }

    #[test]
    fn simple_define_substitution() {
        let result = preprocess("$DEFINE VERSION 2\nPRINT VERSION");
        assert!(result.contains("PRINT 2"));
    }

    #[test]
    fn define_without_value_sets_one() {
        let result = preprocess("$DEFINE DEBUG\n$IFDEF DEBUG\nPRINT \"Debug mode\"\n$ENDIF");
        assert!(result.contains("PRINT \"Debug mode\""));
    }

    #[test]
    fn undef_toggles_ifdef_branch() {
        let result = preprocess(
            "$DEFINE DEBUG\n$UNDEF DEBUG\n$IFDEF DEBUG\nPRINT \"Debug\"\n$ELSE\nPRINT \"Release\"\n$ENDIF",
        );
        assert!(result.contains("PRINT \"Release\""));
        assert!(!result.contains("PRINT \"Debug\""));
    }

    #[test]
    fn nested_conditionals_preserve_line_positions() {
        let result = preprocess("$DEFINE X\n$IFDEF X\nline3\n$ENDIF\nline5");
        let lines: Vec<&str> = result.split('\n').collect();
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[2].trim(), "line3");
        assert_eq!(lines[4].trim(), "line5");
    }

    #[test]
    fn parameterized_macro_expands() {
        let result = preprocess("$MACRO MAX(a,b) = IIF(a > b, a, b)\nx = MAX(10, 20)");
        assert!(result.contains("x = IIF(10 > 20, 10, 20)"));
    }

    #[test]
    fn resources_become_handles() {
        let result = preprocess_source("$RESOURCE GRID_BMP AS \"grid.bmp\" ' board\n$resource Snd as \"a.wav\"\nPRINT GRID_BMP", ".", None, PreprocessOptions::default()).unwrap();
        assert_eq!(result.source, "CONST GRID_BMP = 65536\nCONST Snd = 65537\nPRINT GRID_BMP");
        let names: Vec<_> = result.resources.iter().map(|r| (r.name.as_str(), r.file.as_str())).collect();
        assert_eq!(names, [("GRID_BMP", "grid.bmp"), ("Snd", "a.wav")]);
        // Several on a line.
        let result = preprocess_source("$resource T0 as \"a:b.bmp\" : $resource T1 as \"c.bmp\" ' tiles", ".", None, PreprocessOptions::default()).unwrap();
        assert_eq!(result.source, "CONST T0 = 65536 : CONST T1 = 65537");
    }

    #[test]
    fn directives_pass_through_when_expected() {
        let result = preprocess("$APPTYPE GUI\nPRINT \"test\"\n$OPTIMIZE ON");
        assert!(result.contains("$APPTYPE GUI"));
        assert!(result.contains("$OPTIMIZE ON"));
    }

    #[test]
    fn define_substitution_skips_strings() {
        let result = preprocess("$DEFINE GREETING Hello\nPRINT \"GREETING\"\nPRINT GREETING");
        assert!(result.contains("PRINT \"GREETING\""));
        assert!(result.contains("PRINT Hello"));
    }

    #[test]
    fn rapidq_inc_has_the_option_constants() {
        // RapidQ's own values (its RAPIDQ.INC): styles, grid options,
        // alignment, cursors; clGreen and clPurple are RapidQ's, not Delphi's.
        let names = [
            ("lbOwnerDrawVariable", 2), ("csOwnerDrawFixed", 3), ("gcsList", 0), ("goRangeSelect", 4), ("goColSizing", 7),
            ("taCenter", 2), ("taRightJustify", 1), ("crHandPoint", -21), ("ssShift", 256), ("vsReport", 3), ("clGreen", 0x00FF00),
            ("clPurple", 0xFF00FF),
        ];
        for (name, value) in names {
            let found = crate::RAPIDQ_INC_CONSTANTS.iter().find(|(n, _)| n.eq_ignore_ascii_case(name));
            assert_eq!(found.map(|(_, v)| *v), Some(value), "{name}");
        }
        // No name twice (the first would silently win).
        let mut seen = std::collections::HashSet::new();
        for (n, _) in crate::RAPIDQ_INC_CONSTANTS {
            assert!(seen.insert(n.to_ascii_lowercase()), "{n} is defined twice");
        }
    }

    #[test]
    fn rapidq_inc_names_and_values_are_pinned() {
        // The table was regrouped by source (docs/legal/rapidq-review.md);
        // what programs get must not change: the same 483 names with the
        // same numbers (FNV-1a over "name=value\n", sorted by name).
        let mut pairs: Vec<_> = crate::RAPIDQ_INC_CONSTANTS.to_vec();
        pairs.sort_by(|a, b| a.0.cmp(b.0));
        let text: String = pairs.iter().map(|(n, v)| format!("{n}={v}\n")).collect();
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in text.bytes() {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
        }
        assert_eq!((pairs.len(), hash), (483, 0x3f4b_730b_4fbe_3807));
    }

    #[test]
    fn escapechars_belongs_to_its_file() {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("rapidr-escape-{unique}"));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("lib.inc"), "p$ = \"\\\"\n").unwrap();
        let main = root.join("main.rr");
        fs::write(&main, "$ESCAPECHARS ON\n$INCLUDE \"lib.inc\"\nPRINT \"a\\tb\"\n").unwrap();
        let result = preprocess_file(&main, PreprocessOptions::default()).unwrap();
        let lines: Vec<&str> = result.source.lines().collect();
        let off = lines.iter().position(|l| l.trim() == "$ESCAPECHARS OFF").expect("off before the include");
        let on = lines.iter().rposition(|l| l.trim() == "$ESCAPECHARS ON").expect("on again after it");
        let include = lines.iter().position(|l| l.contains("p$ =")).unwrap();
        assert!(off < include && include < on, "{lines:?}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn include_reads_nested_file() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("rapidr-preprocess-{unique}"));
        fs::create_dir_all(&root).unwrap();
        let include = root.join("shared.inc");
        let main = root.join("main.rr");
        fs::write(&include, "PRINT 123").unwrap();
        fs::write(&main, "$INCLUDE \"shared.inc\"").unwrap();

        let result = preprocess_file(&main, PreprocessOptions::default()).unwrap();
        assert!(result.source.contains("PRINT 123"));

        let _ = fs::remove_file(include);
        let _ = fs::remove_file(main);
        let _ = fs::remove_dir(root);
    }

    #[test]
    fn vb_conditional_compilation() {
        let src = "#Const DEBUG = 1\n#If False Then\nskipped1\n#If True Then\nskipped2\n#End If\n#ElseIf DEBUG Then\nkept1\n#Else\nskipped3\n#End If\n#If Not DEBUG Then\nskipped4\n#Else\nkept2\n#End If\nafter";
        let result = preprocess_source(src, ".", None, PreprocessOptions::default()).unwrap();
        assert!(!result.source.contains("skipped"), "{}", result.source);
        assert!(result.source.contains("kept1") && result.source.contains("kept2") && result.source.contains("after"));
        assert_eq!(result.source.lines().count(), src.lines().count(), "line numbers are preserved");
    }

    #[test]
    fn rapidq_style_includes_share_defines_and_decode_ansi() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("rapidr-preprocess-rq-{unique}"));
        let lib = root.join("Lib");
        let extra = root.join("extra");
        fs::create_dir_all(&lib).unwrap();
        fs::create_dir_all(&extra).unwrap();
        // Windows path, different case, guarded with $IFNDEF like RapidQ includes.
        fs::write(lib.join("Guarded.INC"), "$IFNDEF __GUARD\n$DEFINE __GUARD\n$DEFINE LIMIT 10\n$ENDIF").unwrap();
        fs::write(extra.join("fromdir.inc"), "PRINT \"found in include dir\"").unwrap();
        let main = root.join("main.bas");
        // "caf\xE9" is Windows-1252, not UTF-8.
        let mut source = b"$INCLUDE \"lib\\guarded.inc\"\n$INCLUDE \"LIB\\GUARDED.inc\"\n$INCLUDE \"fromdir.inc\"\n$IFDEF __guard\nPRINT LIMIT\n$ENDIF\n$IFDEF win32\nPRINT \"caf".to_vec();
        source.extend_from_slice(b"\xE9\"\n$ENDIF\n");
        fs::write(&main, source).unwrap();

        let options = PreprocessOptions { include_dirs: vec![extra.clone()], ..Default::default() };
        let result = preprocess_file(&main, options).unwrap();
        assert!(result.source.contains("PRINT 10"), "{}", result.source);
        assert!(result.source.contains("found in include dir"));
        assert!(result.source.contains("PRINT \"café\""), "{}", result.source);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn virtual_files_are_included_without_a_file_system() {
        let options = PreprocessOptions {
            virtual_files: vec![("util.inc".into(), "$INCLUDE \"inner.INC\"\nSUB Hi\nEND SUB".into()), ("lib/Inner.inc".into(), "PRINT 1".into())],
            ..Default::default()
        };
        let result = preprocess_source("$INCLUDE \"C:\\src\\UTIL.inc\"\nHi\n", ".", Some(std::path::PathBuf::from("main.bas")), options).unwrap();
        assert_eq!(result.source, "PRINT 1\nSUB Hi\nEND SUB\nHi\n");
        let origins: Vec<(String, usize)> = result.line_map.iter().map(|(f, l)| (f.as_ref().unwrap().display().to_string(), *l)).collect();
        assert_eq!(origins, [("lib/Inner.inc".to_string(), 1), ("util.inc".into(), 2), ("util.inc".into(), 3), ("main.bas".into(), 2), ("main.bas".into(), 3)]);
    }

    #[test]
    fn rapidq_inc_is_built_in_and_keeps_line_numbers() {
        let result = preprocess("$INCLUDE \"RAPIDQ.INC\"\nPRINT clBlue\n");
        let lines: Vec<&str> = result.lines().collect();
        assert!(lines[0].contains("CONST clBlue = 16711680"), "{}", lines[0]);
        assert_eq!(lines[1], "PRINT clBlue");
    }

    #[test]
    fn options_seed_initial_defines() {
        let mut defines = HashMap::new();
        defines.insert("DEBUG".to_string(), "1".to_string());
        let result = preprocess_source(
            "$IFDEF DEBUG\nPRINT 1\n$ENDIF",
            ".",
            None,
            PreprocessOptions { defines, ..Default::default() },
        )
        .unwrap();
        assert!(result.source.contains("PRINT 1"));
    }

    /// Every byte of the output is mapped, in order, and exact segments are
    /// the file's own bytes.
    fn check_origin_map(result: &super::PreprocessResult) {
        let map = &result.origins;
        let mut at = 0;
        for s in &map.segments {
            assert_eq!(s.pp_start, at, "segments are contiguous: {:?}", s);
            assert!(s.pp_end > s.pp_start);
            if s.exact {
                let file = &map.files[s.file].text;
                assert_eq!(&result.source[s.pp_start..s.pp_end], &file[s.src_start..s.src_end]);
            }
            at = s.pp_end;
        }
        assert_eq!(at, result.source.len(), "every byte is mapped");
    }

    #[test]
    fn origin_map_points_into_included_files() {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("rapidr-origins-{unique}"));
        fs::create_dir_all(&root).unwrap();
        let inc = "' lib\r\nSUB Hello\r\n  PRINT \"hi\"\r\nEND SUB\r\n";
        fs::write(root.join("lib.inc"), inc).unwrap();
        let main_text = "$DEFINE N 42\n$INCLUDE \"lib.inc\"\nx = N + 1\nHello\n$MACRO SQ(a) = ((a)*(a))\ny = SQ(x) : z = 7\n";
        let main = root.join("main.bas");
        fs::write(&main, main_text).unwrap();
        let result = preprocess_file(&main, PreprocessOptions::default()).unwrap();
        check_origin_map(&result);
        let map = &result.origins;
        let pos = |needle: &str| result.source.find(needle).unwrap();
        // Code from the include: its file, its byte.
        let o = map.origin(pos("PRINT \"hi\"")).unwrap();
        assert_eq!(map.files[o.file].path.as_deref(), Some(root.join("lib.inc").as_path()));
        assert_eq!(o.offset, inc.find("PRINT").unwrap());
        assert!(o.exact);
        // The program's own code after the include.
        let o = map.origin(pos("Hello\n")).unwrap();
        assert_eq!((map.files[o.file].path.as_deref(), o.offset), (Some(main.as_path()), main_text.find("Hello\n").unwrap()));
        // A $DEFINE's value maps to the name it replaced.
        let at = pos("x = 42");
        let o = map.origin(at + 4).unwrap();
        assert_eq!((o.offset, o.exact), (main_text.find("N + 1").unwrap(), false));
        let span = map.origin_span(at, at + "x = 42 + 1".len()).unwrap();
        assert_eq!(&main_text[span.start..span.end], "x = N + 1");
        // A macro call: the expansion maps to the call; text after it is exact.
        let at = pos("((x)*(x))");
        let span = map.origin_span(at, at + "((x)*(x))".len()).unwrap();
        assert_eq!(&main_text[span.start..span.end], "SQ(x)");
        let o = map.origin(pos("z = 7")).unwrap();
        assert_eq!((o.offset, o.exact), (main_text.find("z = 7").unwrap(), true));
        // Back from a file position to the preprocessed text.
        let file = map.find_file(&root.join("lib.inc")).unwrap();
        assert_eq!(map.to_preprocessed(file, inc.find("PRINT").unwrap()), Some(pos("PRINT \"hi\"")));
        // What each line was.
        let main_id = map.find_file(&main).unwrap();
        use super::LineKind::*;
        assert_eq!(map.files[main_id].lines, [Directive, Directive, Code, Code, Directive, Code, Code]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn line_kinds_of_one_file() {
        use super::LineKind::*;
        let kinds = super::scan_lines("#!/usr/bin/env rapidr\n$IFDEF NOPE\nskipped\n$ELSE\nkept\n$ENDIF\n$INCLUDE \"missing.inc\"\n$RESOURCE bad\nPRINT 1", PreprocessOptions::default());
        assert_eq!(kinds, [Shebang, Directive, Inactive, Directive, Code, Directive, Directive, Directive, Code]);
    }

    #[test]
    fn origin_map_of_builtin_includes_and_resources() {
        let src = "$INCLUDE \"RAPIDQ.INC\"\n$RESOURCE B AS \"b.bmp\"\nPRINT clBlue, B\n";
        let result = preprocess_source(src, ".", None, PreprocessOptions::default()).unwrap();
        check_origin_map(&result);
        let o = result.origins.origin(result.source.find("CONST clBlue").unwrap()).unwrap();
        assert_eq!((o.offset, o.exact), (0, false));
        let o = result.origins.origin(result.source.find("PRINT clBlue").unwrap()).unwrap();
        assert_eq!((o.offset, o.exact), (src.find("PRINT").unwrap(), true));
    }

    #[test]
    fn encodings_round_trip() {
        for bytes in [&b"PRINT \"caf\xC3\xA9\"\r\n"[..], b"\xEF\xBB\xBFPRINT 1", b"PRINT \"caf\xE9\x80\x81\x9D\"", b"\xEF\xBB\xBFx\xFF"] {
            let (text, encoding) = super::decode_source(bytes);
            assert_eq!(super::encode_source(&text, encoding), bytes, "{encoding:?}");
        }
    }
}