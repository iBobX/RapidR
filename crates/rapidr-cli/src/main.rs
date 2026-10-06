use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, ExitCode};

use rapidr_codegen_rust::AppTarget;
use rapidr_lexer::lex_file as lexer_lex_file;
use rapidr_parser::parse_file as parser_parse_file;
use rapidr_preprocessor::{preprocess_file, PreprocessOptions};

mod home;
mod lang;
mod macos;
mod launch;
mod notices;
mod setup;

use home::Home;

/// The subcommands (a first argument that is one isn't a file).
const SUBCOMMANDS: &[&str] = &[
    "version", "run", "open", "info", "about", "ide", "setup", "notices", "lang", "parse", "preprocess", "lex", "codegen", "build", "build-bc", "run-bc", "bundle-bc", "__dialog",
];

/// `--log <file> <command…>`: this rapidr again with the command, its
/// standard output and error in the file.
fn run_logged(args: &[String]) -> ExitCode {
    let Some((log, rest)) = args.split_first() else {
        eprintln!("--log <file> <command…>");
        return ExitCode::from(2);
    };
    let file = match fs::File::create(log) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{log}: {e}");
            return ExitCode::from(1);
        }
    };
    let status = file.try_clone().and_then(|err| {
        let exe = env::current_exe()?;
        process::Command::new(exe).args(rest).stdin(process::Stdio::null()).stdout(file).stderr(err).status()
    });
    match status {
        Ok(s) => ExitCode::from(s.code().unwrap_or(1).clamp(0, 255) as u8),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

/// A `#!` script: its first line starts with `#!` (`#!/usr/bin/env rapidr`).
fn is_script(path: &str) -> bool {
    use std::io::Read;
    let mut start = [0u8; 2];
    fs::File::open(path).and_then(|mut f| f.read_exact(&mut start)).is_ok() && &start == b"#!"
}

fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().collect();
    args.remove(0); // program name

    // `rapidr --log <file> <command…>`: the command's output (and that of
    // the tools it runs: cargo) in a file — for programs that run rapidr
    // (the IDE) on any system without a shell's redirections.
    if args.first().map(String::as_str) == Some("--log") {
        return run_logged(&args[1..]);
    }

    // Shortcuts: `rapidr [--release|--debug] [--web] [--interp] <file.rr|.bas>`
    // builds it; `rapidr <file.rrbc> [args]` and a `#!/usr/bin/env rapidr`
    // script (`rapidr script.rr [args]`) run it.
    if let Some(at) = args.iter().position(|a| !a.starts_with('-')) {
        let file = args[at].clone();
        let lower = file.to_ascii_lowercase();
        let is_subcommand = at == 0 && SUBCOMMANDS.contains(&file.as_str());
        if !is_subcommand && (lower.ends_with(".rrbc") || is_script(&file)) {
            return launch::run(&file, args[at + 1..].to_vec(), launch::From::Command);
        }
        if !is_subcommand && (lower.ends_with(".rr") || lower.ends_with(".bas")) {
            let mut release = true; // default to release
            let mut web = false;
            let mut interp = false;
            for arg in &args {
                match arg.as_str() {
                    "--release" | "-r" => release = true,
                    "--debug" | "-d" => release = false,
                    "--web" | "-w" => web = true,
                    "--interp" | "-i" => interp = true,
                    _ => {}
                }
            }
            return build_source_file(&file, None, release, web, interp, None);
        }
    }

    let first = args.first().map(|s| s.as_str());
    let second = args.get(1).cloned();
    let rest: Vec<String> = if args.len() > 2 { args[2..].to_vec() } else { vec![] };

    match (first, second) {
        (Some("version"), _) => {
            println!("RapidR {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        (Some("run"), Some(path)) => launch::run(&path, rest, launch::From::Command),
        (Some("open"), Some(path)) => launch::run(&path, rest, launch::From::Desktop),
        (Some("info"), Some(path)) => launch::info(&path),
        (Some("about"), _) => launch::about(),
        (Some("ide"), _) => launch::ide(args[1..].to_vec()),
        (Some("setup"), _) => setup::setup(&args[1..]),
        (Some("notices"), _) => notices::command(&args[1..]),
        (Some("lang"), _) => lang::command(&args[1..]),
        (Some("__dialog"), Some(path)) => launch::run_dialog(&path),
        (Some("parse"), Some(path)) => parse_source_file(&path),
        (Some("preprocess"), Some(path)) => preprocess_source_file(&path),
        (Some("lex"), Some(path)) => lex_source_file(&path),
        (Some("codegen"), Some(path)) => {
            let next = rest.first().cloned();
            codegen_source_file(&path, next)
        }
        (Some("build"), Some(path)) => {
            let mut output_dir = None;
            let mut release = None;
            let mut web = false;
            let mut interp = false;
            let mut target = None;
            let mut iter = rest.iter();
            while let Some(arg) = iter.next() {
                match arg.as_str() {
                    "--target" => target = iter.next().cloned(),
                    "--release" | "-r" => release = Some(true),
                    "--debug" | "-d" => release = Some(false),
                    "--web" | "-w" => web = true,
                    "--interp" | "-i" => interp = true,
                    // (the UI kernel is the only desktop host now: `--host
                    // kernel` is accepted and changes nothing)
                    "--host" | "--host=kernel" | "--host=fltk" => {
                        let host = arg.strip_prefix("--host=").map(str::to_string).or_else(|| iter.next().cloned()).unwrap_or_default();
                        if host != "kernel" {
                            eprintln!("--host {host}: RapidR has one desktop host, the UI kernel (FLTK was removed)");
                            return ExitCode::from(2);
                        }
                        eprintln!("note: --host kernel is no longer needed: the UI kernel is RapidR's only desktop host");
                    }
                    _ => output_dir = Some(arg.clone()),
                }
            }
            // Native builds default to a quick debug compile; interpreted
            // ones to the optimized runner (built once, then reused).
            if target.is_some() && !interp {
                eprintln!("--target: only interpreted builds (--interp) pick a target; native builds are for this machine");
                return ExitCode::from(2);
            }
            build_source_file(&path, output_dir, release.unwrap_or(interp), web, interp, target)
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
            eprintln!("  rapidr --log <file> <command…>                     The command's output in a file");
            eprintln!("  rapidr run <file.rrbc|.rr|.bas> [args]             Run a program (the RapidR Runtime)");
            eprintln!("  rapidr open <file> [args]                        Run it as opening it from the desktop does");
            eprintln!("  rapidr info <file>                               Its app type, format and the runtime it needs");
            eprintln!("  rapidr setup [--check] [--yes] [--toolchain gnullvm|msvc]  Rust for native builds, rapidr on PATH");
            eprintln!("  rapidr ide [file.rr]                             The IDE");
            eprintln!("  rapidr notices [<os>-<arch>|web|tools-<os>] [-o FILE]  The third-party notices builds carry");
            eprintln!("  rapidr lang export --json|--prompt|--vscode|--web-ide|--manual|--all  What the language registry generates");
            eprintln!("  rapidr lang conformance <dir> [--target desktop|web]  The registry's conformance programs");
            eprintln!("  rapidr about");
            eprintln!("  rapidr [--release|--debug] [--web] [--interp] <file.rr>  Build source file");
            eprintln!("  rapidr parse <file.rr>");
            eprintln!("  rapidr preprocess <file.rr>");
            eprintln!("  rapidr lex <file.rr>");
            eprintln!("  rapidr codegen <file.rr> [output_dir]");
            eprintln!("  rapidr build <file.rr> [output_dir] [--release|-r] [--debug|-d] [--web|-w] [--interp|-i] [--target <os>-<arch>]");
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

    // RapidR's home: the runtime crates and the lockfile (home.rs)
    let workspace_root = Home::find().map(|h| h.root);

    // Preprocess to detect $APPTYPE
    let pre = preprocess_file(path, PreprocessOptions::default()).ok();
    let app_type = pre.as_ref().and_then(|r| r.app_type.clone());
    let resources = match pre.as_ref().map(resource_files).transpose() {
        Ok(r) => r.unwrap_or_default(),
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };

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

    // The same checks as the bytecode compiler (unknown SUBs, missing
    // labels, …), with positions, so both backends reject the same programs.
    if let Err(errors) = compile_to_bytecode(path) {
        let native: Vec<&str> = errors.lines().filter(|l| rapidr_bcgen::error_applies_to_native_builds(l)).collect();
        if !native.is_empty() {
            eprintln!("{}", native.join("\n"));
            return ExitCode::from(1);
        }
    }

    // Generate Rust source
    let rust_source = rapidr_codegen_rust::generate_with_resources(&program, target, &resources);
    let cargo_toml = if target == AppTarget::Web {
        rapidr_codegen_rust::generate_cargo_toml_web(stem, &runtime_path.to_string_lossy())
    } else {
        rapidr_codegen_rust::generate_cargo_toml(stem, &runtime_path.to_string_lossy())
    };
    // (the workspace's lockfile, so wgpu, vello, winit — and on the web the
    // UI kernel's vello_cpu and parley — are the versions RapidR is tested
    // with, not whatever is newest)
    if let Some(lock) = workspace_root.as_ref().map(|r| r.join("Cargo.lock")).filter(|l| l.exists()) {
        if fs::create_dir_all(&out_dir).is_ok() {
            if let Err(e) = fs::copy(&lock, out_dir.join("Cargo.lock")) {
                eprintln!("Warning: could not copy {}: {e}", lock.display());
            }
        }
    }

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
    target: Option<String>,
) -> ExitCode {
    // Detect web target from $APPTYPE or --web flag
    let app_type = preprocess_file(path, PreprocessOptions::default())
        .ok()
        .and_then(|r| r.app_type);
    let is_web = web || app_type.as_deref() == Some("WEB");

    // Native means compiled: what the Rust backend can't compile yet is an
    // error, never a silent switch to the interpreter.
    if !interp {
        if let Some(gap) = parser_parse_file(path).ok().as_ref().and_then(rapidr_codegen_rust::native_gap) {
            eprintln!("{path}: error: {gap}, which native builds don't compile yet. Run it with the interpreter (rapidr build-bc / run-bc, or --interp).");
            return ExitCode::from(1);
        }
    }

    if interp {
        // Bytecode pipeline: skip Rust codegen entirely.
        return if is_web {
            build_interp_web(path, output_dir)
        } else {
            build_interp_desktop(path, output_dir, release, target)
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
    let mut cargo = match cargo_for_programs() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    // macOS: a release build is universal (arm64 + x86_64) when Rust has
    // both targets — what's shipped runs on every Mac —, for macOS from
    // DEPLOYMENT_TARGET on (macos.rs); a debug build is this Mac's only (half
    // the build time while developing)
    let universal = release && cfg!(target_os = "macos") && macos::rust_has_both_targets(home::rust_command("rustc"));
    if cfg!(target_os = "macos") {
        if env::var_os("MACOSX_DEPLOYMENT_TARGET").is_none() {
            cargo.env("MACOSX_DEPLOYMENT_TARGET", macos::DEPLOYMENT_TARGET);
        }
        if universal {
            for t in macos::TRIPLES {
                cargo_args.extend(["--target", t]);
            }
        } else if release {
            println!("(this Mac's architecture only: `rustup target add aarch64-apple-darwin x86_64-apple-darwin` makes universal executables)");
        }
    }
    let status = cargo.args(&cargo_args).current_dir(out_dir).status();

    match status {
        Ok(s) if s.success() => {
            // Copy the built binary to the same directory as the .rr source
            let binary_name = rapidr_codegen_rust::crate_name(stem);
            let target_root = match std::env::var_os("CARGO_TARGET_DIR") {
                Some(p) => PathBuf::from(p),
                None => out_dir.join("target"),
            };
            // (`.exe` on Windows)
            let exe = std::env::consts::EXE_SUFFIX;
            let mut built_binary = target_root.join(profile).join(format!("{binary_name}{exe}"));
            let dest_dir = source_path.parent().unwrap_or(Path::new("."));
            let dest_binary = dest_dir.join(format!("{stem}{exe}"));
            if universal {
                // (the two slices made one: lipo, which macOS' command line tools have —
                // the linker Rust uses comes with them)
                built_binary = target_root.join(format!("{binary_name}-universal"));
                let slices: Vec<PathBuf> = macos::TRIPLES.iter().map(|t| target_root.join(t).join(profile).join(&binary_name)).collect();
                let lipo = process::Command::new("lipo").arg("-create").args(&slices).arg("-output").arg(&built_binary).status();
                if !lipo.is_ok_and(|s| s.success()) {
                    eprintln!("lipo -create failed: the slices are in {}", target_root.display());
                    return ExitCode::from(1);
                }
            }

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
            // The open-source notices the program ships with (notices.rs)
            match notices::write(dest_dir, &notices::Kind::Desktop(home::host_target())) {
                Ok(p) => println!("Notices: {}", p.display()),
                Err(e) => {
                    eprintln!("{}", notices::missing(&e));
                    return ExitCode::from(1);
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
            eprintln!("Failed to run cargo: {e}\nNative builds compile with Rust: `rapidr setup` installs it.");
            ExitCode::from(1)
        }
    }
}

/// `cargo` for building a program against RapidR's runtime. An install
/// builds with its vendored crates, offline (home.rs); a runtime-only
/// install builds none.
fn cargo_for_programs() -> Result<process::Command, String> {
    // (an install: its own exact toolchain, never the user's default)
    let mut cargo = home::rust_command("cargo");
    let Some(home) = Home::find() else { return Ok(cargo) };
    if !home.can_build() {
        return Err("This is the RapidR Runtime: it runs programs (`rapidr run`) and builds none. Native and web builds need the RapidR SDK.".into());
    }
    if home.release.is_some() {
        let vendor = home.root.join("vendor");
        cargo
            .arg("--offline")
            .arg("--config")
            .arg("source.crates-io.replace-with='vendored-sources'")
            .arg("--config")
            .arg(format!("source.vendored-sources.directory='{}'", vendor.display()));
    }
    // Windows: Rust's gnullvm toolchain, linked by the LLVM-MinGW RapidR
    // ships — no Visual Studio (RAPIDR_TOOLCHAIN=msvc: Rust's default instead).
    if let Some(tc) = home.windows_toolchain() {
        let triple = home::windows_gnullvm_triple();
        let env_triple = triple.replace('-', "_");
        let bin = tc.join("bin");
        let clang = bin.join(format!("{}-w64-mingw32-clang.exe", env::consts::ARCH));
        cargo
            .env(format!("CARGO_TARGET_{}_LINKER", env_triple.to_uppercase()), &clang)
            // (libunwind and the mingw-w64 runtime linked in: an executable
            // that needs no DLL beside it)
            .env(format!("CARGO_TARGET_{}_RUSTFLAGS", env_triple.to_uppercase()), "-C target-feature=+crt-static")
            .env(format!("CC_{env_triple}"), &clang)
            .env(format!("AR_{env_triple}"), bin.join("llvm-ar.exe"));
        let path = env::var_os("PATH").unwrap_or_default();
        let paths = std::iter::once(bin).chain(env::split_paths(&path));
        if let Ok(joined) = env::join_paths(paths) {
            cargo.env("PATH", joined);
        }
    }
    Ok(cargo)
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
    let mut cargo = match cargo_for_programs() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    cargo.args(&cargo_args).current_dir(out_dir);
    // (wasm SIMD: the UI kernel's CPU renderer on simd128, as the web
    // runtime's own build — every 2026 browser has it)
    if env::var_os("RUSTFLAGS").is_none() {
        cargo.env("RUSTFLAGS", "-C target-feature=+simd128");
    }
    // SQLite's C sources go into the wasm (RSQLITE), compiled as the
    // workspace compiles them (its .cargo/config.toml, wherever the program
    // is), archived by llvm-ar or without one by the system's ar
    // (tools/wasm-ar.sh).
    if let Some(root) = Home::find().map(|h| h.root) {
        let config = root.join(".cargo/config.toml");
        if config.exists() {
            cargo.arg("--config").arg(config);
        }
        let ar = root.join("tools/wasm-ar.sh");
        if cfg!(unix) && ar.exists() && env::var_os("AR_wasm32_unknown_unknown").is_none() {
            cargo.env("AR_wasm32_unknown_unknown", ar);
        }
    }
    let status = cargo.status();

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
        .join(format!("{}.wasm", rapidr_codegen_rust::crate_name(stem).replace('-', "_")));

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
            eprintln!("  Install with: cargo install wasm-bindgen-cli --version 0.2.129");
            return ExitCode::from(1);
        }
        _ => {}
    }

    // Step 3: Generate index.html
    let wasm_module = rapidr_codegen_rust::crate_name(stem).replace('-', "_");
    let assets = collect_assets(source_path);
    if !assets.is_empty() {
        println!("Embedding {} asset(s) in index.html...", assets.len());
        for name in assets.keys() {
            println!("  - {}", name);
        }
    }
    let html = generate_html_shell(stem, &wasm_module, &assets);
    if let Err(e) = fs::write(web_out.join("index.html"), &html) {
        eprintln!("Cannot write index.html: {e}");
        return ExitCode::from(1);
    }
    // (the fallback fonts beside the page: loaded as its text needs them)
    let fonts = fallback_fonts_dir().map(|d| fallback_fonts(&d)).unwrap_or_default();
    if !fonts.is_empty() {
        let dir = web_out.join("fonts");
        if let Err(e) = fs::create_dir_all(&dir).and_then(|_| fonts.iter().try_for_each(|(n, data)| fs::write(dir.join(n), data))) {
            eprintln!("Cannot write the fallback fonts: {e}");
            return ExitCode::from(1);
        }
    }
    // The open-source notices the page ships with (index.html links them)
    if let Err(e) = notices::write(&web_out, &notices::Kind::Web) {
        eprintln!("{}", notices::missing(&e));
        return ExitCode::from(1);
    }

    println!("Web build: {}", web_out.display());
    println!("  {}/index.html", web_out.display());
    println!("  {}/{}", web_out.display(), notices::FILE_NAME);
    println!("  {}/{}_bg.wasm", web_out.display(), wasm_module);
    println!("  {}/{}.js", web_out.display(), wasm_module);
    println!("\nServe with: python3 -m http.server -d {} 8080", web_out.display());
    println!("Build succeeded!");
    ExitCode::SUCCESS
}

fn generate_html_shell(title: &str, wasm_module: &str, assets: &std::collections::HashMap<String, String>) -> String {
    let css = rapidr_webbundle::PAGE_CSS;
    let mut assets_script = String::new();
    if !assets.is_empty() {
        assets_script.push_str("  <script>\n    window.__rapidr_assets = {\n");
        for (name, base64) in assets {
            let escaped_name = name.replace('"', "\\\"");
            assets_script.push_str(&format!("      \"{}\": \"{}\",\n", escaped_name, base64));
            assets_script.push_str(&format!("      \"assets/{}\": \"{}\",\n", escaped_name, base64));
        }
        assets_script.push_str("    };\n  </script>\n");
    }
    let notices = notices::html_head_lines();
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{title}</title>
{notices}  <style>{css}</style>
{assets_script}</head>
<body>
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

// ---------------- Bytecode (rapidrintr) ----------------

/// Preprocess → lex → parse → bytecode, keeping the preprocessed source so
/// every error carries `file:line:col`. All errors are reported, one per line.
fn compile_to_bytecode(path: &str) -> Result<rapidr_bcgen::Compiled, String> {
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

fn build_bytecode_file(path: &str, output: Option<String>) -> ExitCode {
    let compiled = match compile_to_bytecode(path) {
        Ok(c) => c,
        Err(e) => { eprintln!("{e}"); return ExitCode::from(1); }
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
    // program that calls `Form.ShowModal` from MAIN (the VM serves the
    // modal's wait a pump step at a time, so events fire while we are
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
    let compiled = match compile_to_bytecode(path) {
        Ok(c) => c,
        Err(e) => { eprintln!("{e}"); return ExitCode::from(1); }
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

    // 3. Build the bundle, with the open-source notices it ships with.
    let notices_text = match notices::text(&notices::Kind::Web) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}", notices::missing(&e));
            return ExitCode::from(1);
        }
    };
    let assets = collect_assets(Path::new(path));
    if !assets.is_empty() {
        println!("Embedding {} asset(s) in web bundle...", assets.len());
        for name in assets.keys() {
            println!("  - {}", name);
        }
    }
    let bundle = match rapidr_webbundle::build_bundle(&rapidr_webbundle::BundleInputs {
        project_name: &stem,
        rrbc: &rrbc,
        rapidrintr_wasm: &wasm_bytes,
        rapidrintr_js: &js_text,
        title: None,
        assets: Some(&assets),
        fonts: &fallback_fonts(&wasm_p.parent().unwrap_or(Path::new(".")).join("fonts")),
        notices: &notices_text,
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

/// The fallback fonts' files in `dir` (`index.json`, the chunks,
/// `OFL.txt`: tools/fonts.py's, beside the web runtime — an install's
/// `lib/rapidr/web/fonts`, a checkout's `target/web/fonts`), by name; none
/// (with a warning) when they aren't there: the page then shows characters
/// the built-in fonts lack as boxes.
fn fallback_fonts(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.ends_with(".otf") || name.ends_with(".ttf") || name == "index.json" || name == "OFL.txt" {
                if let Ok(data) = fs::read(e.path()) {
                    out.push((name, data));
                }
            }
        }
    }
    if !out.iter().any(|(n, _)| n == "index.json") {
        eprintln!("warning: no fallback fonts in {} (tools/build_web_artifacts.sh makes them): characters the built-in fonts lack will show as boxes", dir.display());
        return Vec::new();
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Where the web runtime's fallback fonts are: an install's
/// `lib/rapidr/web/fonts`, a checkout's `target/web/fonts`.
fn fallback_fonts_dir() -> Option<PathBuf> {
    match Home::find() {
        Some(home) if home.release.is_some() => Some(home.root.join("web").join("fonts")),
        Some(home) => Some(home.root.join("target").join("web").join("fonts")),
        None => None,
    }
}

/// Default lookup for the rapidrintr wasm/js artifacts produced by
/// `wasm-pack build interpreter/rapidr-vm-host-web --target web`: an
/// install has them in its home's `web/` (home.rs); in a checkout, a few
/// well-known locations relative to the current dir.
fn locate_rapidrintr_artifacts() -> Option<(PathBuf, PathBuf)> {
    let candidates: Vec<PathBuf> = match Home::find() {
        Some(home) if home.release.is_some() => vec![home.root.join("web")],
        // (a checkout: from the current directory, then the checkout's root)
        home => {
            let rel = ["target/web", "target/web-bundle", "interpreter/rapidr-vm-host-web/pkg", "pkg"].map(PathBuf::from);
            let mut c = rel.to_vec();
            if let Some(h) = home {
                c.extend(rel.iter().map(|r| h.root.join(r)));
            }
            c
        }
    };
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
    target: Option<String>,
) -> ExitCode {
    // 1. Compile source → bytecode.
    let compiled = match compile_to_bytecode(path) {
        Ok(c) => c,
        Err(e) => { eprintln!("{e}"); return ExitCode::from(1); }
    };
    for w in &compiled.warnings { eprintln!("warning: {w}"); }
    let rrbc = compiled.module.to_bytes();

    // 2. Locate (or build) the runner stub: this machine's, or the
    //    `--target` an install ships. A windowed program for Windows starts
    //    from the windowed runner (no console window opens with it).
    let target = target.unwrap_or_else(home::host_target);
    let windowed = target.starts_with("windows-") && !compiled.module.app_type.wants_console();
    let stub = match locate_or_build_stub(release, &target, windowed).and_then(|path| {
        let bytes = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        // (`--target macos-arm64` / `macos-x86_64`: that slice of the universal runner)
        match macos::slice_of(&target) {
            Some(cpu) if bytes.starts_with(&[0xCA, 0xFE, 0xBA]) => macos::thin(&bytes, cpu).map_err(|e| format!("{}: {e}", path.display())),
            _ => Ok(bytes),
        }
    }) {
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
    let dest = dest_dir.join(format!("{stem}{}", home::exe_suffix(&target)));

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
    // 5. The open-source notices it ships with (the same file as a native
    //    build's for this target: notices.rs).
    match notices::write(&dest_dir, &notices::Kind::Desktop(target)) {
        Ok(p) => println!("Notices: {}", p.display()),
        Err(e) => {
            eprintln!("{}", notices::missing(&e));
            return ExitCode::from(1);
        }
    }
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
fn attach_payload(stub: &[u8], rrbc: &[u8], dest: &Path) -> Result<(), String> {
    fs::write(dest, stub).map_err(|e| format!("write {}: {e}", dest.display()))?;

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

/// The runner stub `--interp` executables start from: `rapidrintr-runner`,
/// or `rapidrintr-runnerw` (Windows' windowed one, `windowed`).
///
/// An install ships one per target (home.rs: `runners/<os>-<arch>/`). A
/// checkout builds this machine's with `cargo build -p rapidr-runner-stub`
/// every time (a quick no-op when it's up to date), so the runner never lags
/// behind the CLI; an existing runner in `target/runner`, `target/release`
/// or `target/debug` is used only if cargo can't run.
fn locate_or_build_stub(release: bool, target: &str, windowed: bool) -> Result<PathBuf, String> {
    let name = if windowed { "rapidrintr-runnerw" } else { "rapidrintr-runner" };
    let exe_name = format!("{name}{}", home::exe_suffix(target));
    let home = Home::find();
    if let Some(home) = home.as_ref().filter(|h| h.release.is_some()) {
        // (one universal runner for macOS: a thin --target is a slice of it)
        let shipped_as = if macos::slice_of(target).is_some() { "macos" } else { target };
        let path = home.runner(shipped_as, name);
        if path.is_file() {
            return Ok(path);
        }
        let shipped = home.runner_targets();
        return Err(if shipped.is_empty() {
            "This is the RapidR Runtime: it runs programs (`rapidr run`) and builds none. Executables need the RapidR SDK.".to_string()
        } else {
            format!("no runner for {target} in this install ({}); it has: {}", home.root.display(), shipped.join(", "))
        });
    }
    if target != home::host_target() && target != home::host_arch_target() {
        return Err(format!("--target {target}: a source checkout builds this machine's runner only ({}); an installed RapidR ships the others", home::host_arch_target()));
    }
    // Release: the stripped `runner` profile (Cargo.toml).
    let preferred = if release { "runner" } else { "debug" };

    let mut args = vec!["build", "--quiet", "-p", "rapidr-runner-stub", "--bin", name];
    if release { args.extend(["--profile", "runner"]); }
    // Built in the RapidR workspace, wherever the program being built is.
    let root = home.map(|h| h.root).unwrap_or_else(|| PathBuf::from("."));
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    // Where cargo puts it: CARGO_TARGET_DIR when set (relative to where
    // rapidr was run), else the workspace's target/.
    let target_dir = std::env::var_os("CARGO_TARGET_DIR").map(|t| cwd.join(t)).unwrap_or_else(|| root.join("target"));
    let built = process::Command::new(home::rust_tool("cargo")).args(&args).current_dir(&root).env("CARGO_TARGET_DIR", &target_dir).status();
    let path = target_dir.join(preferred).join(&exe_name);
    match built {
        Ok(status) if status.success() && path.exists() => return Ok(path),
        Ok(status) => eprintln!("warning: cargo build rapidr-runner-stub failed ({status}); using an existing runner if there is one"),
        Err(e) => eprintln!("warning: can't run cargo ({e}); using an existing runner if there is one"),
    }
    for profile in [preferred, if release { "release" } else { "runner" }] {
        let candidate = target_dir.join(profile).join(&exe_name);
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(format!("could not build or find {exe_name} (cargo build -p rapidr-runner-stub)"))
}

fn collect_assets(source_path: &Path) -> std::collections::HashMap<String, String> {
    use base64::Engine;
    let mut assets = std::collections::HashMap::new();
    let source_dir = source_path.parent().unwrap_or(Path::new("."));
    
    let entries = match fs::read_dir(source_dir) {
        Ok(e) => e,
        Err(_) => return assets,
    };

    let asset_exts = ["csv", "db", "sqlite", "png", "jpg", "jpeg", "gif", "bmp", "txt", "wav", "mp3"];

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext_lower = ext.to_lowercase();
                if asset_exts.contains(&ext_lower.as_str()) {
                    if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                        if let Ok(bytes) = fs::read(&path) {
                            let mime = mime_type_from_ext(&ext_lower);
                            let encoded_base64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                            let data_url = format!("data:{};base64,{}", mime, encoded_base64);
                            assets.insert(filename.to_string(), data_url);
                        }
                    }
                }
            }
        }
    }
    assets
}

fn mime_type_from_ext(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "csv" => "text/csv",
        "txt" => "text/plain",
        "wav" => "audio/wav",
        "mp3" => "audio/mpeg",
        _ => "application/octet-stream",
    }
}
