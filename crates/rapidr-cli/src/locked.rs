//! The new program of a native build, on Windows, can be out of reach after
//! cargo has said "Finished": opening it gets "Access is denied" (os error 5)
//! or a sharing / lock violation (32, 33). Two different things look the same:
//!
//! - the security software is *scanning* the new `.exe` (any antivirus opens a
//!   program it has not seen) and lets go in a moment: waited for, here;
//! - the security software *decided against* the program: a false alarm on a
//!   program it has never seen, which it denies and moves to its quarantine.
//!   The 2.121.0 release test hit this on Windows x64: a consumer antivirus
//!   called every natively built x64 test program a generic detection (its
//!   quarantine and pending-actions records name each one; the arm64 programs
//!   were left alone), so all six native cases failed with error 5 although
//!   nothing else was wrong. Waiting never helps, and RapidR can't overrule
//!   the antivirus: the person is told what happened and what to do
//!   ([`explain`]).
//!
//! Only on Windows: elsewhere those error numbers mean other things, and
//! nothing is retried or explained.

use std::io;
use std::path::Path;
use std::time::Duration;

/// ERROR_ACCESS_DENIED, ERROR_SHARING_VIOLATION, ERROR_LOCK_VIOLATION.
const LOCKED: [i32; 3] = [5, 32, 33];
/// The first wait; each next one is twice as long, up to [`LONGEST_WAIT`].
const FIRST_WAIT: Duration = Duration::from_millis(100);
const LONGEST_WAIT: Duration = Duration::from_secs(2);
/// How long the new program is waited for (all the waits together), to read it.
pub const PATIENCE: Duration = Duration::from_secs(10);
/// How long a best-effort step (cleaning up) waits: it must not hold the build.
pub const BRIEFLY: Duration = Duration::from_secs(2);

/// What a locked file looks like, on Windows.
pub fn is_locked(e: &io::Error) -> bool {
    e.raw_os_error().is_some_and(|code| LOCKED.contains(&code))
}

/// `op` run, again after a short wait while it fails because Windows holds
/// the file (up to `patience` in all); then its last error, unchanged
/// ([`explain`] says what it probably is). Not Windows: `op` run once.
pub fn while_locked<T>(patience: Duration, mut op: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    if cfg!(windows) {
        retry(patience, is_locked, std::thread::sleep, op)
    } else {
        op()
    }
}

/// The waiting itself, its clock and its idea of "locked" given (the tests'
/// simulated errors and sleeps).
fn retry<T>(patience: Duration, locked: impl Fn(&io::Error) -> bool, mut sleep: impl FnMut(Duration), mut op: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    let mut waited = Duration::ZERO;
    let mut wait = FIRST_WAIT;
    loop {
        match op() {
            Err(e) if locked(&e) => {
                if waited >= patience {
                    return Err(e);
                }
                let now = wait.min(patience - waited);
                sleep(now);
                waited += now;
                wait = (wait * 2).min(LONGEST_WAIT);
            }
            done => return done,
        }
    }
}

/// The error as the person reads it: `<program>: <the error>`, and on
/// Windows, for the errors above (and for a program that vanished after the
/// build), what it probably is and what to do — `folders` are the ones to put
/// in the antivirus' exceptions (RapidR's build folder, the folder the app
/// goes to). Elsewhere: just the first line.
pub fn explain(program: &Path, e: &io::Error, folders: &[&Path]) -> String {
    explain_for(cfg!(windows), program, e, folders)
}

