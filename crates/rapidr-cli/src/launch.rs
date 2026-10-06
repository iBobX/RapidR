//! The RapidR Runtime running a program file: `rapidr run`, `rapidr open`
//! (what opening a `.rrbc`, `.rr` or `.bas` from the desktop does),
//! `#!/usr/bin/env rapidr` scripts and `rapidr info`.
//!
//! - A `.rrbc` runs as it is, once its header says this runtime can run it
//!   (else: which RapidR Runtime it needs, and where to get it); a source
//!   file is compiled in memory first.
//! - A downloaded file (macOS' quarantine, Windows' Mark of the Web, the
//!   origin browsers record on Linux) asks once before it runs: [`trust`].
//! - Opened from the desktop, a console program (`$APPTYPE CONSOLE` / CGI)
//!   gets a terminal. On Windows `rapidrw.exe` (the windowed launcher) gives
//!   it a console instead.
//! - The program is the file: Application.ExeName / Path name it and
//!   COMMAND$ is its arguments, as when it is built.

use std::env;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use rapidr_bytecode::Header;

/// How the program was started.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum From {
    /// `rapidr run` and scripts: from a command line.
    Command,
    /// `rapidr open`: the desktop (a double click, Open With).
    Desktop,
    /// RapidR's own program (`rapidr ide`): not asked about.
    Own,
}

/// The program's bytecode: a `.rrbc` as it is (when this runtime runs it),
/// a source file compiled.
pub fn load(path: &str) -> Result<Vec<u8>, String> {
    let bytes = fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    if bytes.starts_with(rapidr_bytecode::MAGIC) {
        Header::read(&bytes).and_then(|h| h.check_runtime()).map_err(|e| format!("{path}: {e}"))?;
        return Ok(bytes);
    }
    let compiled = crate::compile_to_bytecode(path)?;
    for w in &compiled.warnings {
        eprintln!("warning: {w}");
    }
    Ok(compiled.module.to_bytes())
}

/// Run a program file with `args`.
pub fn run(path: &str, args: Vec<String>, from: From) -> ExitCode {
    match start(path, args, from) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            // (opened from the desktop: nobody reads stderr)
            if from == From::Desktop && !has_terminal() {
                let _ = dialog(&format!("SHOWMESSAGE {}\n", basic_string(&e)));
            }
            ExitCode::from(1)
        }
    }
}

fn start(path: &str, args: Vec<String>, from: From) -> Result<(), String> {
    if !Path::new(path).is_file() {
        return Err(format!("{path}: no such file"));
    }
    if from != From::Own {
        trust::check(Path::new(path))?;
    }
    let bytes = load(path)?;
    let app_type = Header::read(&bytes).map(|h| h.app_type).unwrap_or_default();
    if from == From::Desktop && app_type.wants_console() && !has_terminal() {
        return open_in_terminal(path, &args);
    }
    let program = crate::home::canonical(path).map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|_| path.to_string());
    rapidr_vm_host_native::set_program(&program, args);
    // (the runtime running it and the system's temporary folder, for a
    // program that runs others: the IDE)
    if let Ok(exe) = env::current_exe() {
        env::set_var("RAPIDR_RUNTIME", exe);
    }
    env::set_var("RAPIDR_TEMP", env::temp_dir());
    rapidr_vm_host_native::run_bytes(&bytes)
}

