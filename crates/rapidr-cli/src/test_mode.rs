//! `rapidr dev test-mode`: the signed test mode's keys and tokens (SEC-24,
//! docs/security/test-mode.md).
//!
//!   rapidr dev test-mode new-key --out DIR
//!       a new Ed25519 key: DIR/test-mode.key (secret, 0600) and
//!       DIR/test-mode.pub (to copy to crates/rapidr-testmode/)
//!   rapidr dev test-mode issue --key FILE --days N [--out TOKEN] [--machine NAME | --this-machine]
//!       a token valid N days (at most a year), by default
//!       ~/.rapidr/test-mode.token (where the test harnesses look)
//!   rapidr dev test-mode status
//!       what this build trusts, and whether its hooks act now
//!
//! Neither file may go inside a Git work tree: the secret key never goes
//! near the repository, and a token is a key to the hooks for a year.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rapidr_testmode::{self as tm, issue, Mode};

pub fn command(args: &[String]) -> ExitCode {
    match (args.first().map(String::as_str), args.get(1).map(String::as_str)) {
        (Some("test-mode"), Some("new-key")) => new_key(&args[2..]),
        (Some("test-mode"), Some("issue")) => issue_token(&args[2..]),
        (Some("test-mode"), Some("status")) => status(),
        _ => {
            eprintln!("rapidr dev test-mode new-key --out DIR        A new test-mode signing key: DIR/test-mode.key (secret) and DIR/test-mode.pub");
            eprintln!("rapidr dev test-mode issue --key FILE --days N [--out TOKEN] [--machine NAME | --this-machine]");
            eprintln!("                                               A test-mode token (default {})", default_token().display());
            eprintln!("rapidr dev test-mode status                    What this build trusts, and whether its test hooks act");
            ExitCode::from(2)
        }
    }
}

/// `--name value` from the arguments.
fn opt<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).map(String::as_str)
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).filter(|h| !h.is_empty()).map(PathBuf::from)
}

/// Where the harnesses look for a token: `~/.rapidr/test-mode.token`.
fn default_token() -> PathBuf {
    home().unwrap_or_default().join(".rapidr").join("test-mode.token")
}

/// `~` at the start of a path, as a shell would.
fn expand(p: &str) -> PathBuf {
    match (p.strip_prefix("~/"), home()) {
        (Some(rest), Some(h)) => h.join(rest),
        _ => PathBuf::from(p),
    }
}

/// The Git work tree `dir` is in, if any (its own folder or a parent's `.git`).
fn work_tree(dir: &Path) -> Option<PathBuf> {
    let dir = std::path::absolute(dir).ok()?;
    dir.ancestors().find(|a| a.join(".git").exists()).map(Path::to_path_buf)
}

/// A folder for secrets: made (0700 on Unix) when missing; refused inside a
/// Git work tree.
fn private_dir(dir: &Path, what: &str) -> Result<(), String> {
    if let Some(repo) = work_tree(dir) {
        return Err(format!("{what} can't go in {}: it is inside the Git work tree {} (use a folder outside any repository, such as ~/.rapidr)", dir.display(), repo.display()));
    }
    fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o700));
    }
    Ok(())
}

/// Writes a file only its owner reads (0600 on Unix); `new`: never over one.
fn write_private(path: &Path, text: &str, new: bool) -> Result<(), String> {
    let mut o = fs::OpenOptions::new();
    o.write(true);
    if new {
        o.create_new(true);
    } else {
        o.create(true).truncate(true);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    let mut f = o.open(path).map_err(|e| if e.kind() == std::io::ErrorKind::AlreadyExists { format!("{} is there already: a key is never overwritten (move it away first)", path.display()) } else { format!("{}: {e}", path.display()) })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = f.set_permissions(fs::Permissions::from_mode(0o600));
    }
    f.write_all(text.as_bytes()).map_err(|e| format!("{}: {e}", path.display()))
}

fn fail(e: String) -> ExitCode {
    eprintln!("rapidr dev test-mode: {e}");
    ExitCode::from(1)
}

