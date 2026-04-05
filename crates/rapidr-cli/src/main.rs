use std::env;
use std::fs;
use std::path::Path;
use std::process::{self, ExitCode};

use rapidr_codegen_rust::AppTarget;
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
                    let mut web = false;
                    let mut source_path = String::new();
                    for arg in &args {
                        match arg.as_str() {
                            "--release" | "-r" => release = true,
                            "--debug" | "-d" => release = false,
                            "--web" | "-w" => web = true,
                            _ if !arg.starts_with('-') => source_path = arg.clone(),
                            _ => {}
                        }
                    }
                    return build_source_file(&source_path, None, release, web);
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
            let mut web = false;
            for arg in &rest {
                match arg.as_str() {
                    "--release" | "-r" => release = true,
                    "--debug" | "-d" => release = false,
                    "--web" | "-w" => web = true,
                    _ => output_dir = Some(arg.clone()),
                }
            }
            build_source_file(&path, output_dir, release, web)
        }
        _ => {
            eprintln!("Usage:");
            eprintln!("  rapidr version");
            eprintln!("  rapidr [--release|--debug] [--web] <file.rr>     Build source file");
            eprintln!("  rapidr parse <file.rr>");
            eprintln!("  rapidr preprocess <file.rr>");
            eprintln!("  rapidr lex <file.rr>");
            eprintln!("  rapidr codegen <file.rr> [output_dir]");
            eprintln!("  rapidr build <file.rr> [output_dir] [--release|-r] [--debug|-d] [--web|-w]");
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
    codegen_source_file_inner(path, output_dir, false)
}

fn codegen_source_file_inner(path: &str, output_dir: Option<String>, force_web: bool) -> ExitCode {
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

    // Preprocess to detect $APPTYPE
    let app_type = preprocess_file(path, PreprocessOptions::default())
        .ok()
        .and_then(|r| r.app_type);

    let target = if force_web || app_type.as_deref() == Some("WEB") {
        AppTarget::Web
    } else {
        AppTarget::Desktop
    };

    let runtime_path = if target == AppTarget::Web {
        workspace_root
            .as_ref()
            .map(|r| r.join("crates/rapidr-runtime-web"))
            .unwrap_or_else(|| Path::new("crates/rapidr-runtime-web").to_path_buf())
    } else {
        workspace_root
            .as_ref()
            .map(|r| r.join("crates/rapidr-runtime-core"))
            .unwrap_or_else(|| Path::new("crates/rapidr-runtime-core").to_path_buf())
    };

    // Parse
    let program = match parser_parse_file(path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Parse error: {e}");
            return ExitCode::from(1);
        }
    };

    // Generate Rust source
    let rust_source = rapidr_codegen_rust::generate_for_target(&program, target);
    let cargo_toml = if target == AppTarget::Web {
        rapidr_codegen_rust::generate_cargo_toml_web(stem, &runtime_path.to_string_lossy())
    } else {
        rapidr_codegen_rust::generate_cargo_toml(stem, &runtime_path.to_string_lossy())
    };

    // Write output
    if let Err(e) = fs::create_dir_all(&src_dir) {
        eprintln!("Cannot create output directory: {e}");
        return ExitCode::from(1);
    }

    // For web, the generated code goes in lib.rs (cdylib); for desktop, main.rs
    let source_filename = if target == AppTarget::Web { "lib.rs" } else { "main.rs" };
    if let Err(e) = fs::write(src_dir.join(source_filename), &rust_source) {
        eprintln!("Cannot write {}: {e}", source_filename);
        return ExitCode::from(1);
    }
    if let Err(e) = fs::write(out_dir.join("Cargo.toml"), &cargo_toml) {
        eprintln!("Cannot write Cargo.toml: {e}");
        return ExitCode::from(1);
    }

    let target_label = if target == AppTarget::Web { "web" } else { "desktop" };
    println!("Generated Rust project ({}) in {}", target_label, out_dir.display());
    println!("  {}/Cargo.toml", out_dir.display());
    println!("  {}/src/{}", out_dir.display(), source_filename);
    ExitCode::SUCCESS
}

