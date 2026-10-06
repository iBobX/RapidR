//! The CLI's compile pipeline (`compile_to_bytecode` in
//! crates/rapidr-cli/src/main.rs), copied: the program debugged is the
//! program `rapidr run` runs, with the same messages for its errors.

use std::fs;
use std::path::Path;

use rapidr_preprocessor::{preprocess_file, PreprocessOptions};

pub fn compile_to_bytecode(path: &str) -> Result<rapidr_bcgen::Compiled, String> {
    let pre = preprocess_file(path, PreprocessOptions::default()).map_err(|e| e.to_string())?;
    // Positions count preprocessed lines; map them back to the real file
    // and line (code from an $INCLUDE reports the include file).
    let remap = |text: String| pre.remap_messages(path, &text);
    let tokens = rapidr_lexer::Lexer::new(&pre.source, Some(path.to_string()))
        .tokenize()
        .map_err(|e| remap(e.to_string()))?;
    let program = rapidr_parser::parse_tokens(&tokens).map_err(|mut e| {
        for d in &mut e.diagnostics {
            d.file_path.get_or_insert_with(|| path.to_string());
        }
        remap(e.to_string())
    })?;
    // Lines that came from $INCLUDE files (not the program itself).
    let main_file = Path::new(path);
    let library_lines: Vec<bool> = pre
        .line_map
        .iter()
        .map(|(file, _)| file.as_deref().is_some_and(|f| f != main_file))
        .collect();
    let mut compiled = rapidr_bcgen::compile_program_with_libraries(&program, Some(&pre.source), &library_lines).map_err(|e| {
        remap(e.lines().map(|l| format!("{path}:{l}")).collect::<Vec<_>>().join("\n"))
    })?;
    // Run-time errors name the file and line (file names only).
    let origins = pre.line_map.iter().map(|(file, line)| (file.as_deref().and_then(|f| f.to_str()), *line as u32));
    compiled.module.source_map = rapidr_bytecode::SourceMap::from_origins(path, origins);
    compiled.module.apply_app_type_directive(pre.app_type.as_deref());
    // `$RESOURCE` files are built into the module.
    for (name, file) in resource_files(&pre)? {
        let bytes = if file.is_empty() { Vec::new() } else { fs::read(&file).map_err(|e| format!("$RESOURCE {name}: {file}: {e}"))? };
        compiled.module.resources.push((name, bytes));
    }
    Ok(compiled)
}

/// The `$RESOURCE` files of a program, as (name, absolute path); an error
/// names one that wasn't found (as RapidQ's compiler does) — an optional one
/// (`$OPTION ICON`) that isn't there has an empty path: built in empty.
fn resource_files(pre: &rapidr_preprocessor::PreprocessResult) -> Result<Vec<(String, String)>, String> {
    pre.resources
        .iter()
        .map(|r| {
            if r.optional && r.path.is_none() {
                return Ok((r.name.clone(), String::new()));
            }
            let path = r.path.as_ref().ok_or_else(|| format!("$RESOURCE {}: file not found: '{}'", r.name, r.file))?;
            let abs = path.canonicalize().map_err(|e| format!("$RESOURCE {}: {}: {e}", r.name, path.display()))?;
            Ok((r.name.clone(), abs.to_string_lossy().into_owned()))
        })
        .collect()
}
