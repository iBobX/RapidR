//! RapidR's signed test mode (SEC-24, docs/security/test-mode.md).
//!
//! The GUI test hooks — `RAPIDR_CAPTURE`, `RAPIDR_TEST_EVENTS`,
//! `RAPIDR_TEST_DUMP`, the dialogs' answers, `RAPIDR_TEST_SYSTEM_LOOK`,
//! `RAPIDR_TEST_CRASH`, `RAPIDR_SCHOOL_POLICY` and the rest of [`HOOKS`] —
//! read their variables through [`var`] / [`var_os`]. They act only:
//!
//! - in a **development build** (debug assertions, or the `dev-hooks`
//!   feature that local builds turn on and release builds never do), or
//! - when `RAPIDR_TEST_MODE` names a **token** file signed by the key this
//!   build trusts (`test-mode.pub`, compiled in), for the purpose
//!   `rapidr-test-mode`, not expired, and for this computer when it names one.
//!
//! Otherwise every hook variable is ignored, and the first one a run meets
//! says so on stderr, once. The answer is worked out once per process (the
//! first time a hook asks: one environment lookup when there is no token, a
//! file read and one signature check when there is) and kept.
//!
//! The browser has no process environment: the web runtime's hooks come from
//! the page (`window.RAPIDR_STUDIO_TEST`), which a deployed program never sets.

use std::ffi::OsString;
use std::fmt;
use std::io::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

/// The variable naming the token file.
pub const ENV: &str = "RAPIDR_TEST_MODE";
/// What a token is for: nothing else's signature is taken.
pub const PURPOSE: &str = "rapidr-test-mode";
/// A token's first line (the format's version).
const HEADER: &str = "rapidr-test-mode 1";
/// The longest a token may run (from today): a year.
pub const MAX_DAYS: i64 = 366;
/// (a token is a few lines: anything bigger isn't one)
#[cfg(not(target_arch = "wasm32"))]
const MAX_TOKEN_BYTES: u64 = 4096;

/// A development build: the hooks act without a token (debug assertions, or
/// the `dev-hooks` feature of local builds).
pub const DEV_BUILD: bool = cfg!(debug_assertions) || cfg!(feature = "dev-hooks");

/// The public key file compiled into this build.
pub const TRUSTED_PUB: &str = include_str!("../test-mode.pub");

/// The test hooks' variables (besides every `RAPIDR_TEST_*` but
/// `RAPIDR_TEST_MODE` itself, and every `RAPIDR_CAPTURE*`): [`is_hook`].
pub const HOOKS: &[&str] = &["RAPIDR_SCHOOL_POLICY", "RAPIDR_AI_TEST_SERVER", "RAPIDR_AI_TEST_DNS_FAIL_ONCE", "RAPIDR_LOGIN_SHELL", "RAPIDR_DUMP_MENUS"];

/// An Ed25519 public key.
pub type Key = [u8; 32];

/// Whether `name` is one of the test hooks' variables (any case, as
/// Windows' names are).
pub fn is_hook(name: &str) -> bool {
    let n = name.to_ascii_uppercase();
    (n.starts_with("RAPIDR_TEST_") && n != ENV) || n.starts_with("RAPIDR_CAPTURE") || HOOKS.contains(&n.as_str())
}

// ---- the process's answer --------------------------------------------------------

/// How this process stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    /// A development build: the hooks act.
    Dev,
    /// A valid token: the hooks act until the token's last day.
    Token { expires: String },
    /// The hooks are off, and why.
    Off(Refused),
}

static MODE: OnceLock<Mode> = OnceLock::new();
static NOTED: AtomicBool = AtomicBool::new(false);

/// This process's mode, worked out the first time it's asked.
pub fn mode() -> &'static Mode {
    MODE.get_or_init(|| {
        let path = std::env::var_os(ENV).filter(|p| !p.is_empty());
        let asked = path.is_some();
        let m = decide(DEV_BUILD, path, trusted_key(), today(), this_machine().as_deref(), read_token);
        if let (Mode::Off(why), true) = (&m, asked) {
            note(&format!("RapidR: the test-mode token was refused ({why}); the test hooks are off."));
        }
        m
    })
}

