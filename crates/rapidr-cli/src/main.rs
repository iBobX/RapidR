use std::env;
use std::fs;
use std::path::Path;
use std::process::{self, ExitCode};

use rapidr_lexer::lex_file as lexer_lex_file;
use rapidr_parser::parse_file as parser_parse_file;
use rapidr_preprocessor::{preprocess_file, PreprocessOptions};

fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().collect();
    args.remove(0); // program name

    // Shortcut: rapidr [--release|--debug] <file.rr|file.rr>
    // When the first non-flag argument looks like a source file and no subcommand is given
    if !args.is_empty() {
        let first_non_flag = args.iter().find(|a| !a.starts_with('-'));
        let has_subcommand = matches!(
            args.first().map(|s| s.as_str()),
            Some("version" | "parse" | "preprocess" | "lex" | "codegen" | "build")
        );
        if !has_subcommand {
            if let Some(file) = first_non_flag {
                if file.ends_with(".rr") || file.ends_with(".rr") {
                    let mut release = true; // default to release
                    let mut source_path = String::new();
                    for arg in &args {
                        match arg.as_str() {
                            "--release" | "-r" => release = true,
                            "--debug" | "-d" => release = false,
                            _ if !arg.starts_with('-') => source_path = arg.clone(),
                            _ => {}
                        }
                    }
                    return build_source_file(&source_path, None, release);
                }
            }
        }
    }

    let first = args.first().map(|s| s.as_str());
    let second = args.get(1).cloned();
    let rest: Vec<String> = if args.len() > 2 { args[2..].to_vec() } else { vec![] };

    match (first, second) {
        (Some("version"), _) => {
            println!("RapidR 0.1.0");
            ExitCode::SUCCESS
        }
        (Some("parse"), Some(path)) => parse_source_file(&path),
        (Some("preprocess"), Some(path)) => preprocess_source_file(&path),
        (Some("lex"), Some(path)) => lex_source_file(&path),
        (Some("codegen"), Some(path)) => {
            let next = rest.first().cloned();
            codegen_source_file(&path, next)
        }
        (Some("build"), Some(path)) => {
            let mut output_dir = None;
            let mut release = false;
            for arg in &rest {
                match arg.as_str() {
                    "--release" | "-r" => release = true,
                    "--debug" | "-d" => release = false,
                    _ => output_dir = Some(arg.clone()),
                }
            }
            build_source_file(&path, output_dir, release)
        }
        _ => {
            eprintln!("Usage:");
            eprintln!("  rapidr version");
            eprintln!("  rapidr [--release|--debug] <file.rr>     Build source file");
            eprintln!("  rapidr parse <file.rr>");
            eprintln!("  rapidr preprocess <file.rr>");
            eprintln!("  rapidr lex <file.rr>");
            eprintln!("  rapidr codegen <file.rr> [output_dir]");
            eprintln!("  rapidr build <file.rr> [output_dir] [--release|-r] [--debug|-d]");
            ExitCode::from(2)
        }
    }
}

