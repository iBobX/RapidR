use std::env;
use std::fs;
use std::path::{Path, PathBuf};
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
                    let mut interp = false;
                    let mut source_path = String::new();
                    for arg in &args {
                        match arg.as_str() {
                            "--release" | "-r" => release = true,
                            "--debug" | "-d" => release = false,
                            "--web" | "-w" => web = true,
                            "--interp" | "-i" => interp = true,
                            _ if !arg.starts_with('-') => source_path = arg.clone(),
                            _ => {}
                        }
                    }
                    return build_source_file(&source_path, None, release, web, interp);
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
            let mut interp = false;
            for arg in &rest {
                match arg.as_str() {
                    "--release" | "-r" => release = true,
                    "--debug" | "-d" => release = false,
                    "--web" | "-w" => web = true,
                    "--interp" | "-i" => interp = true,
                    _ => output_dir = Some(arg.clone()),
                }
            }
            build_source_file(&path, output_dir, release, web, interp)
        }
        (Some("build-bc"), Some(path)) => {
            let mut out: Option<String> = None;
            let mut iter = rest.iter();
            while let Some(a) = iter.next() {
                match a.as_str() {
                    "-o" | "--output" => { out = iter.next().cloned(); }
                    _ => {}
                }
            }
            build_bytecode_file(&path, out)
        }
        (Some("run-bc"), Some(path)) => run_bytecode_file(&path),
        (Some("bundle-bc"), Some(path)) => {
            let mut out: Option<String> = None;
            let mut wasm: Option<String> = None;
            let mut js: Option<String> = None;
            let mut iter = rest.iter();
            while let Some(a) = iter.next() {
                match a.as_str() {
                    "-o" | "--output" => { out = iter.next().cloned(); }
                    "--wasm" => { wasm = iter.next().cloned(); }
                    "--js" => { js = iter.next().cloned(); }
                    _ => {}
                }
            }
            bundle_bc_file(&path, out, wasm, js)
        }
        _ => {
            eprintln!("Usage:");
            eprintln!("  rapidr version");
            eprintln!("  rapidr [--release|--debug] [--web] [--interp] <file.rr>  Build source file");
            eprintln!("  rapidr parse <file.rr>");
            eprintln!("  rapidr preprocess <file.rr>");
            eprintln!("  rapidr lex <file.rr>");
            eprintln!("  rapidr codegen <file.rr> [output_dir]");
            eprintln!("  rapidr build <file.rr> [output_dir] [--release|-r] [--debug|-d] [--web|-w] [--interp|-i]");
            eprintln!("  rapidr build-bc <file.rr> [-o out.rrbc]          Compile to bytecode");
            eprintln!("  rapidr run-bc <file.rrbc>                        Run bytecode (stub host)");
            eprintln!("  rapidr bundle-bc <file.rr> [-o out.zip]          Build static web bundle");
            eprintln!("        [--wasm rapidrintr.wasm] [--js rapidrintr.js]");
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
///
/// `interp = true` switches the desktop path to **bytecode + stub
/// runner** (single self-contained exe) and the `--web` path to a
/// `bundle-bc`-style static zip.
fn build_source_file(
    path: &str,
    output_dir: Option<String>,
    release: bool,
    web: bool,
    interp: bool,
) -> ExitCode {
    // Detect web target from $APPTYPE or --web flag
    let app_type = preprocess_file(path, PreprocessOptions::default())
        .ok()
        .and_then(|r| r.app_type);
    let is_web = web || app_type.as_deref() == Some("WEB");

    if interp {
        // Bytecode pipeline: skip Rust codegen entirely.
        return if is_web {
            build_interp_web(path, output_dir)
        } else {
            build_interp_desktop(path, output_dir, release)
        };
    }

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
            let target_root = match std::env::var_os("CARGO_TARGET_DIR") {
                Some(p) => PathBuf::from(p),
                None => out_dir.join("target"),
            };
            let built_binary = target_root.join(profile).join(binary_name);
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
    let target_root = match std::env::var_os("CARGO_TARGET_DIR") {
        Some(p) => PathBuf::from(p),
        None => out_dir.join("target"),
    };
    let wasm_file = target_root
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
    let css = rapidr_rrcss::RR_BASE_CSS;
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{title}</title>
  <style>{css}</style>
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

// ---------------- Bytecode (rapidrintr) ----------------

fn build_bytecode_file(path: &str, output: Option<String>) -> ExitCode {
    let program = match parser_parse_file(path) {
        Ok(p) => p,
        Err(e) => { eprintln!("{e}"); return ExitCode::from(1); }
    };
    let compiled = match rapidr_bcgen::compile_program(&program) {
        Ok(c) => c,
        Err(e) => { eprintln!("bcgen error: {e}"); return ExitCode::from(1); }
    };
    for w in &compiled.warnings {
        eprintln!("warning: {w}");
    }
    let bytes = compiled.module.to_bytes();
    let out_path = output.unwrap_or_else(|| {
        let p = Path::new(path);
        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("program");
        format!("{stem}.rrbc")
    });
    if let Err(e) = fs::write(&out_path, &bytes) {
        eprintln!("write {out_path}: {e}"); return ExitCode::from(1);
    }
    println!("wrote {} ({} bytes, {} fns, {} consts)",
        out_path, bytes.len(),
        compiled.module.functions.len(),
        compiled.module.consts.len());
    ExitCode::SUCCESS
}

fn run_bytecode_file(path: &str) -> ExitCode {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => { eprintln!("read {path}: {e}"); return ExitCode::from(1); }
    };
    // Delegate to `rapidr-vm-host-native::run_bytes`, which installs the
    // indirect event dispatcher *before* `MAIN` runs — required for any
    // program that calls `Form.ShowModal` from MAIN (the modal blocks
    // in FLTK's own `app::wait()` loop, so events fire while we are
    // still inside `vm.run`).
    if let Err(e) = rapidr_vm_host_native::run_bytes(&bytes) {
        eprintln!("{e}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

/// `bundle-bc <file.rr> [-o out.zip] [--wasm rapidrintr.wasm] [--js rapidrintr.js]`
///
/// Compiles the source to bytecode, then assembles a static-hostable
/// web bundle (zip) containing index.html, loader.js, the bytecode
/// interpreter wasm + js, and the program's `.rrbc`. The `.wasm` and
/// `.js` paths default to looking next to the rapidr binary or under
/// `target/web/` (see [`locate_rapidrintr_artifacts`]).
fn bundle_bc_file(
    path: &str,
    output: Option<String>,
    wasm_path: Option<String>,
    js_path: Option<String>,
) -> ExitCode {
    // 1. Compile source to bytecode.
    let program = match parser_parse_file(path) {
        Ok(p) => p,
        Err(e) => { eprintln!("{e}"); return ExitCode::from(1); }
    };
    let compiled = match rapidr_bcgen::compile_program(&program) {
        Ok(c) => c,
        Err(e) => { eprintln!("bcgen error: {e}"); return ExitCode::from(1); }
    };
    for w in &compiled.warnings { eprintln!("warning: {w}"); }
    let rrbc = compiled.module.to_bytes();

    let stem = Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("program")
        .to_string();

    // 2. Locate rapidrintr.wasm + rapidrintr.js (user-supplied or defaults).
    let (wasm_p, js_p) = match (wasm_path, js_path) {
        (Some(w), Some(j)) => (PathBuf::from(w), PathBuf::from(j)),
        (w_opt, j_opt) => match locate_rapidrintr_artifacts() {
            Some((w, j)) => (
                w_opt.map(PathBuf::from).unwrap_or(w),
                j_opt.map(PathBuf::from).unwrap_or(j),
            ),
            None => {
                eprintln!(
                    "error: could not locate rapidrintr.wasm + rapidrintr.js\n\
                     hint: build with `wasm-pack build interpreter/rapidr-vm-host-web --target web --out-dir ../../target/web`\n\
                     or pass --wasm <path> --js <path>",
                );
                return ExitCode::from(1);
            }
        },
    };
    let wasm_bytes = match fs::read(&wasm_p) {
        Ok(b) => b,
        Err(e) => { eprintln!("read {}: {e}", wasm_p.display()); return ExitCode::from(1); }
    };
    let js_text = match fs::read_to_string(&js_p) {
        Ok(s) => s,
        Err(e) => { eprintln!("read {}: {e}", js_p.display()); return ExitCode::from(1); }
    };

    // 3. Build the bundle.
    let bundle = match rapidr_webbundle::build_bundle(&rapidr_webbundle::BundleInputs {
        project_name: &stem,
        rrbc: &rrbc,
        rapidrintr_wasm: &wasm_bytes,
        rapidrintr_js: &js_text,
        title: None,
    }) {
        Ok(b) => b,
        Err(e) => { eprintln!("bundle error: {e}"); return ExitCode::from(1); }
    };

    // 4. Write to disk.
    let out_path = output.unwrap_or_else(|| format!("{stem}-web.zip"));
    if let Err(e) = fs::write(&out_path, &bundle) {
        eprintln!("write {out_path}: {e}"); return ExitCode::from(1);
    }
    println!(
        "wrote {} ({} bytes) — unzip and serve via any static host",
        out_path,
        bundle.len(),
    );
    ExitCode::SUCCESS
}

/// Default lookup for the rapidrintr wasm/js artifacts produced by
/// `wasm-pack build interpreter/rapidr-vm-host-web --target web`.
/// Searches a few well-known locations relative to the current dir.
fn locate_rapidrintr_artifacts() -> Option<(PathBuf, PathBuf)> {
    let candidates = [
        Path::new("target/web"),
        Path::new("target/web-bundle"),
        Path::new("interpreter/rapidr-vm-host-web/pkg"),
        Path::new("pkg"),
    ];
    for dir in candidates {
        let wasm = dir.join("rapidrintr_bg.wasm");
        let wasm_alt = dir.join("rapidr_vm_host_web_bg.wasm");
        let js = dir.join("rapidrintr.js");
        let js_alt = dir.join("rapidr_vm_host_web.js");
        let w = if wasm.exists() { Some(wasm) }
                else if wasm_alt.exists() { Some(wasm_alt) }
                else { None };
        let j = if js.exists() { Some(js) }
                else if js_alt.exists() { Some(js_alt) }
                else { None };
        if let (Some(w), Some(j)) = (w, j) {
            return Some((w, j));
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Phase 8: interpreted-mode build helpers
// ---------------------------------------------------------------------------

/// `rapidr build <file.rr> --interp` — compile to bytecode, then
/// produce a single self-contained native executable by appending the
/// bytecode + 12-byte footer to a copy of `rapidrintr-runner`.
fn build_interp_desktop(
    path: &str,
    output_dir: Option<String>,
    release: bool,
) -> ExitCode {
    // 1. Compile source → bytecode.
    let program = match parser_parse_file(path) {
        Ok(p) => p,
        Err(e) => { eprintln!("{e}"); return ExitCode::from(1); }
    };
    let compiled = match rapidr_bcgen::compile_program(&program) {
        Ok(c) => c,
        Err(e) => { eprintln!("bcgen error: {e}"); return ExitCode::from(1); }
    };
    for w in &compiled.warnings { eprintln!("warning: {w}"); }
    let rrbc = compiled.module.to_bytes();

    // 2. Locate (or build) the runner stub.
    let stub = match locate_or_build_stub(release) {
        Ok(p) => p,
        Err(e) => { eprintln!("{e}"); return ExitCode::from(1); }
    };

    // 3. Choose destination — same convention as compiled mode: drop
    //    the binary alongside the source file (or in `output_dir`).
    let source_path = Path::new(path);
    let stem = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let dest_dir = match &output_dir {
        Some(d) => Path::new(d.as_str()).to_path_buf(),
        None => source_path.parent().unwrap_or(Path::new(".")).to_path_buf(),
    };
    if let Err(e) = fs::create_dir_all(&dest_dir) {
        eprintln!("create_dir_all {}: {e}", dest_dir.display());
        return ExitCode::from(1);
    }
    let dest = dest_dir.join(stem);

    // 4. Attach payload.
    if let Err(e) = attach_payload(&stub, &rrbc, &dest) {
        eprintln!("attach_payload: {e}");
        return ExitCode::from(1);
    }

    println!(
        "Built interpreted binary: {} ({} bytes total, {} bytes payload)",
        dest.display(),
        fs::metadata(&dest).map(|m| m.len()).unwrap_or(0),
        rrbc.len(),
    );
    ExitCode::SUCCESS
}

/// `rapidr build --web --interp <file.rr>` — compile to bytecode and
/// emit a static web bundle (`<stem>-web.zip`). Delegates to the same
/// pipeline as `bundle-bc`.
fn build_interp_web(path: &str, output_dir: Option<String>) -> ExitCode {
    let stem = Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("program")
        .to_string();
    let out_dir = match &output_dir {
        Some(d) => Path::new(d.as_str()).to_path_buf(),
        None => Path::new(path).parent().unwrap_or(Path::new(".")).to_path_buf(),
    };
    if let Err(e) = fs::create_dir_all(&out_dir) {
        eprintln!("create_dir_all {}: {e}", out_dir.display());
        return ExitCode::from(1);
    }
    let out_path = out_dir.join(format!("{stem}-web.zip"));
    bundle_bc_file(path, Some(out_path.to_string_lossy().into_owned()), None, None)
}

/// Append `[rrbc bytes][magic 8B "RRBCEXE1"][u32 LE length]` to a copy
/// of `stub`. The result is a fully self-contained executable that, on
/// startup, slices off its own payload and runs it via
/// `rapidr-vm-host-native`.
fn attach_payload(stub: &Path, rrbc: &[u8], dest: &Path) -> Result<(), String> {
    fs::copy(stub, dest).map_err(|e| format!("copy stub: {e}"))?;

    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(dest)
        .map_err(|e| format!("open {}: {e}", dest.display()))?;
    f.write_all(rrbc).map_err(|e| format!("write payload: {e}"))?;
    f.write_all(b"RRBCEXE1").map_err(|e| format!("write magic: {e}"))?;
    let len = u32::try_from(rrbc.len())
        .map_err(|_| "bytecode payload exceeds 4 GiB".to_string())?;
    f.write_all(&len.to_le_bytes()).map_err(|e| format!("write len: {e}"))?;
    drop(f);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(dest, fs::Permissions::from_mode(0o755));
    }
    Ok(())
}

/// Locate `rapidrintr-runner`, building it on demand if absent.
///
/// Search order: `target/release/`, `target/debug/`, then fall back to
/// `cargo build -p rapidr-runner-stub --release`.
fn locate_or_build_stub(release: bool) -> Result<PathBuf, String> {
    let exe_name = if cfg!(windows) { "rapidrintr-runner.exe" } else { "rapidrintr-runner" };
    let preferred = if release { "release" } else { "debug" };

    for profile in [preferred, if preferred == "release" { "debug" } else { "release" }] {
        let candidate = Path::new("target").join(profile).join(exe_name);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    println!("Building rapidrintr-runner stub ({preferred})...");
    let mut args = vec!["build", "-p", "rapidr-runner-stub"];
    if release { args.push("--release"); }
    let status = process::Command::new("cargo")
        .args(&args)
        .status()
        .map_err(|e| format!("spawn cargo: {e}"))?;
    if !status.success() {
        return Err(format!("cargo build rapidr-runner-stub failed: {status}"));
    }
    let built = Path::new("target").join(preferred).join(exe_name);
    if built.exists() {
        Ok(built)
    } else {
        Err(format!("could not locate {} after build", built.display()))
    }
}
