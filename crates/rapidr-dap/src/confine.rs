//! What a launch request may ask of `rapidr dap` (docs/security-audit.md
//! SEC-18). A launch configuration is often the project's own file
//! (`.vscode/launch.json`), so it's data, not the user's word: it names a
//! RapidR program to debug — never any other file — and the environment the
//! program sees, never one that changes how a process loads code.

use std::collections::BTreeMap;
use std::path::Path;

/// The files a launch may name as its `program`: what `rapidr run` runs.
pub const PROGRAM_EXTENSIONS: &[&str] = &["bas", "rr", "rqw", "rqb", "rq", "inc", "rrbc"];

/// Whether `program` is a RapidR program (by its extension) that exists.
pub fn check_program(program: &Path) -> Result<(), String> {
    let ext = program.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).unwrap_or_default();
    if !PROGRAM_EXTENSIONS.contains(&ext.as_str()) {
        return Err(format!("{}: not a RapidR program (rapidr dap debugs .bas, .rr, .rqw, .rqb, .rq, .inc and .rrbc files)", program.display()));
    }
    if !program.is_file() {
        return Err(format!("{}: no such file", program.display()));
    }
    Ok(())
}

/// Variables never passed to a debuggee: they make a process load code
/// other than its own (the dynamic loaders' `LD_` / `DYLD_` / `_RLD`
/// families, AIX's and HP-UX's library paths, glibc's gconv / locale
/// modules, Windows' DLL search through `PATH`, its compatibility layer).
const LOADER_PREFIXES: &[&str] = &["LD_", "DYLD_", "_RLD"];
const LOADER_NAMES: &[&str] = &["LIBPATH", "SHLIB_PATH", "GCONV_PATH", "LOCPATH", "PATH", "PATHEXT", "COMSPEC", "__COMPAT_LAYER"];

/// Whether a launch's environment variable may reach the program: a plain
/// name (letters, digits, `_`; not starting with a digit), not one of the
/// loaders' ([`LOADER_PREFIXES`], [`LOADER_NAMES`], in any case).
pub fn env_allowed(name: &str) -> bool {
    let plain = !name.is_empty() && !name.starts_with(|c: char| c.is_ascii_digit()) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    let upper = name.to_ascii_uppercase();
    plain && !LOADER_PREFIXES.iter().any(|p| upper.starts_with(p)) && !LOADER_NAMES.contains(&upper.as_str())
}

/// A launch's `env` split into what the program gets (a value set, or
/// `None`: removed) and the names refused (said in the debug console).
pub fn debuggee_env(env: &BTreeMap<String, Option<String>>) -> (Vec<(String, Option<String>)>, Vec<String>) {
    let mut passed = Vec::new();
    let mut refused = Vec::new();
    for (name, value) in env {
        if env_allowed(name) && value.as_deref().is_none_or(|v| !v.contains('\0')) {
            passed.push((name.clone(), value.clone()));
        } else {
            refused.push(name.clone());
        }
    }
    (passed, refused)
}

/// The security regressions (tests/security/, `tools/regress.sh security`).
#[cfg(test)]
#[path = "../../../tests/security/dap_launch_confinement.rs"]
mod security_regressions;