fn parse_source_file(path: &str) -> ExitCode {
    match parser_parse_file(path) {
        Ok(program) => {
            println!("statements: {}", program.statements.len());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn preprocess_source_file(path: &str) -> ExitCode {
    match preprocess_file(path, PreprocessOptions::default()) {
        Ok(result) => {
            print!("{}", result.source);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn lex_source_file(path: &str) -> ExitCode {
    match lexer_lex_file(path) {
        Ok(tokens) => {
            for token in tokens {
                if let Some(trailing) = token.trailing.as_deref() {
                    println!(
                        "{:?} {:?} @ {}:{} trailing={:?}",
                        token.kind, token.lexeme, token.line, token.column, trailing
                    );
                } else {
                    println!(
                        "{:?} {:?} @ {}:{}",
                        token.kind, token.lexeme, token.line, token.column
                    );
                }
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

/// Generate Rust source code from a .rr file into an output directory.
fn codegen_source_file(path: &str, output_dir: Option<String>) -> ExitCode {
    let source_path = Path::new(path);
    let stem = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let out_dir = match &output_dir {
        Some(d) => Path::new(d.as_str()).to_path_buf(),
        None => source_path
            .parent()
            .unwrap_or(Path::new("."))
            .join(format!("{stem}_rust")),
    };
    let src_dir = out_dir.join("src");

    // Find the workspace root (where crates/rapidr-runtime-core lives)
    let workspace_root = find_workspace_root();
    let runtime_path = workspace_root
        .as_ref()
        .map(|r| r.join("crates/rapidr-runtime-core"))
        .unwrap_or_else(|| Path::new("crates/rapidr-runtime-core").to_path_buf());

    // Parse
    let program = match parser_parse_file(path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Parse error: {e}");
            return ExitCode::from(1);
        }
    };

    // Generate Rust source
    let rust_source = rapidr_codegen_rust::generate(&program);
    let cargo_toml =
        rapidr_codegen_rust::generate_cargo_toml(stem, &runtime_path.to_string_lossy());

    // Write output
    if let Err(e) = fs::create_dir_all(&src_dir) {
        eprintln!("Cannot create output directory: {e}");
        return ExitCode::from(1);
    }
    if let Err(e) = fs::write(src_dir.join("main.rs"), &rust_source) {
        eprintln!("Cannot write main.rs: {e}");
        return ExitCode::from(1);
    }
    if let Err(e) = fs::write(out_dir.join("Cargo.toml"), &cargo_toml) {
        eprintln!("Cannot write Cargo.toml: {e}");
        return ExitCode::from(1);
    }

    println!("Generated Rust project in {}", out_dir.display());
    println!("  {}/Cargo.toml", out_dir.display());
    println!("  {}/src/main.rs", out_dir.display());
    ExitCode::SUCCESS
}

/// Generate Rust source and then run `cargo build` on it.
fn build_source_file(path: &str, output_dir: Option<String>, release: bool) -> ExitCode {
    let result = codegen_source_file(path, output_dir.clone());
    if result != ExitCode::SUCCESS {
        return result;
    }

    let source_path = Path::new(path);
    let stem = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let out_dir = match &output_dir {
        Some(d) => Path::new(d.as_str()).to_path_buf(),
        None => source_path
            .parent()
            .unwrap_or(Path::new("."))
            .join(format!("{stem}_rust")),
    };

    let profile = if release { "release" } else { "debug" };
    println!("\nBuilding with cargo ({profile})...");
    let mut cargo_args = vec!["build"];
    if release {
        cargo_args.push("--release");
    }
    let status = process::Command::new("cargo")
        .args(&cargo_args)
        .current_dir(&out_dir)
        .status();

    match status {
        Ok(s) if s.success() => {
            // Copy the built binary to the same directory as the .rr source
            let binary_name = stem;
            let built_binary = out_dir.join("target").join(profile).join(binary_name);
            let dest_dir = source_path.parent().unwrap_or(Path::new("."));
            let dest_binary = dest_dir.join(binary_name);

            if built_binary.exists() {
                if let Err(e) = fs::copy(&built_binary, &dest_binary) {
                    eprintln!("Warning: could not copy binary: {e}");
                } else {
                    // Make it executable on Unix
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = fs::set_permissions(&dest_binary, fs::Permissions::from_mode(0o755));
                    }
                    println!("Binary: {}", dest_binary.display());
                }
            }

            println!("Build succeeded!");
            ExitCode::SUCCESS
        }
        Ok(s) => {
            eprintln!("Build failed with {s}");
            ExitCode::from(1)
        }
        Err(e) => {
            eprintln!("Failed to run cargo: {e}");
            ExitCode::from(1)
        }
    }
}

/// Walk up from CWD to find the RapidR workspace root (contains Cargo.toml with [workspace]).
/// Also checks the RAPIDR_HOME environment variable.
fn find_workspace_root() -> Option<std::path::PathBuf> {
    // Check RAPIDR_HOME environment variable first
    if let Ok(home) = env::var("RAPIDR_HOME") {
        let p = Path::new(&home);
        if p.join("Cargo.toml").exists() {
            return Some(p.to_path_buf());
        }
    }

    let cwd = env::current_dir().ok()?;
    let mut dir = cwd.as_path();
    loop {
        let cargo_path = dir.join("Cargo.toml");
        if cargo_path.exists() {
            if let Ok(content) = fs::read_to_string(&cargo_path) {
                if content.contains("[workspace]") {
                    return Some(dir.to_path_buf());
                }
            }
        }
        dir = dir.parent()?;
    }
}