fn explain_for(windows: bool, program: &Path, e: &io::Error, folders: &[&Path]) -> String {
    let mut text = format!("{}: {e}", program.display());
    if !windows {
        return text;
    }
    if is_locked(e) {
        text.push_str(&format!(
            "\nYour antivirus most likely blocked the new program, even after RapidR waited {} seconds. That is often a false alarm for a brand-new program.",
            PATIENCE.as_secs()
        ));
    } else if e.kind() == io::ErrorKind::NotFound {
        text.push_str("\nThe new program disappeared right after it was built. Your antivirus most likely moved it to its quarantine (often a false alarm for a brand-new program).");
    } else {
        return text;
    }
    text.push_str("\nLook in the antivirus' quarantine and allow the program, or add these folders to its exceptions:");
    for f in folders {
        text.push_str(&format!("\n  {}", f.display()));
    }
    text.push_str("\nThen build again. More: the manual, Troubleshooting > Antivirus (how to report a false alarm to the antivirus maker, and signing).");
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::path::PathBuf;

    fn denied() -> io::Error {
        io::Error::from_raw_os_error(5)
    }

    #[test]
    fn the_windows_errors_are_the_locked_ones() {
        for code in [5, 32, 33] {
            assert!(is_locked(&io::Error::from_raw_os_error(code)), "{code}");
        }
        for code in [2, 3, 80, 183] {
            assert!(!is_locked(&io::Error::from_raw_os_error(code)), "{code}");
        }
        // (an error that isn't the system's: no number)
        assert!(!is_locked(&io::Error::new(io::ErrorKind::PermissionDenied, "no code")));
    }

    #[test]
    fn retries_with_doubling_waits_until_it_works() {
        let slept = RefCell::new(Vec::new());
        let calls = Cell::new(0);
        let got = retry(
            PATIENCE,
            is_locked,
            |d| slept.borrow_mut().push(d),
            || {
                calls.set(calls.get() + 1);
                if calls.get() < 5 { Err(denied()) } else { Ok(calls.get()) }
            },
        );
        assert_eq!(got.unwrap(), 5);
        let ms: Vec<u128> = slept.borrow().iter().map(|d| d.as_millis()).collect();
        assert_eq!(ms, [100, 200, 400, 800]);
    }

    #[test]
    fn sharing_and_lock_violations_are_retried_too() {
        for code in [32, 33] {
            let calls = Cell::new(0);
            let got = retry(PATIENCE, is_locked, |_| {}, || {
                calls.set(calls.get() + 1);
                if calls.get() == 1 { Err(io::Error::from_raw_os_error(code)) } else { Ok(()) }
            });
            assert!(got.is_ok() && calls.get() == 2, "{code}");
        }
    }

    #[test]
    fn gives_up_after_the_patience_with_the_error_unchanged() {
        let slept = RefCell::new(Vec::new());
        let calls = Cell::new(0);
        let got: io::Result<()> = retry(PATIENCE, is_locked, |d| slept.borrow_mut().push(d), || {
            calls.set(calls.get() + 1);
            Err(denied())
        });
        let e = got.unwrap_err();
        // (the waits add up to the patience exactly, none longer than the cap)
        let total: Duration = slept.borrow().iter().sum();
        assert_eq!(total, PATIENCE);
        assert!(slept.borrow().iter().all(|d| *d <= LONGEST_WAIT));
        assert_eq!(calls.get(), slept.borrow().len() + 1);
        // (the first waits: 100 ms, doubling)
        assert_eq!(slept.borrow()[..3], [Duration::from_millis(100), Duration::from_millis(200), Duration::from_millis(400)]);
        // (the error is the one the file gave, nothing added: explain() says the rest)
        assert_eq!(e.raw_os_error(), Some(5));
        assert_eq!(e.to_string(), denied().to_string());
        assert_eq!(e.kind(), denied().kind());
    }

    #[test]
    fn other_errors_fail_at_once() {
        let slept = Cell::new(0);
        let calls = Cell::new(0);
        let got: io::Result<()> = retry(PATIENCE, is_locked, |_| slept.set(slept.get() + 1), || {
            calls.set(calls.get() + 1);
            Err(io::Error::from_raw_os_error(2))
        });
        let e = got.unwrap_err();
        assert_eq!((calls.get(), slept.get()), (1, 0));
        assert_eq!(e.raw_os_error(), Some(2));
    }

    #[test]
    fn no_patience_means_one_try() {
        let calls = Cell::new(0);
        let got: io::Result<()> = retry(Duration::ZERO, is_locked, |_| {}, || {
            calls.set(calls.get() + 1);
            Err(denied())
        });
        assert_eq!(calls.get(), 1);
        assert_eq!(got.unwrap_err().raw_os_error(), Some(5));
    }

    fn build_folders() -> (PathBuf, [PathBuf; 2]) {
        let exe = PathBuf::from(r"C:\Users\me\AppData\Local\RapidR\Cache\build\k\target\release\arithmetic.exe");
        (exe, [PathBuf::from(r"C:\Users\me\AppData\Local\RapidR\Cache\build\k\target"), PathBuf::from(r"C:\Users\me\apps")])
    }

    /// What 2.121.0's release test showed (Windows x64, a consumer antivirus):
    /// the program stays denied; the person is told what it is and what to do.
    #[test]
    fn a_denied_program_is_explained_with_the_folders_to_allow() {
        let (exe, folders) = build_folders();
        let refs: Vec<&Path> = folders.iter().map(|f| f.as_path()).collect();
        let text = explain_for(true, &exe, &denied(), &refs);
        // (the first line is the plain error, with the program's path)
        let first = text.lines().next().unwrap();
        assert_eq!(first, format!("{}: {}", exe.display(), denied()));
        assert!(first.contains("arithmetic.exe") && first.contains("(os error 5)"), "{first}");
        // (then: what it probably is, how long RapidR waited, what to do, the folders)
        assert!(text.contains("Your antivirus most likely blocked the new program"), "{text}");
        assert!(text.contains("often a false alarm for a brand-new program"), "{text}");
        assert!(text.contains("the manual, Troubleshooting > Antivirus"), "{text}");
        assert!(text.contains(&format!("{} seconds", PATIENCE.as_secs())), "{text}");
        assert!(text.contains("quarantine") && text.contains("exceptions") && text.contains("build again"), "{text}");
        for f in &folders {
            assert!(text.contains(&f.display().to_string()), "{text}");
        }
        // (no promise that waiting or trying again helps by itself)
        assert!(!text.contains("still checking") && !text.contains("try again"), "{text}");
    }

    #[test]
    fn a_sharing_violation_is_explained_too() {
        let (exe, folders) = build_folders();
        let refs: Vec<&Path> = folders.iter().map(|f| f.as_path()).collect();
        for code in [32, 33] {
            let text = explain_for(true, &exe, &io::Error::from_raw_os_error(code), &refs);
            assert!(text.contains("antivirus") && text.contains("exceptions"), "{code}: {text}");
        }
    }

    /// A program the antivirus took away (quarantine) after cargo said it was built.
    #[test]
    fn a_vanished_program_is_explained_as_probably_quarantined() {
        let (exe, folders) = build_folders();
        let refs: Vec<&Path> = folders.iter().map(|f| f.as_path()).collect();
        let text = explain_for(true, &exe, &io::Error::from(io::ErrorKind::NotFound), &refs);
        assert!(text.starts_with(&format!("{}: ", exe.display())), "{text}");
        assert!(text.contains("disappeared") && text.contains("quarantine") && text.contains("exceptions"), "{text}");
    }

    /// Any other error is only itself: one line.
    #[test]
    fn other_errors_are_not_explained() {
        let (exe, folders) = build_folders();
        let refs: Vec<&Path> = folders.iter().map(|f| f.as_path()).collect();
        let other = io::Error::from_raw_os_error(80);
        let text = explain_for(true, &exe, &other, &refs);
        assert_eq!(text, format!("{}: {}", exe.display(), other));
    }

    /// Not Windows: error 5 is EIO, 32 is EPIPE; nothing about an antivirus.
    #[test]
    fn elsewhere_only_the_error_is_shown() {
        let (exe, folders) = build_folders();
        let refs: Vec<&Path> = folders.iter().map(|f| f.as_path()).collect();
        let text = explain_for(false, &exe, &denied(), &refs);
        assert_eq!(text, format!("{}: {}", exe.display(), denied()));
        let vanished = io::Error::from(io::ErrorKind::NotFound);
        assert_eq!(explain_for(false, &exe, &vanished, &refs).lines().count(), 1);
        // (and the real entry point follows the platform it runs on)
        assert_eq!(explain(&exe, &denied(), &refs).lines().count() > 1, cfg!(windows));
    }

    /// Not Windows: the error numbers mean other things, and nothing waits.
    #[cfg(not(windows))]
    #[test]
    fn elsewhere_it_runs_once() {
        let calls = Cell::new(0);
        let got: io::Result<()> = while_locked(PATIENCE, || {
            calls.set(calls.get() + 1);
            Err(denied())
        });
        assert_eq!(calls.get(), 1);
        assert_eq!(got.unwrap_err().raw_os_error(), Some(5));
    }

    /// The real thing, Windows: a file another program holds open (nobody
    /// else may share it) is read once it lets go.
    #[cfg(windows)]
    #[test]
    fn a_held_file_is_read_when_it_is_let_go() {
        use std::os::windows::fs::OpenOptionsExt;
        let path = std::env::temp_dir().join(format!("rapidr-locked-test-{}.exe", std::process::id()));
        std::fs::write(&path, b"MZ new program").unwrap();
        let held = std::fs::OpenOptions::new().read(true).share_mode(0).open(&path).unwrap();
        // (held for about half a second, as the antivirus scan does)
        let releaser = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(600));
            drop(held);
        });
        let first = std::fs::read(&path).unwrap_err();
        assert!(is_locked(&first), "{first}");
        let bytes = while_locked(PATIENCE, || std::fs::read(&path)).unwrap();
        assert_eq!(bytes, b"MZ new program");
        releaser.join().unwrap();
        std::fs::remove_file(&path).unwrap();
    }

    /// And a file that stays held: the error after the patience, and what to do about it.
    #[cfg(windows)]
    #[test]
    fn a_file_that_stays_held_fails_with_the_explanation() {
        use std::os::windows::fs::OpenOptionsExt;
        let path = std::env::temp_dir().join(format!("rapidr-locked-test2-{}.exe", std::process::id()));
        std::fs::write(&path, b"MZ").unwrap();
        let _held = std::fs::OpenOptions::new().read(true).share_mode(0).open(&path).unwrap();
        let e = while_locked(Duration::from_millis(700), || std::fs::read(&path)).unwrap_err();
        assert!(is_locked(&e), "{e}");
        let text = explain(&path, &e, &[std::env::temp_dir().as_path()]);
        assert!(text.contains("antivirus") && text.contains("exceptions"), "{text}");
        drop(_held);
        std::fs::remove_file(&path).unwrap();
    }
}