/// `rapidr run --session <file> [args]`: the program under RapidR Studio
/// (or any client of the session protocol, rapidr-session): requests on
/// standard input, events framed on standard output among what it prints.
/// It waits for `start` (after the breakpoints) before it runs. A program
/// that doesn't compile says why on standard error and exits with 1.
pub fn run_session(path: &str, args: Vec<String>) -> ExitCode {
    if !Path::new(path).is_file() {
        eprintln!("{path}: no such file");
        return ExitCode::from(1);
    }
    if let Err(e) = trust::check(Path::new(path)) {
        eprintln!("{e}");
        return ExitCode::from(1);
    }
    let bytes = match load(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    let program = crate::home::canonical(path).map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|_| path.to_string());
    rapidr_vm_host_native::set_program(&program, args);
    if let Ok(exe) = env::current_exe() {
        env::set_var("RAPIDR_RUNTIME", exe);
    }
    env::set_var("RAPIDR_TEMP", env::temp_dir());
    match rapidr_vm_host_native::session::run_session(&bytes, path) {
        Ok(code) => ExitCode::from(code.clamp(0, 255) as u8),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

/// `rapidr ide [file]`: RapidR's IDE — an install's `ide/rapidr-ide.rrbc`,
/// a checkout's `examples/ide.rr`.
pub fn ide(args: Vec<String>) -> ExitCode {
    let Some(home) = crate::home::Home::find() else {
        eprintln!("RapidR's home was not found (set RAPIDR_HOME)");
        return ExitCode::from(1);
    };
    let ide = match &home.release {
        Some(_) => home.root.join("ide").join("rapidr-ide.rrbc"),
        None => home.root.join("examples").join("ide.rr"),
    };
    if !ide.is_file() {
        eprintln!("{}: not found — the IDE comes with the RapidR SDK, not the Runtime", ide.display());
        return ExitCode::from(1);
    }
    run(&ide.to_string_lossy(), args, From::Own)
}

/// `rapidr info <file>`: what the runtime knows of a program before it
/// runs it (`rapidrw.exe` reads `apptype`).
pub fn info(path: &str) -> ExitCode {
    let bytes = match fs::read(path) {
        Ok(b) if b.starts_with(rapidr_bytecode::MAGIC) => b,
        Ok(_) => match crate::compile_to_bytecode(path) {
            Ok(c) => c.module.to_bytes(),
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(1);
            }
        },
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::from(1);
        }
    };
    match Header::read(&bytes) {
        Ok(h) => {
            let [a, b, c] = h.min_runtime;
            println!("apptype: {}", h.app_type.name());
            println!("format: {}", h.format);
            println!("needs-runtime: {a}.{b}.{c}");
            println!("runtime: {}", rapidr_bytecode::RUNTIME_VERSION);
            println!("runs: {}", if h.check_runtime().is_ok() { "yes" } else { "no" });
            println!("downloaded: {}", if trust::downloaded(Path::new(path)).is_some() { "yes" } else { "no" });
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{path}: {e}");
            ExitCode::from(1)
        }
    }
}

/// A terminal to ask on and print to.
fn has_terminal() -> bool {
    std::io::stdin().is_terminal() || std::io::stdout().is_terminal()
}

/// `s` quoted for a POSIX shell.
#[cfg(unix)]
fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// `rapidr run <program> <args>` in a new terminal window, which stays open
/// after the program ends so its output can be read.
#[cfg(unix)]
fn open_in_terminal(path: &str, args: &[String]) -> Result<(), String> {
    let exe = env::current_exe().map_err(|e| e.to_string())?;
    let program = crate::home::canonical(path).map_err(|e| format!("{path}: {e}"))?;
    let mut line = format!("{} run {}", sh_quote(&exe.to_string_lossy()), sh_quote(&program.to_string_lossy()));
    for a in args {
        line.push(' ');
        line.push_str(&sh_quote(a));
    }
    // The terminal the user chose ($TERMINAL, run with `-e`), else the
    // system's: macOS' Terminal, Linux' x-terminal-emulator or a known one.
    let keep_open = format!("{line}; printf '\\n[press Enter to close] '; read _");
    let chosen = env::var("TERMINAL").ok().filter(|t| !t.is_empty());
    if chosen.is_none() && cfg!(target_os = "macos") {
        // Terminal runs a `.command` file it is given (and keeps the window
        // open, "[Process completed]"); the file removes itself.
        let script = env::temp_dir().join(format!("rapidr-{}.command", std::process::id()));
        fs::write(&script, format!("#!/bin/sh\nrm -f \"$0\"\ncd {}\n{line}\n", sh_quote(&program.parent().unwrap_or(Path::new("/")).to_string_lossy())))
            .map_err(|e| format!("{}: {e}", script.display()))?;
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())?;
        let status = Command::new("open").arg("-a").arg("Terminal").arg(&script).status().map_err(|e| format!("open -a Terminal: {e}"))?;
        return if status.success() { Ok(()) } else { Err(format!("open -a Terminal failed ({status})")) };
    }
    let candidates: Vec<(String, &str)> = match chosen {
        Some(t) => vec![(t, "-e")],
        None => [("x-terminal-emulator", "-e"), ("gnome-terminal", "--"), ("konsole", "-e"), ("xfce4-terminal", "-x"), ("xterm", "-e")]
            .into_iter()
            .map(|(t, flag)| (t.to_string(), flag))
            .collect(),
    };
    for (term, flag) in candidates {
        match Command::new(&term).arg(flag).arg("sh").arg("-c").arg(&keep_open).spawn() {
            Ok(_) => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(format!("{term}: {e}")),
        }
    }
    Err(format!("{path} is a console program and no terminal was found to run it in (set TERMINAL, or run: rapidr run {path})"))
}

