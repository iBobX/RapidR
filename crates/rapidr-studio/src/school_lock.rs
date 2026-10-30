//! School Lock (docs/manual/for-teachers.md): a teacher — or a school's IT,
//! for every account on a computer — turns features of RapidR Studio off
//! behind a password. For now the features are its AI: RAI in Studio
//! (`ai`) and AI apps working in Studio over MCP (`ai_apps`); the list is
//! open for more.
//!
//! - **The password is never kept**, only a salted, slow hash of it:
//!   PBKDF2-HMAC-SHA256 (RFC 8018 §5.2, HMAC as RFC 2104), [`ITERATIONS`]
//!   rounds, a 16-byte salt from the system's secure random source, a
//!   32-byte key — written `pbkdf2-sha256$600000$<salt hex>$<key hex>`.
//!   SHA-256's padding, HMAC and PBKDF2 are written here on `sha2`'s
//!   compression function (0.11, already in the tree: it uses the
//!   processor's SHA instructions where there are some) — no new
//!   dependency. A guess is compared in constant time.
//! - **Wrong tries are limited**: after [`MAX_TRIES`] wrong passwords in a
//!   row, [`WAIT_SECS`] seconds pass before the next try is even checked;
//!   the count and the wait are kept in Studio's folder, so closing Studio
//!   doesn't start them again.
//! - **The per-user setting** (on, the features, the hash) is Studio's to
//!   keep with its other settings: [`UserSetting`]'s text.
//! - **The computer's policy** ([`read_policy`]) is a TOML file in a place
//!   only an administrator can write — `/Library/Application
//!   Support/RapidR/policy.toml` (macOS), `%ProgramData%\RapidR\policy.toml`
//!   (Windows), `/etc/rapidr/policy.toml` (Linux):
//!
//!   ```toml
//!   school_lock = true
//!   locked = ["ai", "ai_apps"]
//!   password_hash = "pbkdf2-sha256$600000$…$…"   # rapidr school-lock hash
//!   ```
//!
//!   It wins over the user's setting: while it locks, the lock can only be
//!   unlocked until Studio closes (with its password), never turned off from
//!   Studio. A policy file that can't be read locks everything with no way to
//!   unlock (it fails closed: IT fixes the file). `RAPIDR_SCHOOL_POLICY`
//!   names a file read *only when the computer has none* — the tests' — so
//!   it can add a lock, never take the computer's away.
//! - In a browser there is no policy: the setting lives in the page's
//!   storage (weaker: the manual says so).

use rapidr_value::Value;

/// PBKDF2's rounds for a new hash (OWASP's 2023 figure for
/// PBKDF2-HMAC-SHA256).
pub const ITERATIONS: u32 = 600_000;
/// The fewest characters a password may have.
pub const MIN_PASSWORD: usize = 6;
/// Wrong tries in a row before a wait.
pub const MAX_TRIES: u32 = 5;
/// The wait, in seconds.
pub const WAIT_SECS: f64 = 60.0;
/// The features School Lock can turn off: RAI in Studio, AI apps (MCP).
pub const FEATURES: &[&str] = &["ai", "ai_apps"];
const SCHEME: &str = "pbkdf2-sha256";
/// Rounds a hash may say (a hash asking for more is refused, not run).
const MAX_ITERATIONS: u32 = 10_000_000;

// ---- the hash ------------------------------------------------------------------

/// SHA-256's first state (FIPS 180-4 §5.3.3).
const H0: [u32; 8] = [0x6a09_e667, 0xbb67_ae85, 0x3c6e_f372, 0xa54f_f53a, 0x510e_527f, 0x9b05_688c, 0x1f83_d9ab, 0x5be0_cd19];

fn bytes_of(state: &[u32; 8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (i, w) in state.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&w.to_be_bytes());
    }
    out
}

/// SHA-256 of `parts` one after the other (FIPS 180-4's padding), on the
/// compression function — the processor's SHA instructions where it has
/// them (Apple silicon, recent x86), else portable code.
fn digest(parts: &[&[u8]]) -> [u8; 32] {
    let mut st = H0;
    let mut block = [0u8; 64];
    let (mut n, mut total) = (0usize, 0u64);
    for part in parts {
        for &b in *part {
            block[n] = b;
            n += 1;
            total += 1;
            if n == 64 {
                sha256::block_api::compress256(&mut st, &[block]);
                n = 0;
            }
        }
    }
    block[n] = 0x80;
    block[n + 1..].fill(0);
    if n >= 56 {
        sha256::block_api::compress256(&mut st, &[block]);
        block = [0u8; 64];
    }
    block[56..].copy_from_slice(&(total * 8).to_be_bytes());
    sha256::block_api::compress256(&mut st, &[block]);
    bytes_of(&st)
}