fn new_key(args: &[String]) -> ExitCode {
    let Some(out) = opt(args, "--out").map(expand) else {
        return fail("new-key needs --out DIR (a folder outside any repository, such as ~/.rapidr/keys)".into());
    };
    if let Err(e) = private_dir(&out, "The secret key") {
        return fail(e);
    }
    let seed = match issue::new_seed() {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    let public = tm::public_of(&seed);
    let (key_file, pub_file) = (out.join("test-mode.key"), out.join("test-mode.pub"));
    if let Err(e) = write_private(&key_file, &issue::secret_key_file(&seed), true) {
        return fail(e);
    }
    if let Err(e) = fs::write(&pub_file, tm::public_key_file(&public)) {
        return fail(format!("{}: {e}", pub_file.display()));
    }
    println!("A new test-mode key, key id {}:", tm::key_id(&public));
    println!("  {}   SECRET: keep it here (and a backup offline); never in a repository", key_file.display());
    println!("  {}   the public half", pub_file.display());
    println!("Next: builds trust it once the public half is in the repository and they are rebuilt:");
    println!("  cp {} crates/rapidr-testmode/test-mode.pub", pub_file.display());
    println!("Then a token: rapidr dev test-mode issue --key {} --days 90", key_file.display());
    ExitCode::SUCCESS
}

fn issue_token(args: &[String]) -> ExitCode {
    let Some(key_file) = opt(args, "--key").map(expand) else {
        return fail("issue needs --key FILE (the test-mode.key new-key made)".into());
    };
    let days = match opt(args, "--days").map(str::parse::<i64>) {
        Some(Ok(d)) if (1..=tm::MAX_DAYS).contains(&d) => d,
        _ => return fail(format!("issue needs --days N, from 1 to {} (a token runs a year at most)", tm::MAX_DAYS)),
    };
    let machine = if args.iter().any(|a| a == "--this-machine") {
        match tm::this_machine() {
            Some(m) => Some(m),
            None => return fail("this computer's name can't be read: give --machine NAME".into()),
        }
    } else {
        opt(args, "--machine").map(str::to_string)
    };
    if machine.as_deref().is_some_and(|m| m.trim().is_empty() || m.chars().any(char::is_control)) {
        return fail("--machine needs a computer's name".into());
    }
    let out = opt(args, "--out").map(expand).unwrap_or_else(default_token);
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if fs::metadata(&key_file).is_ok_and(|m| m.mode() & 0o077 != 0) {
            eprintln!("warning: {} can be read by other users of this computer (chmod 600 it)", key_file.display());
        }
    }
    let seed = match fs::read_to_string(&key_file).map_err(|e| format!("{}: {e}", key_file.display())).and_then(|t| issue::read_secret(&t)) {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    if let Err(e) = private_dir(out.parent().unwrap_or(Path::new(".")), "A token") {
        return fail(e);
    }
    let today = tm::today();
    let token = tm::sign(&seed, tm::PURPOSE, today, days, machine.as_deref());
    // (checked as a build would, before it's written)
    let public = tm::public_of(&seed);
    let checked = match tm::verify(&token, &public, today, machine.as_deref()) {
        Ok(t) => t,
        Err(e) => return fail(format!("the new token doesn't check ({e})")),
    };
    if let Err(e) = write_private(&out, &token, false) {
        return fail(e);
    }
    println!("A test-mode token for key {}, valid through {} (UTC){}:", tm::key_id(&public), checked.expires, machine.as_deref().map(|m| format!(", for the computer {m}")).unwrap_or_default());
    println!("  {}", out.display());
    match tm::trusted_key() {
        Some(k) if k == public => println!("This rapidr trusts it."),
        Some(k) => println!("This rapidr trusts another key ({}): rebuild with this key's test-mode.pub in crates/rapidr-testmode/.", tm::key_id(&k)),
        None => println!("This rapidr trusts no key yet: copy test-mode.pub to crates/rapidr-testmode/ and rebuild."),
    }
    if out == default_token() {
        println!("The test harnesses find it there (they set RAPIDR_TEST_MODE); elsewhere: export RAPIDR_TEST_MODE={}", out.display());
    } else {
        println!("Use it with: export RAPIDR_TEST_MODE={}", out.display());
    }
    ExitCode::SUCCESS
}

fn status() -> ExitCode {
    if tm::DEV_BUILD {
        let how = if cfg!(debug_assertions) { "debug" } else { "dev-hooks" };
        println!("build: development ({how}): the test hooks act without a token");
    } else {
        println!("build: release: the test hooks act only with a token signed by this build's key");
    }
    match tm::trusted_key() {
        Some(k) => println!("trusted key: {}", tm::key_id(&k)),
        None => println!("trusted key: none (the placeholder test-mode.pub)"),
    }
    match std::env::var_os(tm::ENV) {
        Some(p) => println!("token: {}", PathBuf::from(p).display()),
        None => println!("token: none ({} isn't set; the harnesses use {} when it exists)", tm::ENV, default_token().display()),
    }
    match tm::mode() {
        Mode::Dev => println!("hooks: on (development build)"),
        Mode::Token { expires } => println!("hooks: on (the token is valid through {expires})"),
        Mode::Off(why) => println!("hooks: off ({why})"),
    }
    ExitCode::SUCCESS
}