/// Windows: `rapidrw.exe` gives a console program its console.
#[cfg(not(unix))]
fn open_in_terminal(path: &str, args: &[String]) -> Result<(), String> {
    let _ = args;
    Err(format!("{path} is a console program: open it with rapidrw.exe, or run it from a console (rapidr run {path})"))
}

/// A small BASIC program run in a child process (a process has one event
/// loop, and the program that runs next needs it): the runtime's own
/// dialogs — the question before a downloaded file runs, `rapidr about`.
/// Under the GUI tests' hooks (RAPIDR_CAPTURE + RAPIDR_TEST_MESSAGE_DIALOG)
/// they answer headless.
pub fn dialog(source: &str) -> Result<String, String> {
    let file = env::temp_dir().join(format!("rapidr-dialog-{}.bas", std::process::id()));
    fs::write(&file, source).map_err(|e| format!("{}: {e}", file.display()))?;
    let out = Command::new(env::current_exe().map_err(|e| e.to_string())?)
        .arg("__dialog")
        .arg(&file)
        .stdin(std::process::Stdio::null())
        .output();
    let _ = fs::remove_file(&file);
    let out = out.map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// `rapidr __dialog <file.bas>` (the child of [`dialog`]): compile and run.
pub fn run_dialog(path: &str) -> ExitCode {
    rapidr_vm_host_native::set_program(path, Vec::new());
    match crate::compile_to_bytecode(path).map(|c| c.module.to_bytes()).and_then(|b| rapidr_vm_host_native::run_bytes(&b)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

/// A BASIC string literal of `text` (no `"` inside one: CHR$(34)).
fn basic_string(text: &str) -> String {
    let parts: Vec<String> = text.split('\n').map(|line| format!("\"{}\"", line.replace('"', "\" + CHR$(34) + \""))).collect();
    parts.join(" + CHR$(10) + ")
}

/// `rapidr about`: the runtime's version, in a window when there's no
/// terminal (opening the RapidR Runtime app itself).
pub fn about() -> ExitCode {
    let home = crate::home::Home::find();
    let text = format!(
        "RapidR Runtime {}\n\nRuns RapidR and RapidQ programs: open a .rrbc, .rr or .bas file.\n{}\n\nCopyright (c) 2025-2026 Ruta Internet SRL. MIT licence, with open-source\ncomponents (THIRD-PARTY-NOTICES.txt).\nRapidR is compatible with RapidQ; it is not affiliated with RapidQ's author\nor any vendor it names. Legal: https://github.com/iBobX/RapidR/blob/main/LEGAL.md",
        rapidr_bytecode::RUNTIME_VERSION,
        rapidr_bytecode::RELEASES_URL
    );
    if has_terminal() {
        println!("{text}");
        if let Some(h) = home {
            println!("home: {}", h.root.display());
        }
        return ExitCode::SUCCESS;
    }
    match dialog(&format!("SHOWMESSAGE {}\n", basic_string(&text))) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

/// Files that came from the internet ask before they run, once: the answer
/// is kept per file content (SHA-256) in the user's RapidR folder
/// (`trusted-files.txt`; `RAPIDR_CONFIG_DIR` puts it elsewhere).
pub mod trust {
    use super::*;
    use sha2::{Digest, Sha256};

    /// Where it came from, when it was downloaded.
    pub fn downloaded(path: &Path) -> Option<String> {
        #[cfg(target_os = "macos")]
        {
            xattr(path, "com.apple.quarantine").map(|q| q.split(';').nth(2).filter(|a| !a.is_empty()).unwrap_or("the internet").to_string())
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            // (what Chromium and Firefox record on a download)
            xattr(path, "user.xdg.origin.url")
        }
        #[cfg(windows)]
        {
            let mut stream = path.as_os_str().to_owned();
            stream.push(":Zone.Identifier");
            let text = fs::read_to_string(PathBuf::from(stream)).ok()?;
            let zone = text.lines().find_map(|l| l.trim().strip_prefix("ZoneId=")).and_then(|z| z.trim().parse::<u32>().ok())?;
            // 3: the internet, 4: restricted sites
            (zone >= 3).then(|| {
                text.lines().find_map(|l| l.trim().strip_prefix("HostUrl=")).map(str::to_string).unwrap_or_else(|| "the internet".into())
            })
        }
    }

    #[cfg(unix)]
    fn xattr(path: &Path, name: &str) -> Option<String> {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;
        let p = CString::new(path.as_os_str().as_bytes()).ok()?;
        let n = CString::new(name).ok()?;
        let mut buf = vec![0u8; 4096];
        // SAFETY: both strings are NUL-terminated; the buffer's length is given.
        #[cfg(target_os = "macos")]
        let len = unsafe { libc::getxattr(p.as_ptr(), n.as_ptr(), buf.as_mut_ptr().cast(), buf.len(), 0, 0) };
        #[cfg(not(target_os = "macos"))]
        let len = unsafe { libc::getxattr(p.as_ptr(), n.as_ptr(), buf.as_mut_ptr().cast(), buf.len()) };
        if len < 0 {
            return None;
        }
        buf.truncate(len as usize);
        Some(String::from_utf8_lossy(&buf).into_owned())
    }

    /// The file holding the answers.
    pub fn store() -> Option<PathBuf> {
        let dir = env::var_os("RAPIDR_CONFIG_DIR").map(PathBuf::from).or_else(|| crate::home::config_dir().map(|d| d.join(if cfg!(target_os = "linux") { "rapidr" } else { "RapidR" })))?;
        Some(dir.join("trusted-files.txt"))
    }

    /// Ok to run: not downloaded, or the user said so (now or before).
    pub fn check(path: &Path) -> Result<(), String> {
        let Some(origin) = downloaded(path) else { return Ok(()) };
        let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let path = &crate::home::canonical(path).unwrap_or_else(|_| path.to_path_buf());
        let hash: String = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
        let store = store().ok_or("no folder for RapidR's settings (set RAPIDR_CONFIG_DIR)")?;
        let known = fs::read_to_string(&store).unwrap_or_default();
        let answer = match known.lines().find_map(|l| l.strip_prefix(&hash).and_then(|r| r.split_whitespace().next()).map(str::to_string)) {
            Some(a) => a,
            None => {
                let yes = ask(path, &origin)?;
                let a = if yes { "run" } else { "refuse" };
                if let Some(dir) = store.parent() {
                    fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
                }
                let line = format!("{hash} {a} {}\n", path.display());
                use std::io::Write;
                fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&store)
                    .and_then(|mut f| f.write_all(line.as_bytes()))
                    .map_err(|e| format!("{}: {e}", store.display()))?;
                a.to_string()
            }
        };
        if answer == "run" {
            Ok(())
        } else {
            Err(format!("{}: not run — it was downloaded and you chose not to run it (remove its line from {} to be asked again)", path.display(), store.display()))
        }
    }

    /// The user's answer: on the terminal when there is one, else in a window.
    fn ask(path: &Path, origin: &str) -> Result<bool, String> {
        let question = format!(
            "{} was downloaded from {origin}.\n\nPrograms from the internet can harm your computer. Run it only if you trust where it came from.\n\nRun it?",
            path.display()
        );
        if std::io::stdin().is_terminal() {
            use std::io::Write;
            eprint!("{question} [y/N] ");
            let _ = std::io::stderr().flush();
            let mut line = String::new();
            std::io::stdin().read_line(&mut line).map_err(|e| e.to_string())?;
            return Ok(matches!(line.trim().to_ascii_lowercase().as_str(), "y" | "yes"));
        }
        // MB_YESNO + MB_ICONEXCLAMATION + MB_DEFBUTTON2 (No is the default)
        let answer = dialog(&format!(
            "IF MESSAGEBOX({}, \"RapidR Runtime\", 4 + 48 + 256) = 6 THEN PRINT \"yes\" ELSE PRINT \"no\"\n",
            basic_string(&question)
        ))?;
        Ok(answer.lines().last() == Some("yes"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rapidr_bytecode::Module;

    #[test]
    fn basic_strings_carry_quotes_and_lines() {
        assert_eq!(basic_string("a \"b\"\nc"), "\"a \" + CHR$(34) + \"b\" + CHR$(34) + \"\" + CHR$(10) + \"c\"");
    }

    #[test]
    fn a_rrbc_too_new_says_which_runtime() {
        let dir = env::temp_dir().join(format!("rapidr-launch-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("new.rrbc");
        let mut bytes = Module::new().to_bytes();
        bytes[10..12].copy_from_slice(&99u16.to_le_bytes());
        fs::write(&file, &bytes).unwrap();
        let err = load(&file.to_string_lossy()).unwrap_err();
        assert!(err.contains("needs RapidR Runtime 99."), "{err}");
        fs::remove_dir_all(&dir).unwrap();
    }
}
