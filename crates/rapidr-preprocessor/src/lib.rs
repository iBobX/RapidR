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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreprocessResult {
    pub source: String,
    /// The value of `$APPTYPE` if present (e.g. "GUI", "CONSOLE", "WEB").
    pub app_type: Option<String>,
}

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
    let source = fs::read_to_string(path).map_err(|error| {
        PreprocessError::new(
            format!("Failed to read source file '{}': {error}", path.display()),
            1,
            1,
            Some(path.display().to_string()),
        )
    })?;

    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    preprocess_with_state(
        &source,
        base_dir,
        Some(path.to_path_buf()),
        options.defines,
        Vec::new(),
    )
}

pub fn preprocess_source(
    source: &str,
    base_dir: impl AsRef<Path>,
    file_path: Option<PathBuf>,
    options: PreprocessOptions,
) -> Result<PreprocessResult, PreprocessError> {
    preprocess_with_state(
        source,
        base_dir.as_ref(),
        file_path,
        options.defines,
        Vec::new(),
    )
}

fn preprocess_with_state(
    source: &str,
    base_dir: &Path,
    file_path: Option<PathBuf>,
    mut defines: HashMap<String, String>,
    mut include_stack: Vec<PathBuf>,
) -> Result<PreprocessResult, PreprocessError> {
    let mut macros = HashMap::<String, MacroDefinition>::new();
    let mut output_lines = Vec::new();
    let mut skip_stack: Vec<bool> = Vec::new();
    let mut app_type: Option<String> = None;
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
            let should_skip = !defines.contains_key(&symbol);
            if skip_stack.last().copied().unwrap_or(false) {
                skip_stack.push(true);
            } else {
                skip_stack.push(should_skip);
            }
            output_lines.push(String::new());
            continue;
        }

        if upper_line.starts_with("$IFNDEF") {
            let symbol = line
                .split_once(char::is_whitespace)
                .map(|(_, value)| strip_inline_comment(value).trim().to_string())
                .unwrap_or_default();
            let should_skip = defines.contains_key(&symbol);
            if skip_stack.last().copied().unwrap_or(false) {
                skip_stack.push(true);
            } else {
                skip_stack.push(should_skip);
            }
            output_lines.push(String::new());
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
            output_lines.push(String::new());
            continue;
        }

        if upper_line.starts_with("$ENDIF") {
            if !skip_stack.is_empty() {
                skip_stack.pop();
            }
            output_lines.push(String::new());
            continue;
        }

        if skip_stack.last().copied().unwrap_or(false) {
            output_lines.push(String::new());
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
                defines.insert(symbol, value);
            }
            output_lines.push(String::new());
            continue;
        }

        if upper_line.starts_with("$UNDEF") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                defines.remove(parts[1]);
            }
            output_lines.push(String::new());
            continue;
        }

        if upper_line.starts_with("$MACRO") {
            if let Some((name, definition)) = parse_macro_definition(line) {
                macros.insert(name, definition);
            }
            output_lines.push(String::new());
            continue;
        }

        if upper_line.starts_with("$APPTYPE")
            || upper_line.starts_with("$OPTIMIZE")
            || upper_line.starts_with("$ESCAPECHARS")
            || upper_line.starts_with("$THEME")
        {
            // Extract $APPTYPE value
            if upper_line.starts_with("$APPTYPE") {
                if let Some((_, value)) = line.split_once(char::is_whitespace) {
                    let val = strip_inline_comment(value).trim().to_uppercase();
                    if !val.is_empty() {
                        app_type = Some(val);
                    }
                }
            }
            output_lines.push(original_line.to_string());
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

            let include_path = resolve_include_path(base_dir, &include_file).ok_or_else(|| {
                PreprocessError::new(
                    format!("Include file not found: '{include_file}'"),
                    line_number,
                    1,
                    file_label.clone(),
                )
            })?;

            if include_stack.iter().any(|entry| entry == &include_path) {
                return Err(PreprocessError::new(
                    format!("Recursive include detected: '{include_file}'"),
                    line_number,
                    1,
                    file_label.clone(),
                ));
            }

            let include_source = fs::read_to_string(&include_path).map_err(|error| {
                PreprocessError::new(
                    format!("Failed to include '{include_file}': {error}"),
                    line_number,
                    1,
                    file_label.clone(),
                )
            })?;

            include_stack.push(include_path.clone());
            let nested = preprocess_with_state(
                &include_source,
                include_path.parent().unwrap_or_else(|| Path::new(".")),
                Some(include_path.clone()),
                defines.clone(),
                include_stack.clone(),
            )?;
            include_stack.pop();
            output_lines.push(nested.source);
            continue;
        }

        let mut processed_line = original_line.to_string();

        if !macros.is_empty() {
            for (name, definition) in &macros {
                processed_line = expand_macro(&processed_line, name, definition);
            }
        }

        if !defines.is_empty() && !upper_line.contains('$') {
            processed_line = substitute_defines_outside_strings(&processed_line, &defines);
        }

        output_lines.push(processed_line);
    }

    Ok(PreprocessResult {
        source: output_lines.join("\n"),
        app_type,
    })
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

fn resolve_include_path(base_dir: &Path, include_file: &str) -> Option<PathBuf> {
    let direct = base_dir.join(include_file);
    if direct.exists() {
        return Some(direct);
    }

    let cwd = std::env::current_dir().ok()?.join(include_file);
    cwd.exists().then_some(cwd)
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
    fn options_seed_initial_defines() {
        let mut defines = HashMap::new();
        defines.insert("DEBUG".to_string(), "1".to_string());
        let result = preprocess_source(
            "$IFDEF DEBUG\nPRINT 1\n$ENDIF",
            ".",
            None,
            PreprocessOptions { defines },
        )
        .unwrap();
        assert!(result.source.contains("PRINT 1"));
    }
}