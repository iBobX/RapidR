// The git commit RapidR was built from, for crash reports
// (`RAPIDR_COMMIT` when set — a release built from a source archive —,
// else `git rev-parse` when the source is a git checkout, else "unknown").
// A program built on a user's computer from RapidR's crates gets
// "unknown": no git is run outside a checkout.
use std::path::Path;
use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).current_dir(env!("CARGO_MANIFEST_DIR")).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string()).filter(|s| !s.is_empty())
}

fn main() {
    println!("cargo:rerun-if-env-changed=RAPIDR_COMMIT");
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    // (RapidR's own checkout only: its root two folders up — never another
    // repository a home's copy of the crates happens to sit in)
    let checkout = manifest.ancestors().nth(2).is_some_and(|root| root.join(".git").exists() && root.join("crates").join("rapidr-crash").is_dir());
    let commit = std::env::var("RAPIDR_COMMIT")
        .ok()
        .filter(|c| !c.trim().is_empty())
        .or_else(|| if checkout { git(&["rev-parse", "--short=10", "HEAD"]) } else { None });
    if checkout {
        // (a new commit or checkout: this worktree's HEAD log changes)
        if let Some(log) = git(&["rev-parse", "--git-path", "logs/HEAD"]) {
            let p = if Path::new(&log).is_absolute() { Path::new(&log).to_path_buf() } else { manifest.join(&log) };
            if p.exists() {
                println!("cargo:rerun-if-changed={}", p.display());
            }
        }
    }
    println!("cargo:rustc-env=RAPIDR_BUILD_COMMIT={}", commit.unwrap_or_else(|| "unknown".into()));
}