/// HMAC-SHA256's two padded key blocks (RFC 2104: the key XOR ipad, XOR opad).
fn hmac_pads(key: &[u8]) -> ([u8; 64], [u8; 64]) {
    let mut k = [0u8; 64];
    if key.len() > 64 {
        k[..32].copy_from_slice(&digest(&[key]));
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let (mut ipad, mut opad) = ([0x36u8; 64], [0x5cu8; 64]);
    for i in 0..64 {
        ipad[i] ^= k[i];
        opad[i] ^= k[i];
    }
    (ipad, opad)
}

/// HMAC-SHA256 of `parts` (any length).
fn hmac(pads: &([u8; 64], [u8; 64]), parts: &[&[u8]]) -> [u8; 32] {
    let mut inner: Vec<&[u8]> = vec![&pads.0];
    inner.extend_from_slice(parts);
    let ih = digest(&inner);
    digest(&[&pads.1, &ih])
}

/// SHA-256's state after one 64-byte block (a key block, done once).
fn state_after(block: &[u8; 64]) -> [u32; 8] {
    let mut st = H0;
    sha256::block_api::compress256(&mut st, &[*block]);
    st
}

/// The hash of a key block (its state, `state_after`) and a 32-byte
/// message: one compression — the message, its padding and the length
/// (96 bytes, 768 bits) in a single block.
fn finish32(state: &[u32; 8], msg: &[u8; 32]) -> [u8; 32] {
    let mut block = [0u8; 64];
    block[..32].copy_from_slice(msg);
    block[32] = 0x80;
    block[62] = 0x03;
    let mut st = *state;
    sha256::block_api::compress256(&mut st, &[block]);
    bytes_of(&st)
}

/// PBKDF2-HMAC-SHA256 with a 32-byte result (one block: RFC 8018's T_1).
/// Each round after the first is two compressions on the key blocks'
/// states, computed once.
pub fn pbkdf2_sha256(password: &[u8], salt: &[u8], iterations: u32) -> [u8; 32] {
    let pads = hmac_pads(password);
    let (inner, outer) = (state_after(&pads.0), state_after(&pads.1));
    let mut u = hmac(&pads, &[salt, &1u32.to_be_bytes()]);
    let mut t = u;
    for _ in 1..iterations.max(1) {
        u = finish32(&outer, &finish32(&inner, &u));
        for (a, b) in t.iter_mut().zip(u) {
            *a ^= b;
        }
    }
    t
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) || !s.bytes().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok()).collect()
}

/// The hash line of `password` with `salt` (16 random bytes for a real one).
pub fn hash_password(password: &str, salt: &[u8], iterations: u32) -> String {
    let key = pbkdf2_sha256(password.as_bytes(), salt, iterations);
    format!("{SCHEME}${iterations}${}${}", hex(salt), hex(&key))
}

/// A new hash line for `password`: [`ITERATIONS`] rounds, a fresh salt
/// from `random` (the system's secure source: false when it has none).
pub fn new_hash(password: &str, random: impl FnOnce(&mut [u8]) -> bool) -> Result<String, String> {
    if password.chars().count() < MIN_PASSWORD {
        return Err(format!("the password needs at least {MIN_PASSWORD} characters"));
    }
    let mut salt = [0u8; 16];
    if !random(&mut salt) {
        return Err("this system gave no secure random numbers for the password's salt".into());
    }
    Ok(hash_password(password, &salt, ITERATIONS))
}

/// A new hash line with the system's secure random source (the desktop's:
/// `rapidr school-lock hash`).
#[cfg(not(target_arch = "wasm32"))]
pub fn new_hash_here(password: &str) -> Result<String, String> {
    new_hash(password, |b| getrandom::fill(b).is_ok())
}

/// Equal, in a time that doesn't depend on where they differ.
fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Whether `password` is the one `stored` was made from; Err when `stored`
/// is no hash line this understands.
pub fn verify_password(password: &str, stored: &str) -> Result<bool, String> {
    let mut parts = stored.trim().split('$');
    let (Some(scheme), Some(rounds), Some(salt), Some(key), None) = (parts.next(), parts.next(), parts.next(), parts.next(), parts.next()) else {
        return Err("not a School Lock password hash".into());
    };
    if scheme != SCHEME {
        return Err(format!("a password hash of another kind ({scheme})"));
    }
    let rounds: u32 = rounds.parse().map_err(|_| "a password hash without its rounds".to_string())?;
    if rounds == 0 || rounds > MAX_ITERATIONS {
        return Err(format!("a password hash with {rounds} rounds"));
    }
    let (Some(salt), Some(key)) = (unhex(salt), unhex(key)) else {
        return Err("a damaged password hash".into());
    };
    if salt.is_empty() || key.len() != 32 {
        return Err("a damaged password hash".into());
    }
    Ok(same(&pbkdf2_sha256(password.as_bytes(), &salt, rounds), &key))
}

