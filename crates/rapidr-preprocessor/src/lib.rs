use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use rapidr_diagnostics::{Diagnostic, SourceLocation, TextSpan};

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
}

/// State shared by a file and everything it includes: a `$DEFINE` or `$MACRO`
/// in an include file is visible to the code after the `$INCLUDE`.
struct PpState {
    defines: HashMap<String, String>,
    macros: HashMap<String, MacroDefinition>,
    include_stack: Vec<PathBuf>,
    include_dirs: Vec<PathBuf>,
    app_type: Option<String>,
    resources: Vec<Resource>,
    /// `$ESCAPECHARS ON` is in effect (it belongs to the file it's in: an
    /// include file starts with it off, and the includer's setting comes back
    /// after the include).
    escape_chars: bool,
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
            app_type: None,
            resources: Vec::new(),
            escape_chars: false,
        }
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
    let bytes = fs::read(path)?;
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
    Ok(match std::str::from_utf8(bytes) {
        Ok(text) => text.to_string(),
        Err(_) => bytes.iter().map(|&b| windows_1252_char(b)).collect(),
    })
}

fn windows_1252_char(byte: u8) -> char {
    const HIGH: [char; 32] = [
        '\u{20AC}', '\u{81}', '\u{201A}', '\u{192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
        '\u{2C6}', '\u{2030}', '\u{160}', '\u{2039}', '\u{152}', '\u{8D}', '\u{17D}', '\u{8F}',
        '\u{90}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
        '\u{2DC}', '\u{2122}', '\u{161}', '\u{203A}', '\u{153}', '\u{9D}', '\u{17E}', '\u{178}',
    ];
    match byte {
        0x80..=0x9F => HIGH[(byte - 0x80) as usize],
        _ => byte as char,
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
    let source = read_source(path).map_err(|error| {
        PreprocessError::new(
            format!("Failed to read source file '{}': {error}", path.display()),
            1,
            1,
            Some(path.display().to_string()),
        )
    })?;

    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut state = PpState::new(options);
    let (source, line_map) = preprocess_with_state(&source, base_dir, Some(path.to_path_buf()), &mut state)?;
    Ok(PreprocessResult { source, line_map, app_type: state.app_type, resources: state.resources })
}

pub fn preprocess_source(
    source: &str,
    base_dir: impl AsRef<Path>,
    file_path: Option<PathBuf>,
    options: PreprocessOptions,
) -> Result<PreprocessResult, PreprocessError> {
    let mut state = PpState::new(options);
    let (source, line_map) = preprocess_with_state(source, base_dir.as_ref(), file_path, &mut state)?;
    Ok(PreprocessResult { source, line_map, app_type: state.app_type, resources: state.resources })
}

fn preprocess_with_state(
    source: &str,
    base_dir: &Path,
    file_path: Option<PathBuf>,
    state: &mut PpState,
) -> Result<(String, Vec<LineOrigin>), PreprocessError> {
    let mut output_lines = Vec::new();
    let mut origins = Vec::new();
    let mut skip_stack: Vec<bool> = Vec::new();
    // One entry per open `#If`: whether one of its branches was taken.
    let mut vb_taken: Vec<bool> = Vec::new();
    let file_label = file_path.as_ref().map(|path| path.display().to_string());

    for (line_index, original_line) in source.split('\n').enumerate() {
        let line_number = line_index + 1;
        let line = original_line.trim();
        let upper_line = line.to_ascii_uppercase();

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
            emit_line(&mut output_lines, &mut origins, &file_path, line_number, String::new());
            continue;
        }

        // VB conditional compilation: `#If False Then`, `#If DEBUG Then`,
        // `#ElseIf`, `#Else`, `#End If`, `#Const NAME = value`.
        if let Some(rest) = upper_line.strip_prefix("#IF ") {
            let condition = rest.trim().strip_suffix("THEN").unwrap_or(rest).trim();
            let parent_skip = skip_stack.last().copied().unwrap_or(false);
            skip_stack.push(parent_skip || !state.vb_condition(condition));
            vb_taken.push(!skip_stack.last().copied().unwrap_or(true));
            emit_line(&mut output_lines, &mut origins, &file_path, line_number, String::new());
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
            emit_line(&mut output_lines, &mut origins, &file_path, line_number, String::new());
            continue;
        }
        if upper_line == "#ELSE" || upper_line.starts_with("#ELSE ") || upper_line.starts_with("#ELSE'") {
            if let Some(taken) = vb_taken.last().copied() {
                let parent_skip = skip_stack.len() > 1 && skip_stack[skip_stack.len() - 2];
                let last = skip_stack.len() - 1;
                skip_stack[last] = parent_skip || taken;
            }
            emit_line(&mut output_lines, &mut origins, &file_path, line_number, String::new());
            continue;
        }
        if upper_line.starts_with("#END IF") || upper_line.starts_with("#ENDIF") {
            if vb_taken.pop().is_some() {
                skip_stack.pop();
            }
            emit_line(&mut output_lines, &mut origins, &file_path, line_number, String::new());
            continue;
        }
        if let Some(rest) = upper_line.strip_prefix("#CONST ") {
            if !skip_stack.last().copied().unwrap_or(false) {
                if let Some((name, value)) = line["#CONST ".len()..].split_once('=') {
                    let _ = rest;
                    state.defines.insert(name.trim().to_string(), strip_inline_comment(value).trim().to_string());
                }
            }
            emit_line(&mut output_lines, &mut origins, &file_path, line_number, String::new());
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
            emit_line(&mut output_lines, &mut origins, &file_path, line_number, String::new());
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
            emit_line(&mut output_lines, &mut origins, &file_path, line_number, String::new());
            continue;
        }

        if upper_line.starts_with("$ENDIF") {
            if !skip_stack.is_empty() {
                skip_stack.pop();
            }
            emit_line(&mut output_lines, &mut origins, &file_path, line_number, String::new());
            continue;
        }

        if skip_stack.last().copied().unwrap_or(false) {
            emit_line(&mut output_lines, &mut origins, &file_path, line_number, String::new());
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
            emit_line(&mut output_lines, &mut origins, &file_path, line_number, String::new());
            continue;
        }

        if upper_line.starts_with("$UNDEF") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                state.defines.retain(|key, _| !key.eq_ignore_ascii_case(parts[1]));
            }
            emit_line(&mut output_lines, &mut origins, &file_path, line_number, String::new());
            continue;
        }

        if upper_line.starts_with("$MACRO") {
            if let Some((name, definition)) = parse_macro_definition(line) {
                state.macros.insert(name, definition);
            }
            emit_line(&mut output_lines, &mut origins, &file_path, line_number, String::new());
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
            emit_line(&mut output_lines, &mut origins, &file_path, line_number, original_line.to_string());
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
            emit_line(&mut output_lines, &mut origins, &file_path, line_number, text);
            continue;
        }

        // `$RESOURCE NAME AS "file"` (several may share a line, separated
        // by `:`): `NAME` is the resource's handle.
        if upper_line.starts_with("$RESOURCE") {
            let mut consts = Vec::new();
            for part in split_statements(line) {
                let (name, file) = parse_resource(part.trim()).ok_or_else(|| {
                    PreprocessError::new(format!("Invalid $RESOURCE syntax: {}", part.trim()), line_number, 1, file_label.clone())
                })?;
                let handle = RESOURCE_BASE + state.resources.len() as i64;
                let path = resolve_include_path(base_dir, &file, &[]);
                state.resources.push(Resource { name: name.clone(), file, path, optional: false });
                consts.push(format!("CONST {name} = {handle}"));
            }
            emit_line(&mut output_lines, &mut origins, &file_path, line_number, consts.join(" : "));
            continue;
        }

        if upper_line.starts_with("$INCLUDE") {
            let include_file = parse_include_target(line).ok_or_else(|| {
                PreprocessError::new(
                    format!("Invalid $INCLUDE syntax: {line}"),
                    line_number,
                    1,
                    file_label.clone(),
                )
            })?;

            let include_path = match resolve_include_path(base_dir, &include_file, &state.include_dirs) {
                Some(path) => path,
                None => {
                    // RapidQ programs start with `$INCLUDE "RAPIDQ.INC"`; supply
                    // its constants when the file isn't next to the program.
                    if let Some(builtin) = builtin_include(&include_file) {
                        emit_line(&mut output_lines, &mut origins, &file_path, line_number, builtin);
                        continue;
                    }
                    return Err(PreprocessError::new(
                        format!("Include file not found: '{include_file}'"),
                        line_number,
                        1,
                        file_label.clone(),
                    ));
                }
            };

            if state.include_stack.iter().any(|entry| entry == &include_path) {
                return Err(PreprocessError::new(
                    format!("Recursive include detected: '{include_file}'"),
                    line_number,
                    1,
                    file_label.clone(),
                ));
            }

            let include_source = read_source(&include_path).map_err(|error| {
                PreprocessError::new(
                    format!("Failed to include '{include_file}': {error}"),
                    line_number,
                    1,
                    file_label.clone(),
                )
            })?;

            // The include file has its own $ESCAPECHARS: off to begin with.
            let escape_before = state.escape_chars;
            if escape_before {
                emit_line(&mut output_lines, &mut origins, &file_path, line_number, "$ESCAPECHARS OFF".to_string());
                state.escape_chars = false;
            }
            state.include_stack.push(include_path.clone());
            let nested = preprocess_with_state(
                &include_source,
                include_path.parent().unwrap_or_else(|| Path::new(".")),
                Some(include_path.clone()),
                state,
            );
            state.include_stack.pop();
            let (nested_source, nested_origins) = nested?;
            output_lines.push(nested_source);
            origins.extend(nested_origins);
            // …and the includer's comes back.
            if state.escape_chars != escape_before {
                let text = if escape_before { "$ESCAPECHARS ON" } else { "$ESCAPECHARS OFF" };
                emit_line(&mut output_lines, &mut origins, &file_path, line_number, text.to_string());
                state.escape_chars = escape_before;
            }
            continue;
        }

        let mut processed_line = original_line.to_string();

        if !state.macros.is_empty() {
            for (name, definition) in &state.macros {
                processed_line = expand_macro(&processed_line, name, definition);
            }
        }

        if !state.defines.is_empty() && !upper_line.contains('$') {
            processed_line = substitute_defines_outside_strings(&processed_line, &state.defines);
        }

        emit_line(&mut output_lines, &mut origins, &file_path, line_number, processed_line);
    }

    Ok((output_lines.join("\n"), origins))
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