/// Whether the test hooks act in this process.
pub fn on() -> bool {
    !matches!(mode(), Mode::Off(_))
}

/// A test hook's variable: the environment's when the hooks act, else
/// `None` (and, the first time one is set, a line on stderr saying it's
/// ignored).
pub fn var(name: &str) -> Option<String> {
    allowed(name).then(|| std::env::var(name).ok()).flatten()
}

/// [`var`], as the system has it.
pub fn var_os(name: &str) -> Option<OsString> {
    allowed(name).then(|| std::env::var_os(name)).flatten()
}

/// [`var`] as `std::env::var` answers it (`NotPresent` while the hooks are
/// off): a drop-in for a hook's `std::env::var`.
pub fn env_var(name: &str) -> Result<String, std::env::VarError> {
    if allowed(name) {
        std::env::var(name)
    } else {
        Err(std::env::VarError::NotPresent)
    }
}

/// Whether a test hook's variable is set (and the hooks act).
pub fn is_set(name: &str) -> bool {
    var_os(name).is_some()
}

/// Whether a hook may read `name` now: the hooks act; else `false`, said
/// once when it is set.
pub fn allowed(name: &str) -> bool {
    if on() {
        return true;
    }
    if std::env::var_os(name).is_some() {
        let why = match mode() {
            Mode::Off(why) => why.to_string(),
            _ => String::new(),
        };
        note(&format!("RapidR: test hooks are off ({why}): {name} and the other test variables are ignored."));
    }
    false
}

/// One line on stderr, once a process (never a panic: a GUI app may have
/// no stderr).
fn note(line: &str) {
    if !NOTED.swap(true, Ordering::SeqCst) {
        let _ = writeln!(std::io::stderr(), "{line}");
    }
}