/// How hard a password is to guess, for the hint under the field: 0 too
/// short (fewer than [`MIN_PASSWORD`] characters), 1 weak, 2 fair, 3 strong.
pub fn strength(password: &str) -> u8 {
    let n = password.chars().count();
    if n < MIN_PASSWORD {
        return 0;
    }
    let lower = password.to_lowercase();
    let common = ["123456", "1234567", "12345678", "123456789", "password", "qwerty", "abc123", "111111", "000000", "contraseña", "maestro", "teacher", "letmein"];
    let one_char = password.chars().all(|c| password.starts_with(c));
    if one_char || common.contains(&lower.as_str()) || is_run(&lower) {
        return 1;
    }
    let kinds = [password.chars().any(char::is_lowercase), password.chars().any(char::is_uppercase), password.chars().any(|c| c.is_ascii_digit()), password.chars().any(|c| !c.is_alphanumeric())].iter().filter(|k| **k).count();
    match (n, kinds) {
        (12.., 2..) | (10.., 3..) => 3,
        (8.., 2..) | (10.., _) => 2,
        _ => 1,
    }
}

/// "abcdef", "654321": every character the one after (or before) the last.
fn is_run(s: &str) -> bool {
    let c: Vec<u32> = s.chars().map(u32::from).collect();
    c.windows(2).all(|w| w[1] == w[0] + 1) || c.windows(2).all(|w| w[0] == w[1] + 1)
}

// ---- wrong tries ------------------------------------------------------------------

/// Wrong tries in a row and, after [`MAX_TRIES`], until when the next one
/// waits (seconds since 1970).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Throttle {
    pub failures: u32,
    pub until: f64,
}

impl Throttle {
    /// Seconds before another try is checked (0: now).
    pub fn wait_left(&self, now: f64) -> f64 {
        (self.until - now).max(0.0)
    }

    /// Wrong tries left before a wait.
    pub fn tries_left(&self) -> u32 {
        MAX_TRIES - self.failures % MAX_TRIES
    }

    fn wrong(&mut self, now: f64) {
        self.failures += 1;
        if self.failures.is_multiple_of(MAX_TRIES) {
            self.until = now + WAIT_SECS;
        }
    }

    fn right(&mut self) {
        *self = Throttle::default();
    }

    /// As kept in Studio's folder: "failures until".
    pub fn to_text(&self) -> String {
        format!("{} {}", self.failures, self.until)
    }

    pub fn parse(text: &str) -> Throttle {
        let mut it = text.split_whitespace();
        let failures = it.next().and_then(|f| f.parse().ok()).unwrap_or(0);
        let until = it.next().and_then(|u| u.parse::<f64>().ok()).filter(|u| u.is_finite()).unwrap_or(0.0);
        Throttle { failures, until }
    }
}

// ---- the settings ---------------------------------------------------------------

fn features_of(list: &str) -> Vec<String> {
    list.split(',').map(|f| f.trim().to_ascii_lowercase()).filter(|f| FEATURES.contains(&f.as_str())).fold(Vec::new(), |mut v, f| {
        if !v.contains(&f) {
            v.push(f);
        }
        v
    })
}

/// The per-user setting Studio keeps with its others: `on=1;features=ai,ai_apps;hash=…`
/// (no tab, no line break: a registry value as it is).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UserSetting {
    pub on: bool,
    pub features: Vec<String>,
    pub hash: String,
}

impl UserSetting {
    pub fn parse(text: &str) -> UserSetting {
        let mut s = UserSetting::default();
        for part in text.split(';') {
            let (k, v) = part.split_once('=').unwrap_or((part, ""));
            match k.trim() {
                "on" => s.on = v.trim() == "1",
                "features" => s.features = features_of(v),
                "hash" => s.hash = v.trim().to_string(),
                _ => {}
            }
        }
        // (on without a password can't be unlocked: a setting written by
        // hand, not by Studio — taken as off)
        if s.hash.is_empty() {
            s.on = false;
        }
        s
    }

    pub fn to_text(&self) -> String {
        if !self.on {
            return String::new();
        }
        format!("on=1;features={};hash={}", self.features.join(","), self.hash)
    }
}

/// The computer's policy, as read from its file.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Policy {
    /// It locks (`school_lock`; true when the file doesn't say, or can't be read).
    pub lock: bool,
    pub features: Vec<String>,
    pub hash: String,
    /// The file it was read from.
    pub file: String,
    /// Why it couldn't be read ("" it could): then it locks every feature,
    /// with no password.
    pub error: String,
}

