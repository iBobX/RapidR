//! A program cargo has just linked is held for a moment on Windows: the
//! antivirus (Windows Defender's real-time scan) opens a new `.exe` to check
//! it, and whatever opens it meanwhile gets "Access is denied" (os error 5)
//! or a sharing / lock violation (32, 33) — although cargo said "Finished".
//! The release smoke test failed this way on Windows x64 (2.121.0). Waiting
//! a little makes it pass: this is that wait, for the places that touch the
//! new program (cache.rs, main.rs). Only on Windows: elsewhere those error
//! numbers mean other things, and nothing is retried.

use std::io;
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
/// the file (up to `patience` in all); then its error, with a hint what it
/// probably is. Not Windows: `op` run once.
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
                    return Err(still_locked(e));
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

/// The error it was, and what to do about it, in plain words.
fn still_locked(e: io::Error) -> io::Error {
    io::Error::new(e.kind(), format!("{e}\nWindows is still checking the new program (antivirus); try again"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

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
    fn gives_up_after_the_patience_with_the_error_and_a_hint() {
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
        // (the original error stays what it was, and the hint says what it probably is)
        let text = e.to_string();
        assert!(text.contains(&denied().to_string()), "{text}");
        assert!(text.contains("Windows is still checking the new program (antivirus); try again"), "{text}");
        assert_eq!(e.kind(), denied().kind());
    }

    #[test]
    fn other_errors_fail_at_once_without_a_hint() {
        let slept = Cell::new(0);
        let calls = Cell::new(0);
        let got: io::Result<()> = retry(PATIENCE, is_locked, |_| slept.set(slept.get() + 1), || {
            calls.set(calls.get() + 1);
            Err(io::Error::from_raw_os_error(2))
        });
        let e = got.unwrap_err();
        assert_eq!((calls.get(), slept.get()), (1, 0));
        assert_eq!(e.raw_os_error(), Some(2));
        assert!(!e.to_string().contains("antivirus"));
    }

    #[test]
    fn no_patience_means_one_try_and_the_hint() {
        let calls = Cell::new(0);
        let got: io::Result<()> = retry(Duration::ZERO, is_locked, |_| {}, || {
            calls.set(calls.get() + 1);
            Err(denied())
        });
        assert_eq!(calls.get(), 1);
        assert!(got.unwrap_err().to_string().contains("antivirus"));
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

    /// And a file that stays held: the error, with the hint, after the patience.
    #[cfg(windows)]
    #[test]
    fn a_file_that_stays_held_fails_with_the_hint() {
        use std::os::windows::fs::OpenOptionsExt;
        let path = std::env::temp_dir().join(format!("rapidr-locked-test2-{}.exe", std::process::id()));
        std::fs::write(&path, b"MZ").unwrap();
        let _held = std::fs::OpenOptions::new().read(true).share_mode(0).open(&path).unwrap();
        let e = while_locked(Duration::from_millis(700), || std::fs::read(&path)).unwrap_err();
        assert!(e.to_string().contains("Windows is still checking the new program"), "{e}");
        drop(_held);
        std::fs::remove_file(&path).unwrap();
    }
}
