//! The proof that a conversion changes nothing: the program and its copy
//! compiled to bytecode, as `rapidr build-bc` compiles them, and compared
//! byte for byte (the source map, which names the files and lines, left out:
//! the copy's files are elsewhere).

use std::path::{Path, PathBuf};

use rapidr_preprocessor::PreprocessOptions;

use crate::files::Files;

/// The bytecode `main` compiles to, its `$INCLUDE`s looked for as the
/// compiler looks for them — its folder, then `include_dirs` … (nothing
/// else: not the user's include folders unless they are in
/// `include_dirs`); `replace`: the main file's text instead of the file's
/// (an upgrade planned, not written yet).
pub fn bytecode(main: &Path, include_dirs: &[PathBuf], replace: Option<&str>) -> Result<Vec<u8>, String> {
    bytecode_with(&crate::files::Disk, main, include_dirs, replace, &[]).map(|(b, _)| b)
}

/// [`bytecode`] of a program in `files`, and every file it read (its
/// includes: the copy must read its own).
///
/// `builtins`: include file names (`RAPIDQ.INC`) read as RapidR supplies
/// them even where a file of that name would be found — a program whose
/// own RAPIDQ.INC isn't copied is compared with its copy so (the copy finds
/// RapidR's).
pub fn bytecode_with(files: &dyn Files, main: &Path, include_dirs: &[PathBuf], replace: Option<&str>, builtins: &[String]) -> Result<(Vec<u8>, Vec<PathBuf>), String> {
    let text = match replace {
        Some(t) => t.to_string(),
        None => rapidr_preprocessor::decode_source(&files.read(main)?).0,
    };
    let base = main.parent().unwrap_or_else(|| Path::new("."));
    let mut options = PreprocessOptions { include_dirs: include_dirs.to_vec(), ..Default::default() };
    for name in builtins {
        if let Some(text) = rapidr_preprocessor::builtin_include_text(name) {
            options.virtual_files.push((name.clone(), text));
        }
    }
    // (in memory: the other files are the preprocessor's virtual files —
    // but those read as RapidR's)
    let is_builtin = |p: &str| builtins.iter().any(|b| p.replace('\\', "/").rsplit('/').next().is_some_and(|n| n.eq_ignore_ascii_case(b)));
    options.virtual_files.extend(files.virtual_files().into_iter().filter(|(p, _)| !is_builtin(p)));
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
    let read = pre.includes.iter().filter_map(|u| if let rapidr_preprocessor::IncludeFound::File(p) = &u.found { Some(p.clone()) } else { None }).collect();
    Ok((compiled.module.to_bytes(), read))
}