/// The mode for these facts (the process's: [`mode`]).
pub fn decide(dev: bool, token: Option<OsString>, key: Option<Key>, today: Day, machine: Option<&str>, read: impl Fn(&std::path::Path) -> Result<String, String>) -> Mode {
    if dev {
        return Mode::Dev;
    }
    let Some(path) = token else { return Mode::Off(Refused::NoToken) };
    let Some(key) = key else { return Mode::Off(Refused::NoTrustedKey) };
    let path = std::path::PathBuf::from(path);
    let text = match read(&path) {
        Ok(t) => t,
        Err(e) => return Mode::Off(Refused::Unreadable(format!("{}: {e}", path.display()))),
    };
    match verify(&text, &key, today, machine) {
        Ok(t) => Mode::Token { expires: t.expires.to_string() },
        Err(why) => Mode::Off(why),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_token(path: &std::path::Path) -> Result<String, String> {
    use std::io::Read as _;
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut text = String::new();
    file.take(MAX_TOKEN_BYTES + 1).read_to_string(&mut text).map_err(|e| e.to_string())?;
    if text.len() as u64 > MAX_TOKEN_BYTES {
        return Err("too big for a token".into());
    }
    Ok(text)
}

#[cfg(target_arch = "wasm32")]
fn read_token(_: &std::path::Path) -> Result<String, String> {
    Err("no files here".into())
}

// ---- keys ------------------------------------------------------------------------

/// The key this build trusts (`test-mode.pub`): `None` for the placeholder.
pub fn trusted_key() -> Option<Key> {
    parse_public_key(TRUSTED_PUB).ok().flatten()
}

/// A public key file: `public: <64 hex digits>` (`none`: the placeholder,
/// trusting nothing); `#` lines are comments.
pub fn parse_public_key(text: &str) -> Result<Option<Key>, String> {
    let value = field(text, "public").ok_or("no `public:` line")?;
    if value.eq_ignore_ascii_case("none") {
        return Ok(None);
    }
    let key: Key = unhex(&value).ok_or("the key isn't 64 hex digits")?;
    if key.iter().all(|b| *b == 0) {
        return Err("the key is all zeros".into());
    }
    Ok(Some(key))
}

/// A key's short name (its first 8 bytes, hex): a token names the key it
/// was signed with, so a token for another build says so.
pub fn key_id(key: &Key) -> String {
    hex(&key[..8])
}

/// A public key file's text.
pub fn public_key_file(key: &Key) -> String {
    format!(
        "# RapidR test-mode public key (Ed25519, hex), key id {}.\n# Builds with this file in crates/rapidr-testmode/ trust tokens signed by its\n# secret half (docs/security/test-mode.md).\npublic: {}\n",
        key_id(key),
        hex(key)
    )
}

/// The `name: value` line's value (the first; `#` lines skipped).
fn field(text: &str, name: &str) -> Option<String> {
    text.lines().map(str::trim).filter(|l| !l.starts_with('#')).find_map(|l| l.split_once(':').filter(|(k, _)| k.trim() == name).map(|(_, v)| v.trim().to_string()))
}

// ---- tokens ----------------------------------------------------------------------

/// Why a token isn't taken.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refused {
    /// `RAPIDR_TEST_MODE` isn't set.
    NoToken,
    /// This build trusts no key (the placeholder `test-mode.pub`).
    NoTrustedKey,
    Unreadable(String),
    /// Not a token: what's wrong.
    Garbled(String),
    /// Signed for another key than this build's.
    OtherKey(String),
    BadSignature,
    WrongPurpose(String),
    Expired(String),
    /// Runs longer than [`MAX_DAYS`] from today.
    TooLong(String),
    /// For another computer.
    OtherMachine(String),
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refused::NoToken => write!(f, "no {ENV} token"),
            Refused::NoTrustedKey => write!(f, "this build trusts no test-mode key"),
            Refused::Unreadable(e) => write!(f, "the token can't be read: {e}"),
            Refused::Garbled(what) => write!(f, "not a test-mode token: {what}"),
            Refused::OtherKey(id) => write!(f, "signed with another key ({id}) than this build's"),
            Refused::BadSignature => write!(f, "the signature doesn't match"),
            Refused::WrongPurpose(p) => write!(f, "the token is for {p:?}, not {PURPOSE:?}"),
            Refused::Expired(d) => write!(f, "the token expired on {d}"),
            Refused::TooLong(d) => write!(f, "the token runs to {d}, more than {MAX_DAYS} days"),
            Refused::OtherMachine(m) => write!(f, "the token is for the computer {m:?}"),
        }
    }
}

/// A token's claims, once its signature is checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub purpose: String,
    pub key_id: String,
    pub issued: Day,
    pub expires: Day,
    pub machine: Option<String>,
}

impl Token {
    /// The signed bytes: the claims in one fixed form, whatever the file's
    /// spacing, line ends or comments.
    pub fn message(&self) -> String {
        let mut m = format!("{HEADER}\npurpose: {}\nkey: {}\nissued: {}\nexpires: {}\n", self.purpose, self.key_id, self.issued, self.expires);
        if let Some(machine) = &self.machine {
            m.push_str(&format!("machine: {machine}\n"));
        }
        m
    }

    /// The token file: the claims, then the signature.
    pub fn file(&self, signature: &[u8; 64]) -> String {
        format!("# RapidR test-mode token (docs/security/test-mode.md): keep it outside the repository.\n{}signature: {}\n", self.message(), hex(signature))
    }
}