/// Generate Rust source and then run `cargo build` on it.
fn build_source_file(path: &str, output_dir: Option<String>, release: bool, web: bool) -> ExitCode {
    // Detect web target from $APPTYPE or --web flag
    let app_type = preprocess_file(path, PreprocessOptions::default())
        .ok()
        .and_then(|r| r.app_type);
    let is_web = web || app_type.as_deref() == Some("WEB");

    let result = codegen_source_file_inner(path, output_dir.clone(), is_web);
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

    if is_web {
        build_web(path, &out_dir, stem, release)
    } else {
        build_desktop(path, &out_dir, stem, release)
    }
}

fn build_desktop(path: &str, out_dir: &Path, stem: &str, release: bool) -> ExitCode {
    let source_path = Path::new(path);
    let profile = if release { "release" } else { "debug" };
    println!("\nBuilding with cargo ({profile})...");
    let mut cargo_args = vec!["build"];
    if release {
        cargo_args.push("--release");
    }
    let status = process::Command::new("cargo")
        .args(&cargo_args)
        .current_dir(out_dir)
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

fn build_web(path: &str, out_dir: &Path, stem: &str, release: bool) -> ExitCode {
    let source_path = Path::new(path);
    let profile = if release { "release" } else { "debug" };

    // Step 1: Compile with cargo for wasm32-unknown-unknown
    println!("\nBuilding WASM ({profile})...");
    let mut cargo_args = vec!["build", "--target", "wasm32-unknown-unknown"];
    if release {
        cargo_args.push("--release");
    }
    let status = process::Command::new("cargo")
        .args(&cargo_args)
        .current_dir(out_dir)
        .status();

    match status {
        Ok(s) if !s.success() => {
            eprintln!("WASM build failed with {s}");
            return ExitCode::from(1);
        }
        Err(e) => {
            eprintln!("Failed to run cargo: {e}");
            return ExitCode::from(1);
        }
        _ => {}
    }

    // Step 2: Run wasm-bindgen to generate JS glue
    let wasm_file = out_dir
        .join("target")
        .join("wasm32-unknown-unknown")
        .join(profile)
        .join(format!("{}.wasm", stem.replace('-', "_")));

    let dest_dir = source_path.parent().unwrap_or(Path::new("."));
    let web_out = dest_dir.join(format!("{stem}_web"));
    if let Err(e) = fs::create_dir_all(&web_out) {
        eprintln!("Cannot create web output directory: {e}");
        return ExitCode::from(1);
    }

    println!("Running wasm-bindgen...");
    let wb_status = process::Command::new("wasm-bindgen")
        .args([
            "--out-dir",
            &web_out.to_string_lossy(),
            "--target",
            "web",
            "--no-typescript",
            &wasm_file.to_string_lossy(),
        ])
        .status();

    match wb_status {
        Ok(s) if !s.success() => {
            eprintln!("wasm-bindgen failed with {s}");
            return ExitCode::from(1);
        }
        Err(e) => {
            eprintln!("Failed to run wasm-bindgen: {e}");
            eprintln!("  Install with: cargo install wasm-bindgen-cli");
            return ExitCode::from(1);
        }
        _ => {}
    }

    // Step 3: Generate index.html
    let wasm_module = stem.replace('-', "_");
    let html = generate_html_shell(stem, &wasm_module);
    if let Err(e) = fs::write(web_out.join("index.html"), &html) {
        eprintln!("Cannot write index.html: {e}");
        return ExitCode::from(1);
    }

    println!("Web build: {}", web_out.display());
    println!("  {}/index.html", web_out.display());
    println!("  {}/{}_bg.wasm", web_out.display(), wasm_module);
    println!("  {}/{}.js", web_out.display(), wasm_module);
    println!("\nServe with: python3 -m http.server -d {} 8080", web_out.display());
    println!("Build succeeded!");
    ExitCode::SUCCESS
}

fn generate_html_shell(title: &str, wasm_module: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{title}</title>
  <style>
    * {{ box-sizing: border-box; }}
    body {{ margin: 0; padding: 0; background: #e8e8e8; font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; font-size: 13px; overflow: auto; }}
    .rr-form {{ position: absolute; background: #f0f0f0; border: 1px solid #888; border-radius: 6px; box-shadow: 0 4px 16px rgba(0,0,0,0.18); overflow: hidden; }}
    .rr-form-titlebar {{ background: linear-gradient(135deg, #4a90d9, #357abd); color: white; padding: 7px 12px; font-weight: 600; font-size: 13px; user-select: none; cursor: default; letter-spacing: 0.3px; }}
    .rr-widget {{ position: absolute; box-sizing: border-box; }}
    button.rr-widget {{ background: linear-gradient(to bottom, #4a90d9, #3a7bc8); color: white; border: 1px solid #2d6db5; border-radius: 4px; padding: 4px 14px; font-size: 13px; cursor: pointer; font-family: inherit; transition: background 0.15s; }}
    button.rr-widget:hover {{ background: linear-gradient(to bottom, #5a9ee9, #4a8bd8); }}
    button.rr-widget:active {{ background: linear-gradient(to bottom, #2d6db5, #3a7bc8); }}
    input[type="text"].rr-widget, input[type="password"].rr-widget {{ border: 1px solid #aaa; border-radius: 3px; padding: 4px 8px; font-size: 13px; font-family: inherit; outline: none; background: white; }}
    input[type="text"].rr-widget:focus, input[type="password"].rr-widget:focus {{ border-color: #4a90d9; box-shadow: 0 0 3px rgba(74,144,217,0.4); }}
    textarea.rr-widget {{ border: 1px solid #aaa; border-radius: 3px; padding: 6px 8px; font-size: 13px; font-family: inherit; outline: none; resize: none; background: white; }}
    textarea.rr-widget:focus {{ border-color: #4a90d9; box-shadow: 0 0 3px rgba(74,144,217,0.4); }}
    select.rr-widget {{ border: 1px solid #aaa; border-radius: 3px; padding: 4px 8px; font-size: 13px; font-family: inherit; background: white; cursor: pointer; }}
    progress.rr-widget {{ border: none; border-radius: 3px; height: 22px; appearance: none; -webkit-appearance: none; }}
    progress.rr-widget::-webkit-progress-bar {{ background: #ddd; border-radius: 3px; }}
    progress.rr-widget::-webkit-progress-value {{ background: linear-gradient(to right, #4caf50, #45a049); border-radius: 3px; }}
    table.rr-grid {{ border-collapse: collapse; width: 100%; font-size: 13px; }}
    table.rr-grid th, table.rr-grid td {{ border: 1px solid #ccc; padding: 5px 10px; text-align: left; }}
    table.rr-grid th {{ background: #e0e0e0; font-weight: 600; position: sticky; top: 0; }}
    table.rr-grid tr:nth-child(even) {{ background: #f8f8f8; }}
    table.rr-grid tr:hover {{ background: #e8f0fe; }}
    .rr-tab-btn {{ padding: 6px 16px; cursor: pointer; border: none; background: transparent; font-size: 13px; font-family: inherit; border-bottom: 2px solid transparent; color: #555; transition: all 0.15s; }}
    .rr-tab-btn:hover {{ color: #333; background: #e8e8e8; }}
    .rr-tab-btn.active {{ color: #4a90d9; border-bottom-color: #4a90d9; font-weight: 600; }}
    canvas.rr-widget {{ border: 1px solid #aaa; background: white; cursor: crosshair; }}
    label.rr-widget {{ font-size: 13px; display: flex; align-items: center; gap: 4px; cursor: pointer; }}
    fieldset.rr-widget {{ border: 1px solid #aaa; border-radius: 4px; padding: 8px; }}
    fieldset.rr-widget legend {{ font-size: 13px; padding: 0 4px; }}
    .rr-plot-container {{ border: 1px solid #aaa; border-radius: 3px; background: white; overflow: hidden; }}
    #rr-console {{ position: fixed; bottom: 0; left: 0; width: 100%; max-height: 200px; overflow-y: auto; background: #1e1e1e; color: #d4d4d4; font-family: 'Consolas', 'Monaco', monospace; font-size: 13px; padding: 8px; display: none; z-index: 10000; border-top: 2px solid #333; }}
  </style>
</head>
<body>
  <div id="rr-root"></div>
  <pre id="rr-console"></pre>
  <script type="module">
    import init from './{wasm_module}.js';
    init();
  </script>
</body>
</html>
"#
    )
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