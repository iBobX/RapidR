//! `rapidr school-lock`: School Lock for a school's IT (docs/manual/for-teachers.md).
//!
//!   rapidr school-lock hash            the password asked twice (not shown as
//!                                      typed); prints the policy file's
//!                                      `password_hash = "…"` line
//!   rapidr school-lock hash --stdin    the password from the first line of
//!                                      standard input (a script's)
//!   rapidr school-lock check [FILE]    reads the computer's policy (or FILE)
//!                                      and says what it locks
//!
//! The hash is rapidr-studio's (`school_lock`): PBKDF2-HMAC-SHA256,
//! 600,000 rounds, a random salt. The password itself is never printed or
//! written anywhere.

use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use rapidr_studio::school_lock;

pub fn command(args: &[String]) -> ExitCode {
    match args.first().map(String::as_str) {
        Some("hash") => hash(args.iter().any(|a| a == "--stdin")),
        Some("check") => check(args.get(1).map(String::as_str)),
        _ => {
            eprintln!("rapidr school-lock hash [--stdin]   A password's hash line for the computer's policy file (the password is never shown or kept)");
            eprintln!("rapidr school-lock check [FILE]     What the computer's policy (or FILE) locks, and whether it can be read");
            eprintln!("The policy file: {}", school_lock::system_policy_path().unwrap_or_default());
            ExitCode::from(2)
        }
    }
}

fn hash(from_stdin: bool) -> ExitCode {
    let password = if from_stdin {
        let mut line = String::new();
        if io::stdin().lock().read_line(&mut line).is_err() {
            eprintln!("rapidr school-lock hash: nothing could be read from standard input");
            return ExitCode::from(1);
        }
        line.trim_end_matches(['\r', '\n']).to_string()
    } else {
        let Some(first) = ask("Password: ") else { return ExitCode::from(1) };
        if school_lock::strength(&first) == 0 {
            eprintln!("The password needs at least {} characters.", school_lock::MIN_PASSWORD);
            return ExitCode::from(1);
        }
        let Some(again) = ask("Type it again: ") else { return ExitCode::from(1) };
        if first != again {
            eprintln!("The two passwords differ: nothing was made.");
            return ExitCode::from(1);
        }
        first
    };
    match school_lock::new_hash_here(&password) {
        Ok(h) => {
            if !from_stdin && school_lock::strength(&password) == 1 {
                eprintln!("(This password is easy to guess: a longer one, with numbers or symbols, is safer.)");
            }
            println!("password_hash = \"{h}\"");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("rapidr school-lock hash: {e}");
            ExitCode::from(1)
        }
    }
}

/// A password typed at the terminal, not shown as it is typed.
fn ask(prompt: &str) -> Option<String> {
    eprint!("{prompt}");
    let _ = io::stderr().flush();
    let echo = Echo::off();
    let mut line = String::new();
    let read = io::stdin().lock().read_line(&mut line);
    drop(echo);
    eprintln!();
    match read {
        Ok(0) | Err(_) => None,
        Ok(_) => Some(line.trim_end_matches(['\r', '\n']).to_string()),
    }
}

/// The terminal's echo turned off while it lives (on again when dropped).
struct Echo {
    #[cfg(unix)]
    saved: Option<libc::termios>,
    #[cfg(windows)]
    saved: Option<u32>,
}

#[cfg(unix)]
impl Echo {
    fn off() -> Echo {
        // SAFETY: tcgetattr / tcsetattr on standard input with a termios we own.
        unsafe {
            let mut t: libc::termios = std::mem::zeroed();
            if libc::isatty(0) == 0 || libc::tcgetattr(0, &mut t) != 0 {
                return Echo { saved: None };
            }
            let saved = t;
            t.c_lflag &= !libc::ECHO;
            t.c_lflag |= libc::ECHONL;
            libc::tcsetattr(0, libc::TCSANOW, &t);
            Echo { saved: Some(saved) }
        }
    }
}

#[cfg(unix)]
impl Drop for Echo {
    fn drop(&mut self) {
        if let Some(t) = self.saved {
            // SAFETY: the settings read in `off`, put back.
            unsafe {
                libc::tcsetattr(0, libc::TCSANOW, &t);
            }
        }
    }
}

#[cfg(windows)]
mod console {
    pub type Handle = *mut core::ffi::c_void;
    pub const STD_INPUT_HANDLE: u32 = -10i32 as u32;
    pub const ENABLE_ECHO_INPUT: u32 = 0x0004;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        pub fn GetStdHandle(which: u32) -> Handle;
        pub fn GetConsoleMode(console: Handle, mode: *mut u32) -> i32;
        pub fn SetConsoleMode(console: Handle, mode: u32) -> i32;
    }
}

#[cfg(windows)]
impl Echo {
    fn off() -> Echo {
        // SAFETY: the console's own input handle and its mode, read and set.
        unsafe {
            let h = console::GetStdHandle(console::STD_INPUT_HANDLE);
            let mut mode = 0u32;
            if h.is_null() || console::GetConsoleMode(h, &mut mode) == 0 {
                return Echo { saved: None };
            }
            console::SetConsoleMode(h, mode & !console::ENABLE_ECHO_INPUT);
            Echo { saved: Some(mode) }
        }
    }
}

#[cfg(windows)]
impl Drop for Echo {
    fn drop(&mut self) {
        if let Some(mode) = self.saved {
            // SAFETY: the mode read in `off`, put back.
            unsafe {
                console::SetConsoleMode(console::GetStdHandle(console::STD_INPUT_HANDLE), mode);
            }
        }
    }
}

#[cfg(not(any(unix, windows)))]
impl Echo {
    fn off() -> Echo {
        Echo {}
    }
}

fn check(file: Option<&str>) -> ExitCode {
    let policy = match file {
        Some(f) => match std::fs::read_to_string(f) {
            Ok(t) => Some(school_lock::parse_policy(&t, f)),
            Err(e) => {
                eprintln!("{f}: {e}");
                return ExitCode::from(1);
            }
        },
        None => school_lock::read_policy(|f| match std::fs::read_to_string(f) {
            Ok(t) => Ok(Some(t)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.to_string()),
        }),
    };
    let Some(p) = policy else {
        println!("No School Lock policy on this computer ({} isn't there).", school_lock::system_policy_path().unwrap_or_default());
        return ExitCode::SUCCESS;
    };
    if !p.error.is_empty() {
        println!("{}: {}.", p.file, p.error);
        println!("RapidR Studio keeps every feature locked, with no password, until the file is fixed.");
        return ExitCode::from(1);
    }
    if !p.lock {
        println!("{}: school_lock = false — no lock from this computer (each user's own School Lock still applies).", p.file);
        return ExitCode::SUCCESS;
    }
    let names: Vec<&str> = p
        .features
        .iter()
        .map(|f| match f.as_str() {
            "ai" => "RAI in Studio (ai)",
            "coach" => "RAI kept in Coach mode: hints, no changes to files (coach_only)",
            _ => "AI apps, MCP (ai_apps)",
        })
        .collect();
    println!("{}: School Lock is on for every user of this computer.", p.file);
    println!("Locked: {}.", if names.is_empty() { "nothing (locked = [])".to_string() } else { names.join(", ") });
    if p.hash.is_empty() {
        println!("No password_hash: nobody can unlock it in Studio.");
    } else {
        println!("Unlock: with the password password_hash was made from, until Studio closes. It can't be turned off in Studio.");
    }
    ExitCode::SUCCESS
}