fn emit_line(
    lines: &mut Vec<String>,
    origins: &mut Vec<LineOrigin>,
    file: &Option<PathBuf>,
    line: usize,
    text: String,
) {
    lines.push(text);
    origins.push((file.clone(), line));
}

/// Constants from RapidQ's RAPIDQ.INC (Delphi/Win32 values; colors are BGR
/// like RapidQ's, which RapidR's runtimes also use). System colors have no
/// OS lookup in RapidR, so they get their standard default RGB values.
pub const RAPIDQ_INC_CONSTANTS: &[(&str, i64)] = &[
    // As in the real RAPIDQ.INC (comparisons themselves give -1 / 0).
    ("False", 0), ("True", 1),
    // PLAYWAV options
    ("SND_SYNC", 0), ("SND_ASYNC", 1), ("SND_LOOP", 8),
    // Colors (&HBBGGRR)
    ("clBlack", 0x000000), ("clMaroon", 0x000080), ("clGreen", 0x00FF00),
    ("clOlive", 0x008080), ("clNavy", 0x800000), ("clPurple", 0xFF00FF),
    ("clTeal", 0x808000), ("clGray", 0x808080), ("clSilver", 0xC0C0C0),
    ("clRed", 0x0000FF), ("clLime", 0x00FF00), ("clYellow", 0x00FFFF),
    ("clBlue", 0xFF0000), ("clFuchsia", 0xFF00FF), ("clAqua", 0xFFFF00),
    ("clWhite", 0xFFFFFF), ("clLtGray", 0xC0C0C0), ("clDkGray", 0x808080),
    ("clBtnFace", 0xF0F0F0), ("clWindow", 0xFFFFFF), ("clWindowText", 0x000000),
    ("clBtnText", 0x000000), ("clBtnShadow", 0xA0A0A0), ("clHighlight", 0xD77800),
    ("clHighlightText", 0xFFFFFF), ("clGrayText", 0x6D6D6D),
    // Modal results
    ("mrNone", 0), ("mrOk", 1), ("mrCancel", 2), ("mrAbort", 3), ("mrRetry", 4),
    ("mrIgnore", 5), ("mrYes", 6), ("mrNo", 7), ("mrAll", 8),
    // Message boxes
    ("MB_OK", 0), ("MB_OKCANCEL", 1), ("MB_ABORTRETRYIGNORE", 2), ("MB_YESNOCANCEL", 3),
    ("MB_YESNO", 4), ("MB_RETRYCANCEL", 5), ("MB_ICONHAND", 16), ("MB_ICONSTOP", 16),
    ("MB_ICONERROR", 16), ("MB_ICONQUESTION", 32), ("MB_ICONEXCLAMATION", 48),
    ("MB_ICONWARNING", 48), ("MB_ICONASTERISK", 64), ("MB_ICONINFORMATION", 64),
    ("IDOK", 1), ("IDCANCEL", 2), ("IDABORT", 3), ("IDRETRY", 4), ("IDIGNORE", 5),
    ("IDYES", 6), ("IDNO", 7),
    // Form border styles, window states, alignment
    ("bsNone", 0), ("bsSingle", 1), ("bsSizeable", 2), ("bsDialog", 3),
    ("bsToolWindow", 4), ("bsSizeToolWin", 5),
    ("wsNormal", 0), ("wsMinimized", 1), ("wsMaximized", 2),
    ("alNone", 0), ("alTop", 1), ("alBottom", 2), ("alLeft", 3), ("alRight", 4), ("alClient", 5),
    // Mouse buttons
    ("mbLeft", 0), ("mbRight", 1), ("mbMiddle", 2),
    // MessageDlg types and buttons
    ("mtWarning", 0), ("mtError", 1), ("mtInformation", 2), ("mtConfirmation", 3), ("mtCustom", 4),
    ("mbYes", 1), ("mbNo", 2), ("mbOK", 4), ("mbCancel", 8), ("mbHelp", 16), ("mbAbort", 32),
    ("mbRetry", 64), ("mbIgnore", 128), ("mbAll", 256),
    // File stream modes
    ("fmCreate", 0xFFFF), ("fmOpenRead", 0), ("fmOpenWrite", 1), ("fmOpenReadWrite", 2),
    // Stream seeking and number types (ReadNum/WriteNum)
    ("soFromBeginning", 0), ("soFromCurrent", 1), ("soFromEnd", 2),
    ("Num_BYTE", 1), ("Num_SHORT", 2), ("Num_WORD", 3), ("Num_LONG", 4), ("Num_DWORD", 5),
    ("Num_SINGLE", 6), ("Num_DOUBLE", 8),
    // Font styles (AddStyles/DelStyles), special colors, bitmap formats
    ("fsBold", 0), ("fsItalic", 1), ("fsUnderline", 2), ("fsStrikeOut", 3),
    ("clNone", 536870911), ("clDefault", 536870912),
    ("pfDevice", 0), ("pf1bit", 1), ("pf4bit", 2), ("pf8bit", 3), ("pf15bit", 4),
    ("pf16bit", 5), ("pf24bit", 6), ("pf32bit", 7),
    // Virtual key codes
    ("VK_BACK", 8), ("VK_TAB", 9), ("VK_RETURN", 13), ("VK_SHIFT", 16), ("VK_CONTROL", 17),
    ("VK_MENU", 18), ("VK_PAUSE", 19), ("VK_ESCAPE", 27), ("VK_SPACE", 32),
    ("VK_PRIOR", 33), ("VK_NEXT", 34), ("VK_END", 35), ("VK_HOME", 36),
    ("VK_LEFT", 37), ("VK_UP", 38), ("VK_RIGHT", 39), ("VK_DOWN", 40),
    ("VK_INSERT", 45), ("VK_DELETE", 46),
    ("VK_F1", 112), ("VK_F2", 113), ("VK_F3", 114), ("VK_F4", 115), ("VK_F5", 116),
    ("VK_F6", 117), ("VK_F7", 118), ("VK_F8", 119), ("VK_F9", 120), ("VK_F10", 121),
    ("VK_F11", 122), ("VK_F12", 123),
    // The rest of RapidQ's RAPIDQ.INC (ENUM-like option numbers: alignment,
    // styles, grid options, cursors, …). The system colors have no OS lookup
    // in RapidR: the usual Windows defaults.
    ("taLeftJustify", 0), ("taRightJustify", 1), ("taCenter", 2), ("SND_NODEFAULT", 2), ("SND_NOSTOP", 16),
    ("SND_MEMORY", 4), ("clScrollBar", 0xc8c8c8), ("clBackGround", 0x000000), ("clActiveCaption", 0xd1b499), ("clInActiveCaption", 0xdbcdbf),
    ("clMenu", 0xf0f0f0), ("clWindowFrame", 0x646464), ("clMenuText", 0x000000), ("clCaptionText", 0x000000), ("clActiveBorder", 0xb4b4b4),
    ("clInActiveBorder", 0xfcf7f4), ("clAppWorkSpace", 0xababab), ("clHilight", 0xd77800), ("clHilightText", 0xffffff), ("clInActiveCaptionText", 0x000000),
    ("clBtnHighlight", 0xffffff), ("cl3DDkShadow", 0x696969), ("cl3DLight", 0xe3e3e3), ("clInfoText", 0x000000), ("clInfoBk3DDkShadow", 0xe1ffff),
    ("ssShift", 256), ("ssCtrl", 16), ("ssAlt", 1), ("fpDefault", 0), ("fpVariable", 1),
    ("fpFixed", 2), ("ANSI_CHARSET", 0), ("DEFAULT_CHARSET", 1), ("SYMBOL_CHARSET", 2), ("MAC_CHARSET", 77),
    ("SHIFTJIS_CHARSET", 128), ("HANGEUL_CHARSET", 129), ("JOHAB_CHARSET", 130), ("GB2312_CHARSET", 134), ("CHINESEBIG5_CHARSET", 136),
    ("GREEK_CHARSET", 161), ("TURKISH_CHARSET", 162), ("VIETNAMESE_CHARSET", 163), ("HEBREW_CHARSET", 177), ("ARABIC_CHARSET", 178),
    ("BALTIC_CHARSET", 186), ("RUSSIAN_CHARSET", 204), ("THAI_CHARSET", 222), ("EASTEUROPE_CHARSET", 238), ("OEM_CHARSET", 255),
    ("fsNormal", 0), ("fsMDIChild", 1), ("fsMDIForm", 2), ("fsStayOnTop", 3), ("CtrlDown", 1),
    ("AltDown", 16), ("ShiftDown", 256), ("biSystemMenu", 0), ("biMinimize", 1), ("biMaximize", 2),
    ("biHelp", 3), ("caNone", 0), ("caHide", 1), ("caFree", 2), ("caMinimize", 3),
    ("tlTop", 0), ("tlCenter", 1), ("tlBottom", 2), ("lsNone", 0), ("lsRaised", 1),
    ("lsRecessed", 2), ("bvNone", 0), ("bvLowered", 1), ("bvRaised", 2), ("bpNone", 0),
    ("bpSingle", 1), ("ecNormal", 0), ("ecUpperCase", 1), ("ecLowerCase", 2), ("csDropDown", 0),
    ("csSimple", 1), ("csDropDownList", 2), ("csOwnerDrawFixed", 3), ("csOwnerDrawVariable", 4), ("ssNone", 0),
    ("ssHorizontal", 1), ("ssVertical", 2), ("ssBoth", 3), ("mrNoToAll", 9), ("mrYesToAll", 10),
    ("blBMPLeft", 0), ("blBMPRight", 1), ("blBMPTop", 2), ("blBMPBottom", 3), ("bkCustom", 0),
    ("bkOK", 1), ("bkCancel", 2), ("bkHelp", 3), ("bkYes", 4), ("bkNo", 5),
    ("bkClose", 6), ("bkAbort", 7), ("bkRetry", 8), ("bkIgnore", 9), ("bkAll", 10),
    ("crDefault", 0), ("crNone", -1), ("crArrow", -2), ("crCross", -3), ("crIBeam", -4),
    ("crSize", -5), ("crSizeNESW", -6), ("crSizeNS", -7), ("crSizeNWSE", -8), ("crSizeWE", -9),
    ("crUpArrow", -10), ("crHourGlass", -11), ("crDrag", -12), ("crNoDrop", -13), ("crHSplit", -14),
    ("crVSplit", -15), ("crMultiDrag", -16), ("crSQLWait", -17), ("crNo", -18), ("crAppStart", -19),
    ("crHelp", -20), ("crHandPoint", -21), ("ftReadOnly", 0), ("ftHidden", 1), ("ftSystem", 2),
    ("ftVolumeID", 3), ("ftDirectory", 4), ("ftArchive", 5), ("ftNormal", 6), ("sbHorizontal", 0),
    ("sbVertical", 1), ("scLineUp", 0), ("scLineDown", 1), ("scPageUp", 2), ("scPageDown", 3),
    ("scPosition", 4), ("scTrack", 5), ("scTop", 6), ("scBottom", 7), ("scEndScroll", 8),
    ("dsFocused", 0), ("dsSelected", 1), ("dsNormal", 2), ("dsTransparent", 3), ("itImage", 0),
    ("itMask", 1), ("stNone", 0), ("stText", 2), ("vsIcon", 0), ("vsSmallIcon", 1),
    ("vsList", 2), ("vsReport", 3), ("tbHorizontal", 0), ("tbVertical", 1), ("tmBottomRight", 0),
    ("tmTopLeft", 1), ("tmBoth", 2), ("tsNone", 0), ("tsAuto", 1), ("tsManual", 2),
    ("goFixedVertLine", 0), ("goFixedHorzLine", 1), ("goVertLine", 2), ("goHorzLine", 3), ("goRangeSelect", 4),
    ("goDrawFocusSelected", 5), ("goRowSizing", 6), ("goColSizing", 7), ("goRowMoving", 8), ("goColMoving", 9),
    ("goEditing", 10), ("goTabs", 11), ("goRowSelect", 12), ("goAlwaysShowEditor", 13), ("goThumbTracking", 14),
    ("gcsList", 0), ("gcsEllipsis", 1), ("gcsNone", 2), ("osText", 0), ("osPlusMinusText", 1),
    ("osPictureText", 2), ("osPlusMinusPictureText", 3), ("osTreeText", 4), ("osTreePictureText", 5), ("ooDrawTreeRoot", 0),
    ("ooDrawFocusRect", 1), ("ooDrawStretchBitmaps", 2), ("gkText", 0), ("gkHorizontalBar", 1), ("gkVerticalBar", 2),
    ("gkPie", 3), ("gkNeedle", 4), ("cmBlackness", 66), ("cmDstInvert", 5570569), ("cmMergeCopy", 12583114),
    ("cmMergePaint", 12255782), ("cmNotSrcCopy", 3342344), ("cmNotSrcErase", 1114278), ("cmPatCopy", 15728673), ("cmPatInvert", 5898313),
    ("cmPatPaint", 16452105), ("cmSrcAnd", 8913094), ("cmSrcCopy", 13369376), ("cmSrcErase", 4457256), ("cmSrcInvert", 6684742),
    ("cmSrcPaint", 15597702), ("cmWhiteness", 16711778), ("tmAuto", 0), ("tmFixed", 1), ("lbStandard", 0),
    ("lbOwnerDrawFixed", 1), ("lbOwnerDrawVariable", 2), ("br110", 0), ("br300", 1), ("br600", 2),
    ("br1200", 3), ("br2400", 4), ("br4800", 5), ("br9600", 6), ("br14400", 7),
    ("br19200", 8), ("br38400", 9), ("br56000", 10), ("br57600", 11), ("br115200", 12),
    ("sbOneStopBit", 0), ("sbOne5StopBits", 1), ("sbTwoStopBits", 2), ("prNone", 0), ("prOdd", 1),
    ("prEven", 2), ("prMark", 3), ("prSpace", 4), ("fdAnsiOnly", 0), ("fdTrueTypeOnly", 1),
    ("fdEffects", 2), ("fdFixedPitchOnly", 3), ("fdForceFontExist", 4), ("fdNoFaceSel", 5), ("fdNoOEMFonts", 6),
    ("fdNoSimulations", 7), ("fdNoSizeSel", 8), ("fdNoStyleSel", 9), ("fdNoVectorFonts", 10), ("fdShowHelp", 11),
    ("fdWysiwyg", 12), ("fdLimitSize", 13), ("fdScalableOnly", 14), ("fdApplyButton", 15), ("dtReadOnly", 0),
    ("dtHidden", 1), ("dtSystem", 2), ("dtNormal", 3), ("dtAll", 4), ("drtUnknown", 0),
    ("drtRemovable", 1), ("drtFixed", 2), ("drtRemote", 3), ("drtCDRom", 4), ("drtRamDisk", 5),
    ("IPPROTO_IP", 0), ("IPPROTO_ICMP", 1), ("IPPROTO_IGMP", 2), ("IPPROTO_TCP", 6), ("IPPROTO_PUP", 12),
    ("IPPROTO_UDP", 17), ("IPPROTO_IDP", 22), ("IPPROTO_RAW", 255), ("SOCK_STREAM", 1), ("SOCK_DGRAM", 2),
    ("SOCK_RAW", 3), ("SOCK_RDM", 4), ("SOCK_SEQPACKET", 5), ("AF_UNSPEC", 0), ("AF_UNIX", 1),
    ("AF_INET", 2), ("AF_IMPLINK", 3), ("AF_PUP", 4), ("AF_CHAOS", 5), ("AF_IPX", 6),
    ("AF_NS", 6), ("AF_ISO", 7), ("AF_ECMA", 8), ("AF_DATAKIT", 9), ("AF_CCITT", 10),
    ("AF_SNA", 11), ("AF_DECnet", 12), ("AF_DLI", 13), ("AF_LAT", 14), ("AF_HYLINK", 15),
    ("AF_APPLETALK", 16), ("AF_NETBIOS", 17), ("AF_VOICEVIEW", 18), ("AF_FIREFOX", 19), ("AF_UNKNOWN1", 20),
    ("AF_BAN", 21), ("hsText", 0), ("hsOwnerDraw", 1), ("dupIgnore", 0), ("dupAccept", 1),
    ("dupError", 2), ("smClip", 0), ("smCenter", 1), ("smScale", 2), ("smStretch", 3),
    ("smAutoSize", 4), ("osEmpty", 0), ("osLoaded", 1), ("osRunning", 2), ("osOpen", 3),
    ("osInPlaceActive", 4), ("osUIActive", 5), ("ffGeneral", 0), ("ffExponent", 1), ("ffFixed", 2),
    ("ffNumber", 3), ("faReadOnly", 1), ("faHidden", 2), ("faSysFile", 4), ("faVolumeID", 8),
    ("faDirectory", 16), ("faArchive", 32), ("faAnyFile", 63), ("poPortrait", 0), ("poLandscape", 1),
    ("caClose", 2),
];