/// The policy in `text` (TOML), read from `file`. Anything wrong in it
/// locks every feature with no way to unlock: it fails closed.
pub fn parse_policy(text: &str, file: &str) -> Policy {
    let closed = |error: String| Policy { lock: true, features: FEATURES.iter().map(|f| f.to_string()).collect(), hash: String::new(), file: file.to_string(), error };
    let table: toml::Table = match text.parse() {
        Ok(t) => t,
        Err(e) => return closed(format!("it isn't TOML ({})", e.message())),
    };
    let lock = match table.get("school_lock") {
        None => true,
        Some(toml::Value::Boolean(b)) => *b,
        Some(_) => return closed("school_lock is neither true nor false".into()),
    };
    let features = match table.get("locked") {
        None => FEATURES.iter().map(|f| f.to_string()).collect(),
        Some(toml::Value::Array(a)) => features_of(&a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(",")),
        Some(_) => return closed("locked isn't a list of features".into()),
    };
    let hash = match table.get("password_hash") {
        None => String::new(),
        Some(toml::Value::String(h)) => h.trim().to_string(),
        Some(_) => return closed("password_hash isn't text".into()),
    };
    if lock && !hash.is_empty() {
        if let Err(e) = verify_password("", &hash) {
            return closed(format!("its password_hash can't be used: {e}"));
        }
    }
    Policy { lock, features, hash, file: file.to_string(), error: String::new() }
}

/// Where the computer's policy is: an administrator's place on each system.
pub fn system_policy_path() -> Option<String> {
    system_policy_paths().into_iter().next()
}

/// Every place the computer's policy may be, the usual one first. On
/// Windows the ProgramData folder is also looked for where Windows puts it
/// (`C:\ProgramData`): an environment variable a user sets can't move the
/// policy out of sight.
fn system_policy_paths() -> Vec<String> {
    if cfg!(target_arch = "wasm32") {
        Vec::new()
    } else if cfg!(target_os = "macos") {
        vec!["/Library/Application Support/RapidR/policy.toml".into()]
    } else if cfg!(windows) {
        let mut v: Vec<String> = Vec::new();
        for base in [std::env::var("ProgramData").ok(), Some(r"C:\ProgramData".to_string())].into_iter().flatten().filter(|p| !p.is_empty()) {
            let f = format!(r"{base}\RapidR\policy.toml");
            if !v.iter().any(|x| x.eq_ignore_ascii_case(&f)) {
                v.push(f);
            }
        }
        v
    } else {
        vec!["/etc/rapidr/policy.toml".into()]
    }
}

/// The computer's policy: its file, when there is one; else the file
/// `RAPIDR_SCHOOL_POLICY` names (the tests'), which can only add a lock.
/// `read` gives a file's text: `Ok(None)` when there is no such file.
pub fn read_policy(read: impl Fn(&str) -> Result<Option<String>, String>) -> Option<Policy> {
    let mut places = system_policy_paths();
    if let Some(test) = rapidr_testmode::env_var("RAPIDR_SCHOOL_POLICY").ok().filter(|p| !p.is_empty()) {
        places.push(test);
    }
    for file in places {
        match read(&file) {
            Ok(Some(text)) => return Some(parse_policy(&text, &file)),
            Ok(None) => {}
            Err(e) => return Some(Policy { lock: true, features: FEATURES.iter().map(|f| f.to_string()).collect(), hash: String::new(), file, error: format!("it can't be read ({e})") }),
        }
    }
    None
}

// ---- the lock --------------------------------------------------------------------

/// What a password did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The right password: done.
    Right,
    /// Not the password (counted).
    Wrong,
    /// Too many wrong tries: not checked; [`SchoolLock::wait_left`] says how long.
    Wait,
    /// The computer's policy keeps the lock on: only IT can turn it off.
    Policy,
    /// No password can unlock it (a policy without one, or one that can't be read).
    NoPassword,
}

impl Outcome {
    /// As RPROJECT's methods answer: 1 right, 0 wrong, -1 wait, -2 the
    /// policy keeps it, -3 no password can.
    pub fn code(self) -> i64 {
        match self {
            Outcome::Right => 1,
            Outcome::Wrong => 0,
            Outcome::Wait => -1,
            Outcome::Policy => -2,
            Outcome::NoPassword => -3,
        }
    }
}

/// The lock: the user's setting, the computer's policy, whether it is
/// unlocked until Studio closes, the wrong tries.
#[derive(Clone, Debug, Default)]
pub struct SchoolLock {
    pub user: UserSetting,
    pub policy: Option<Policy>,
    pub unlocked: bool,
    pub throttle: Throttle,
}

impl SchoolLock {
    /// The computer's policy locks (it then wins over the user's setting).
    pub fn by_policy(&self) -> bool {
        self.policy.as_ref().is_some_and(|p| p.lock)
    }

