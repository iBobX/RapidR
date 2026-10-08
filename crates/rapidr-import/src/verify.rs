//! The proof that a conversion changes nothing: the program and its copy
//! compiled to bytecode, as `rapidr build-bc` compiles them, and compared
//! byte for byte (the source map, which names the files and lines, left out:
//! the copy's files are elsewhere).

use std::path::{Path, PathBuf};

use rapidr_preprocessor::PreprocessOptions;

use crate::files::Files;

/// The bytecode `main` compiles to, its `$INCLUDE`s looked for in its
/// folder and then `include_dirs` (nothing else: not `RAPIDR_INCLUDE_PATH`);
/// `replace`: the main file's text instead of the file's (an upgrade
/// planned, not written yet).
///
/// `builtin_rapidq_inc`: `$INCLUDE "RAPIDQ.INC"` reads RapidR's constants
/// even where a RAPIDQ.INC file would be found (the copy of a program whose
/// RAPIDQ.INC was left out is compared with its original so).
pub fn bytecode(main: &Path, include_dirs: &[PathBuf], replace: Option<&str>, builtin_rapidq_inc: bool) -> Result<Vec<u8>, String> {
    bytecode_with(&crate::files::Disk, main, include_dirs, replace, builtin_rapidq_inc)
}

/// [`bytecode`] of a program in `files`.
pub fn bytecode_with(files: &dyn Files, main: &Path, include_dirs: &[PathBuf], replace: Option<&str>, builtin_rapidq_inc: bool) -> Result<Vec<u8>, String> {
    let text = match replace {
        Some(t) => t.to_string(),
        None => rapidr_preprocessor::decode_source(&files.read(main)?).0,
    };
    let base = main.parent().unwrap_or_else(|| Path::new("."));
    let mut options = PreprocessOptions { include_dirs: include_dirs.to_vec(), ..Default::default() };
    if builtin_rapidq_inc {
        options.virtual_files.push(("RAPIDQ.INC".into(), rapidr_preprocessor::rapidq_inc_text()));
    }
    // (in memory: the other files are the preprocessor's virtual files)
    options.virtual_files.extend(files.virtual_files());
    let pre = rapidr_preprocessor::preprocess_source(&text, base, Some(main.to_path_buf()), options).map_err(|e| e.to_string())?;
    let label = main.display().to_string();
    let remap = |e: String| pre.remap_messages(&label, &e);
    let tokens = rapidr_lexer::Lexer::new(&pre.source, Some(label.clone())).tokenize().map_err(|e| remap(e.to_string()))?;
    let program = rapidr_parser::parse_tokens(&tokens).map_err(|e| remap(e.to_string()))?;
    let library_lines: Vec<bool> = pre.line_map.iter().map(|(file, _)| file.as_deref().is_some_and(|f| f != main)).collect();
    let mut compiled = rapidr_bcgen::compile_program_with_libraries(&program, Some(&pre.source), &library_lines)
        .map_err(|e| remap(e.lines().map(|l| format!("{label}:{l}")).collect::<Vec<_>>().join("\n")))?;
    compiled.module.apply_app_type_directive(pre.app_type.as_deref());
    // (the `$RESOURCE` names, not their bytes: the files are copied as they are)
    for r in &pre.resources {
        compiled.module.resources.push((r.name.clone(), r.file.as_bytes().to_vec()));
    }
    Ok(compiled.module.to_bytes())
}