/// Built-in replacement for an include file that isn't on disk, as a single
/// line (so line numbers after the $INCLUDE stay correct).
fn builtin_include(include_file: &str) -> Option<String> {
    let name = Path::new(include_file).file_name()?.to_str()?;
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

fn expand_macro(line: &str, name: &str, definition: &MacroDefinition) -> String {
    if !line.contains(name) {
        return line.to_string();
    }

    match &definition.params {
        Some(params) => expand_parameterized_macro(line, name, params, &definition.body),
        None => replace_identifier_occurrences(line, name, &definition.body),
    }
}

fn expand_parameterized_macro(line: &str, name: &str, params: &[String], body: &str) -> String {
    let mut result = line.to_string();

    loop {
        let Some(call_start) = find_identifier_call(&result, name) else {
            break;
        };

        let open_index = call_start + name.len();
        let Some(close_index) = find_matching_paren(&result, open_index) else {
            break;
        };

        let args_text = &result[open_index + 1..close_index];
        let args = split_macro_args(args_text);
        let mut expanded = body.to_string();
        for (param, arg) in params.iter().zip(args.iter()) {
            expanded = expanded.replace(param, arg);
        }

        result.replace_range(call_start..=close_index, &expanded);
    }

    result
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

fn substitute_defines_outside_strings(line: &str, defines: &HashMap<String, String>) -> String {
    let mut parts = line.split('"').map(ToOwned::to_owned).collect::<Vec<_>>();
    let mut sorted = defines.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| right.0.len().cmp(&left.0.len()));

    for index in (0..parts.len()).step_by(2) {
        let mut segment = parts[index].clone();
        for (symbol, value) in &sorted {
            segment = replace_identifier_occurrences(&segment, symbol, value);
        }
        parts[index] = segment;
    }

    parts.join("\"")
}

fn replace_identifier_occurrences(line: &str, symbol: &str, value: &str) -> String {
    let mut output = String::with_capacity(line.len());
    let mut cursor = 0usize;

    while let Some(relative) = line[cursor..].find(symbol) {
        let start = cursor + relative;
        let end = start + symbol.len();
        let left_ok = start == 0 || !line[..start].chars().next_back().is_some_and(is_identifier_char);
        let right_ok = end >= line.len() || !line[end..].chars().next().is_some_and(is_identifier_char);

        if left_ok && right_ok {
            output.push_str(&line[cursor..start]);
            output.push_str(value);
            cursor = end;
        } else {
            output.push_str(&line[cursor..end]);
            cursor = end;
        }
    }

    output.push_str(&line[cursor..]);
    output
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
}