    /// The lock is on (the policy's or the user's), unlocked for now or not.
    pub fn on(&self) -> bool {
        self.by_policy() || self.user.on
    }

    /// The features it turns off: the policy's while it locks, else the user's.
    pub fn features(&self) -> Vec<String> {
        match &self.policy {
            Some(p) if p.lock => p.features.clone(),
            _ if self.user.on => self.user.features.clone(),
            _ => Vec::new(),
        }
    }

    /// Whether `feature` is off now.
    pub fn locked(&self, feature: &str) -> bool {
        self.on() && !self.unlocked && self.features().iter().any(|f| f.eq_ignore_ascii_case(feature))
    }

    fn hash(&self) -> &str {
        match &self.policy {
            Some(p) if p.lock => &p.hash,
            _ => &self.user.hash,
        }
    }

    pub fn wait_left(&self, now: f64) -> f64 {
        self.throttle.wait_left(now)
    }

    /// `password` checked against the lock's hash — unless a wait runs.
    fn check(&mut self, password: &str, now: f64) -> Outcome {
        if !self.on() {
            return Outcome::Right;
        }
        if self.hash().is_empty() || self.policy.as_ref().is_some_and(|p| p.lock && !p.error.is_empty()) {
            return Outcome::NoPassword;
        }
        if self.throttle.wait_left(now) > 0.0 {
            return Outcome::Wait;
        }
        match verify_password(password, self.hash()) {
            Ok(true) => {
                self.throttle.right();
                Outcome::Right
            }
            Ok(false) => {
                self.throttle.wrong(now);
                Outcome::Wrong
            }
            Err(_) => Outcome::NoPassword,
        }
    }

    /// Unlock… > Unlock Until Studio Closes.
    pub fn unlock(&mut self, password: &str, now: f64) -> Outcome {
        let o = self.check(password, now);
        if o == Outcome::Right {
            self.unlocked = true;
        }
        o
    }

    /// Unlock… > Turn School Lock Off: the user's lock, with its password;
    /// never the policy's.
    pub fn turn_off(&mut self, password: &str, now: f64) -> Outcome {
        if self.by_policy() {
            return Outcome::Policy;
        }
        let o = self.check(password, now);
        if o == Outcome::Right {
            self.user = UserSetting::default();
            self.unlocked = false;
        }
        o
    }

    /// Tools > School Lock… > Lock: on, with a new password's hash.
    pub fn turn_on(&mut self, hash: String, features: &str) -> Result<(), String> {
        if self.by_policy() {
            return Err("this computer's School Lock is set by its IT".into());
        }
        let features = features_of(features);
        if features.is_empty() {
            return Err("choose what to lock".into());
        }
        self.user = UserSetting { on: true, features, hash };
        self.unlocked = false;
        Ok(())
    }

    /// Lock Again: locked as before the unlock.
    pub fn relock(&mut self) {
        self.unlocked = false;
    }
}

// ---- RPROJECT's SchoolLock… members --------------------------------------------

thread_local! {
    /// Studio's lock (one per Studio: the process's).
    static LOCK: std::cell::RefCell<SchoolLock> = std::cell::RefCell::new(SchoolLock::default());
}

/// Where the wrong tries are kept between runs: Studio's folder.
fn tries_file() -> String {
    format!("{}/school-lock-tries", crate::recovery::studio_folder())
}

fn keep_tries(t: &Throttle) {
    // (Studio's folder made first: a new user's isn't there yet)
    crate::recovery::make_folder(&crate::recovery::studio_folder());
    let _ = crate::write_text(&tries_file(), &t.to_text());
}

/// The computer's policy file read from the disk (none in a browser).
fn read_disk(file: &str) -> Result<Option<String>, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        match std::fs::read_to_string(file) {
            Ok(t) => Ok(Some(t)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = file;
        Ok(None)
    }
}