/// Reads a token and checks it: the format, the key it names, its
/// signature by `key`, its purpose, its dates against `today` (valid
/// through its `expires` day, UTC) and its computer against `machine`.
pub fn verify(text: &str, key: &Key, today: Day, machine: Option<&str>) -> Result<Token, Refused> {
    let garbled = |what: &str| Refused::Garbled(what.to_string());
    let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#'));
    if lines.next() != Some(HEADER) {
        return Err(garbled("the first line isn't `rapidr-test-mode 1`"));
    }
    let (mut purpose, mut id, mut issued, mut expires, mut on, mut signature) = (None, None, None, None, None, None);
    for line in lines {
        let (name, value) = line.split_once(':').ok_or_else(|| garbled("a line without `name:`"))?;
        let value = value.trim().to_string();
        let slot = match name.trim() {
            "purpose" => &mut purpose,
            "key" => &mut id,
            "issued" => &mut issued,
            "expires" => &mut expires,
            "machine" => &mut on,
            "signature" => &mut signature,
            other => return Err(Refused::Garbled(format!("an unknown line `{other}:`"))),
        };
        if slot.replace(value).is_some() {
            return Err(Refused::Garbled(format!("`{}:` twice", name.trim())));
        }
    }
    let need = |v: Option<String>, what: &str| v.filter(|v| !v.is_empty()).ok_or_else(|| Refused::Garbled(format!("no `{what}:`")));
    let token = Token {
        purpose: need(purpose, "purpose")?,
        key_id: need(id, "key")?,
        issued: Day::parse(&need(issued, "issued")?).ok_or_else(|| garbled("`issued:` isn't a date (YYYY-MM-DD)"))?,
        expires: Day::parse(&need(expires, "expires")?).ok_or_else(|| garbled("`expires:` isn't a date (YYYY-MM-DD)"))?,
        machine: on.filter(|m| !m.is_empty()),
    };
    let signature: [u8; 64] = unhex(&need(signature, "signature")?).ok_or_else(|| garbled("the signature isn't 128 hex digits"))?;
    if token.key_id != key_id(key) {
        return Err(Refused::OtherKey(token.key_id));
    }
    if !signature_ok(key, token.message().as_bytes(), &signature) {
        return Err(Refused::BadSignature);
    }
    if token.purpose != PURPOSE {
        return Err(Refused::WrongPurpose(token.purpose));
    }
    if token.expires < today {
        return Err(Refused::Expired(token.expires.to_string()));
    }
    if token.expires.0 - today.0 > MAX_DAYS || token.issued > token.expires {
        return Err(Refused::TooLong(token.expires.to_string()));
    }
    if let Some(wanted) = &token.machine {
        if !machine.is_some_and(|m| same_machine(m, wanted)) {
            return Err(Refused::OtherMachine(wanted.clone()));
        }
    }
    Ok(token)
}

#[cfg(not(target_arch = "wasm32"))]
fn signature_ok(key: &Key, message: &[u8], signature: &[u8; 64]) -> bool {
    ed25519_compact::PublicKey::new(*key).verify(message, &ed25519_compact::Signature::new(*signature)).is_ok()
}

#[cfg(target_arch = "wasm32")]
fn signature_ok(_: &Key, _: &[u8], _: &[u8; 64]) -> bool {
    false
}

/// (a computer's name, any case, with or without macOS' `.local`)
fn same_machine(a: &str, b: &str) -> bool {
    let n = |s: &str| {
        let s = s.trim().to_ascii_lowercase();
        s.strip_suffix(".local").map(str::to_string).unwrap_or(s)
    };
    n(a) == n(b)
}

/// This computer's name (a token's optional machine hint — a guard against
/// a token copied by mistake, not a lock).
pub fn this_machine() -> Option<String> {
    #[cfg(unix)]
    {
        let mut buf = [0u8; 256];
        // SAFETY: the buffer is ours and its length is passed.
        let r = unsafe { libc::gethostname(buf.as_mut_ptr().cast(), buf.len()) };
        if r != 0 {
            return None;
        }
        let end = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
        String::from_utf8(buf[..end].to_vec()).ok().filter(|s| !s.is_empty())
    }
    #[cfg(windows)]
    {
        std::env::var("COMPUTERNAME").ok().filter(|s| !s.is_empty())
    }
    #[cfg(not(any(unix, windows)))]
    {
        None
    }
}

// ---- the issuing side (the CLI, the tests) -------------------------------------------

/// A token for `days` from `issued`, signed with the key made from `seed`.
#[cfg(all(any(test, feature = "issue"), not(target_arch = "wasm32")))]
pub fn sign(seed: &[u8; 32], purpose: &str, issued: Day, days: i64, machine: Option<&str>) -> String {
    let pair = ed25519_compact::KeyPair::from_seed(ed25519_compact::Seed::new(*seed));
    let token = Token { purpose: purpose.to_string(), key_id: key_id(&pair.pk), issued, expires: Day(issued.0 + days), machine: machine.map(str::to_string) };
    token.file(&pair.sk.sign(token.message().as_bytes(), None))
}

/// The public key of the key made from `seed`.
#[cfg(all(any(test, feature = "issue"), not(target_arch = "wasm32")))]
pub fn public_of(seed: &[u8; 32]) -> Key {
    *ed25519_compact::KeyPair::from_seed(ed25519_compact::Seed::new(*seed)).pk
}

/// `rapidr dev test-mode new-key` / `issue`.
#[cfg(all(feature = "issue", not(target_arch = "wasm32")))]
pub mod issue {
    use super::*;

    /// A new secret seed from the system's secure random source.
    pub fn new_seed() -> Result<[u8; 32], String> {
        let mut seed = [0u8; 32];
        getrandom::fill(&mut seed).map_err(|e| format!("no secure random numbers: {e}"))?;
        Ok(seed)
    }

    /// The secret key file's text.
    pub fn secret_key_file(seed: &[u8; 32]) -> String {
        format!(
            "# RapidR test-mode SIGNING key (Ed25519 seed, hex), key id {}.\n# SECRET: never in the repository, never shared. Whoever has it can turn on\n# the test hooks of every RapidR build that trusts its public half.\nsecret: {}\n",
            key_id(&public_of(seed)),
            hex(seed)
        )
    }

    /// A secret key file's seed.
    pub fn read_secret(text: &str) -> Result<[u8; 32], String> {
        let v = field(text, "secret").ok_or("no `secret:` line (is this the .key file?)")?;
        unhex(&v).ok_or_else(|| "the secret isn't 64 hex digits".to_string())
    }
}

// ---- days and hex ----------------------------------------------------------------

/// A day: days since 1970-01-01 (UTC).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Day(pub i64);

/// Today (UTC). In a browser there is no operating system clock (std's
/// `SystemTime::now` panics there — it took the whole web Studio down at its
/// start, the gate asking for the day before it knew there is no token) and
/// no token either: day 0, which no token is valid on.
pub fn today() -> Day {
    #[cfg(target_arch = "wasm32")]
    {
        Day(0)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
        Day(secs.div_euclid(86_400))
    }
}

impl Day {
    /// `YYYY-MM-DD`, a real date.
    pub fn parse(s: &str) -> Option<Day> {
        let b = s.as_bytes();
        if b.len() != 10 || b[4] != b'-' || b[7] != b'-' || !s.chars().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit()) {
            return None;
        }
        let (y, m, d): (i64, i64, i64) = (s[..4].parse().ok()?, s[5..7].parse().ok()?, s[8..].parse().ok()?);
        let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
        let len = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        if !(1..=12).contains(&m) || d < 1 || d > len[(m - 1) as usize] {
            return None;
        }
        // (days from the civil date: H. Hinnant's public-domain algorithm)
        let y = if m <= 2 { y - 1 } else { y };
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        Some(Day(era * 146_097 + doe - 719_468))
    }
}

impl fmt::Display for Day {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let z = self.0 + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = yoe + era * 400 + i64::from(m <= 2);
        write!(f, "{y:04}-{m:02}-{d:02}")
    }
}

/// Lower-case hex.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Exactly `N` bytes of hex (either case).
pub fn unhex<const N: usize>(s: &str) -> Option<[u8; N]> {
    let s = s.trim().as_bytes();
    if s.len() != 2 * N {
        return None;
    }
    let digit = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    let mut out = [0u8; N];
    for (i, pair) in s.chunks(2).enumerate() {
        out[i] = digit(pair[0])? << 4 | digit(pair[1])?;
    }
    Some(out)
}

#[cfg(test)]
mod tests;

/// The security regressions (tests/security/, `tools/regress.sh security`):
/// SEC-24's tokens with a fixture key made here, in the tests only.
#[cfg(test)]
#[path = "../../../tests/security/test_mode_tokens.rs"]
mod security_regressions;