/// RPROJECT's School Lock members (`method` in lower case): its value and
/// what Error says ("" nothing went wrong); `None` when it isn't one.
///
/// | Member | |
/// |---|---|
/// | `SchoolLockLoad(Setting)` → on | the user's setting as Studio kept it, the computer's policy read again, the wrong tries kept |
/// | `SchoolLock(What)` → text | `on`, `unlocked`, `policy` ("1" / "0"), `features` ("ai,ai_apps"), `wait` (seconds), `left` (tries before a wait), `setting` (to keep), `file` (the policy's), `error` (why the policy can't be read) |
/// | `SchoolLocked(Feature)` → True / False | `ai` (RAI in Studio), `ai_apps` (AI apps, MCP) turned off now |
/// | `SchoolLockTurnOn(Password, Features)` → True / False | on, the password's hash kept (never the password) |
/// | `SchoolLockUnlock(Password)`, `SchoolLockTurnOff(Password)` | 1 right, 0 wrong, -1 wait, -2 the policy keeps it, -3 no password can |
/// | `SchoolLockRelock` | locked again |
/// | `SchoolLockStrength(Password)` → 0–3 | too short, weak, fair, strong |
pub fn call<H: crate::Host>(host: H, method: &str, args: &[Value]) -> Option<(Value, String)> {
    let s = |i: usize| crate::text_arg(args, i);
    let now = host.unix_time();
    Some(match method {
        "schoollockload" => LOCK.with(|l| {
            let mut l = l.borrow_mut();
            l.user = UserSetting::parse(&s(0));
            l.policy = read_policy(read_disk);
            l.throttle = crate::read_text(&tries_file()).map(|t| Throttle::parse(&t)).unwrap_or_default();
            let err = l.policy.as_ref().map(|p| p.error.clone()).unwrap_or_default();
            (crate::flag(l.on()), err)
        }),
        "schoollock" => LOCK.with(|l| {
            let l = l.borrow();
            let b = |x: bool| if x { "1" } else { "0" }.to_string();
            let text = match s(0).to_ascii_lowercase().as_str() {
                "on" => b(l.on()),
                "unlocked" => b(l.on() && l.unlocked),
                "policy" => b(l.by_policy()),
                "features" => l.features().join(","),
                "wait" => format!("{}", l.wait_left(now).ceil() as i64),
                "left" => format!("{}", l.throttle.tries_left()),
                "setting" => l.user.to_text(),
                "file" => l.policy.as_ref().map(|p| p.file.clone()).unwrap_or_default(),
                "error" => l.policy.as_ref().map(|p| p.error.clone()).unwrap_or_default(),
                _ => String::new(),
            };
            (Value::String(text.into()), String::new())
        }),
        "schoollocked" => (crate::flag(LOCK.with(|l| l.borrow().locked(&s(0)))), String::new()),
        "schoollockturnon" => {
            let hash = new_hash(&s(0), |b| host.random_bytes(b));
            let r = hash.and_then(|h| LOCK.with(|l| l.borrow_mut().turn_on(h, &s(1))));
            (crate::flag(r.is_ok()), r.err().unwrap_or_default())
        }
        "schoollockunlock" | "schoollockturnoff" => LOCK.with(|l| {
            let mut l = l.borrow_mut();
            let before = l.throttle.clone();
            let o = if method == "schoollockunlock" { l.unlock(&s(0), now) } else { l.turn_off(&s(0), now) };
            if l.throttle != before {
                keep_tries(&l.throttle);
            }
            (Value::Integer(o.code()), String::new())
        }),
        "schoollockrelock" => {
            LOCK.with(|l| l.borrow_mut().relock());
            (Value::Null, String::new())
        }
        "schoollockstrength" => (Value::Integer(i64::from(strength(&s(0)))), String::new()),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex_of(b: [u8; 32]) -> String {
        hex(&b)
    }

    #[test]
    fn pbkdf2_matches_the_published_vectors() {
        // (RFC 7914 §11 / the common PBKDF2-HMAC-SHA256 vectors, and ones
        // made with Python's hashlib.pbkdf2_hmac)
        assert_eq!(hex_of(pbkdf2_sha256(b"password", b"salt", 1)), "120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b");
        assert_eq!(hex_of(pbkdf2_sha256(b"password", b"salt", 2)), "ae4d0c95af6b46d32d0adff928f06dd02a303f8ef3c251dfd6e2d85a95474c43");
        assert_eq!(hex_of(pbkdf2_sha256(b"password", b"salt", 4096)), "c5e478d59288c841aa530db6845c4c8d962893a001ce4e11a4963873aa98134a");
        assert_eq!(hex_of(pbkdf2_sha256(b"passwordPASSWORDpassword", b"saltSALTsaltSALTsaltSALTsaltSALTsalt", 4096)), "348c89dbcbd32b2f32d814b8116e84cf2b17347ebc1800181c4e2a1fb8dd53e1");
        // (a key longer than SHA-256's block: hashed first)
        assert_eq!(hex_of(pbkdf2_sha256(&[b'x'; 100], b"salt", 3)), "59bfa49750dd5462ce38370a1e7abe0736ff334cf1c2f0d84f23a03435d660d1");
        let salt: Vec<u8> = (0..16).collect();
        assert_eq!(hex_of(pbkdf2_sha256("maestra1".as_bytes(), &salt, 1000)), "460fbced00029f100253aa7c57fcc723779c2d7899e69f8d3c5628091c8efb48");
    }

    #[test]
    fn hash_and_verify() {
        let salt: Vec<u8> = (0..16).collect();
        let h = hash_password("maestra1", &salt, 1000);
        assert_eq!(h, "pbkdf2-sha256$1000$000102030405060708090a0b0c0d0e0f$460fbced00029f100253aa7c57fcc723779c2d7899e69f8d3c5628091c8efb48");
        assert_eq!(verify_password("maestra1", &h), Ok(true));
        assert_eq!(verify_password("maestra2", &h), Ok(false));
        assert_eq!(verify_password("", &h), Ok(false));
        // (never the password itself in the line)
        assert!(!h.contains("maestra1"));
        // (lines that aren't hashes, or ask for too much work, are refused)
        assert!(verify_password("x", "maestra1").is_err());
        assert!(verify_password("x", "md5$1$00$00").is_err());
        assert!(verify_password("x", "pbkdf2-sha256$0$00$00").is_err());
        assert!(verify_password("x", "pbkdf2-sha256$99999999999$00$00").is_err());
        assert!(verify_password("x", "pbkdf2-sha256$1000$zz$00").is_err());
        assert!(verify_password("x", "pbkdf2-sha256$1000$00$0011").is_err());
    }

    #[test]
    fn new_hashes_are_salted_and_slow() {
        let mut n = 0u8;
        let mut random = |b: &mut [u8]| {
            n += 1;
            b.fill(n);
            true
        };
        assert!(new_hash("short", &mut random).is_err());
        let a = new_hash("maestra1", &mut random).unwrap();
        let b = new_hash("maestra1", &mut random).unwrap();
        assert!(a.starts_with("pbkdf2-sha256$600000$"));
        // (the same password, another salt: another line)
        assert_ne!(a, b);
        assert_eq!(verify_password("maestra1", &a), Ok(true));
        assert!(new_hash("maestra1", |_| false).is_err());
    }

    #[test]
    fn strength_hints() {
        assert_eq!(strength("abc12"), 0);
        assert_eq!(strength("123456"), 1);
        assert_eq!(strength("abcdefgh"), 1);
        assert_eq!(strength("aaaaaaaa"), 1);
        assert_eq!(strength("maestra1"), 2);
        assert_eq!(strength("Maestra-2026!"), 3);
        assert_eq!(strength("profesoragarcia"), 2);
    }

    /// A lock with "maestra1", in few rounds (the tests' time).
    fn locked_with(features: &str) -> SchoolLock {
        let mut l = SchoolLock::default();
        l.turn_on(hash_password("maestra1", b"0123456789abcdef", 100), features).unwrap();
        l
    }

    #[test]
    fn wrong_tries_wait_a_minute() {
        let mut l = locked_with("ai,ai_apps");
        let t = 1_000_000.0;
        for k in 0..4 {
            assert_eq!(l.unlock("wrong", t + f64::from(k)), Outcome::Wrong);
        }
        assert_eq!(l.throttle.tries_left(), 1);
        assert_eq!(l.unlock("wrong", t + 4.0), Outcome::Wrong);
        // (the fifth: a minute's wait, even for the right password)
        assert_eq!(l.wait_left(t + 4.0), 60.0);
        assert_eq!(l.unlock("maestra1", t + 30.0), Outcome::Wait);
        assert_eq!(l.turn_off("maestra1", t + 30.0), Outcome::Wait);
        assert!(l.locked("ai"));
        assert_eq!(l.wait_left(t + 34.0), 30.0);
        // (after it: tried again; a right one clears the count)
        assert_eq!(l.unlock("maestra1", t + 65.0), Outcome::Right);
        assert!(!l.locked("ai") && l.on());
        assert_eq!(l.throttle, Throttle::default());
        // (five more wrong after a wait: another wait)
        l.relock();
        for k in 0..5 {
            assert_eq!(l.unlock("nope", t + 100.0 + f64::from(k)), Outcome::Wrong);
        }
        assert_eq!(l.unlock("maestra1", t + 110.0), Outcome::Wait);
        // (kept as text in Studio's folder and read back)
        assert_eq!(Throttle::parse(&l.throttle.to_text()), l.throttle);
        assert_eq!(Throttle::parse("junk"), Throttle::default());
    }

    #[test]
    fn unlock_lasts_until_relock_and_turn_off_needs_the_password() {
        let mut l = locked_with("ai");
        assert!(l.locked("ai") && !l.locked("ai_apps"));
        assert_eq!(l.unlock("maestra1", 0.0), Outcome::Right);
        assert!(!l.locked("ai") && l.on());
        l.relock();
        assert!(l.locked("ai"));
        assert_eq!(l.turn_off("maestra2", 0.0), Outcome::Wrong);
        assert!(l.on());
        assert_eq!(l.turn_off("maestra1", 0.0), Outcome::Right);
        assert!(!l.on() && !l.locked("ai"));
        assert_eq!(l.user.to_text(), "");
        // (nothing to lock: refused)
        assert!(l.turn_on("x".into(), "").is_err());
    }

    #[test]
    fn the_setting_round_trips_without_tabs() {
        let l = locked_with("ai_apps,ai,nonsense,ai");
        let text = l.user.to_text();
        assert!(!text.contains('\t') && !text.contains('\n'));
        assert!(text.starts_with("on=1;features=ai_apps,ai;hash=pbkdf2-sha256$100$"));
        assert_eq!(UserSetting::parse(&text), l.user);
        // (on with no password: off)
        assert!(!UserSetting::parse("on=1;features=ai").on);
        assert!(!UserSetting::parse("").on);
    }

    #[test]
    fn the_policy_wins() {
        let it = hash_password("it-secret", b"fedcba9876543210", 100);
        let mut l = locked_with("ai");
        l.policy = Some(parse_policy(&format!("school_lock = true\nlocked = [\"ai\", \"ai_apps\"]\npassword_hash = \"{it}\"\n"), "/p.toml"));
        assert!(l.by_policy() && l.locked("ai") && l.locked("ai_apps"));
        // (the user's password doesn't open the policy's lock; IT's does, for now)
        assert_eq!(l.unlock("maestra1", 0.0), Outcome::Wrong);
        assert_eq!(l.unlock("it-secret", 0.0), Outcome::Right);
        assert!(!l.locked("ai_apps"));
        // (it is never turned off from Studio, nor set there)
        assert_eq!(l.turn_off("it-secret", 0.0), Outcome::Policy);
        assert!(l.turn_on("x".into(), "ai").is_err());
        assert!(l.on());
        // (a policy that doesn't lock: the user's setting applies)
        let mut u = locked_with("ai");
        u.policy = Some(parse_policy("school_lock = false\n", "/p.toml"));
        assert!(!u.by_policy() && u.locked("ai") && !u.locked("ai_apps"));
        assert_eq!(u.turn_off("maestra1", 0.0), Outcome::Right);
        // (no setting, a policy that locks only AI apps)
        let mut p = SchoolLock { policy: Some(parse_policy(&format!("locked = [\"ai_apps\"]\npassword_hash = \"{it}\"\n"), "/p.toml")), ..Default::default() };
        assert!(p.on() && p.locked("ai_apps") && !p.locked("ai"));
        assert_eq!(p.unlock("it-secret", 0.0), Outcome::Right);
    }

    #[test]
    fn a_bad_policy_fails_closed() {
        for text in ["school_lock = maybe", "school_lock = 1\n", "locked = \"ai\"\n", "password_hash = 7\n", "password_hash = \"maestra1\"\n"] {
            let p = parse_policy(text, "/p.toml");
            assert!(p.lock && !p.error.is_empty(), "{text}: {p:?}");
            assert_eq!(p.features, vec!["ai".to_string(), "ai_apps".to_string()]);
            let mut l = SchoolLock { policy: Some(p), ..Default::default() };
            assert!(l.locked("ai") && l.locked("ai_apps"));
            assert_eq!(l.unlock("", 0.0), Outcome::NoPassword);
            assert_eq!(l.unlock("maestra1", 0.0), Outcome::NoPassword);
        }
        // (locked, but no password given: nobody unlocks it here)
        let mut l = SchoolLock { policy: Some(parse_policy("school_lock = true\n", "/p.toml")), ..Default::default() };
        assert!(l.locked("ai"));
        assert_eq!(l.unlock("anything", 0.0), Outcome::NoPassword);
    }

    #[test]
    fn the_test_file_never_replaces_the_computers() {
        // (the computer's file is read first; RAPIDR_SCHOOL_POLICY only when
        // there is none — here the read function plays the disk)
        let sys = system_policy_path().unwrap();
        // SAFETY of the test: no other test reads this variable.
        unsafe { std::env::set_var("RAPIDR_SCHOOL_POLICY", "/tmp/test-policy.toml") };
        let both = read_policy(|f| Ok(Some(if f == sys { "school_lock = true\nlocked = [\"ai\"]\n".into() } else { "school_lock = false\n".into() })));
        assert_eq!(both.as_ref().map(|p| (p.lock, p.file.clone())), Some((true, sys.clone())));
        let test_only = read_policy(|f| Ok(if f == sys { None } else { Some("locked = [\"ai_apps\"]\n".into()) }));
        assert_eq!(test_only.map(|p| (p.lock, p.features, p.file)), Some((true, vec!["ai_apps".to_string()], "/tmp/test-policy.toml".to_string())));
        let unreadable = read_policy(|f| if f == sys { Err("permission denied".into()) } else { Ok(None) });
        assert!(unreadable.is_some_and(|p| p.lock && p.error.contains("permission denied")));
        unsafe { std::env::remove_var("RAPIDR_SCHOOL_POLICY") };
        assert_eq!(read_policy(|_| Ok(None)), None);
    }
}
