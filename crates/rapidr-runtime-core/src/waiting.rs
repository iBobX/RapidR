//! Builtins that hold the program until something outside it is done —
//! SHELLWAIT's command, a socket's read, a connection being made — wait
//! with the program's windows served (`ui::gui_serve_waiting`): they keep
//! painting and answering the system, so it never shows them as "not
//! responding" (RUNTIME-BUGS 2026-11-05: RapidR Web Server's window hung
//! while it waited for PHP). The program itself waits exactly as before —
//! none of its code runs until the builtin returns, as in RapidQ — and
//! without a window shown (a console program) the wait is the plain one.

use std::time::{Duration, Instant};

/// Whether a wait now has windows to serve.
pub fn serves() -> bool {
    #[cfg(feature = "gui")]
    {
        crate::ui::gui_waits_serve()
    }
    #[cfg(not(feature = "gui"))]
    {
        false
    }
}

/// The windows served for up to `max` (less when the system had something
/// for them); `false` when there are none to serve (nothing waited).
pub fn moment(max: Duration) -> bool {
    #[cfg(feature = "gui")]
    {
        crate::ui::gui_serve_waiting(max)
    }
    #[cfg(not(feature = "gui"))]
    {
        let _ = max;
        false
    }
}

/// How long each look waits: short at first (a quick command or answer is
/// heard at once), then a frame's time.
pub struct Steps(Duration);

impl Steps {
    pub fn new() -> Self {
        Steps(Duration::from_millis(1))
    }

    /// The next look's wait.
    pub fn next(&mut self) -> Duration {
        let d = self.0;
        self.0 = (self.0 * 2).min(Duration::from_millis(16));
        d
    }
}

impl Default for Steps {
    fn default() -> Self {
        Self::new()
    }
}

/// `work` (a blocking call: a connection made) run on a thread of its own
/// while the windows are served; run here when there are none.
pub fn off_thread<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    if !serves() {
        return work();
    }
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(work());
    });
    let mut steps = Steps::new();
    loop {
        match rx.try_recv() {
            Ok(v) => return v,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => panic!("RapidR: a background wait's thread ended without an answer"),
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
        if !moment(steps.next()) {
            return rx.recv().expect("RapidR: a background wait's thread ended without an answer");
        }
    }
}

/// A started command waited for (SHELLWAIT): its exit status.
pub fn child(child: &mut std::process::Child) -> std::io::Result<std::process::ExitStatus> {
    let mut steps = Steps::new();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        if !moment(steps.next()) {
            return child.wait();
        }
    }
}

/// What's left until `deadline` (`None`: no limit), `Some(0)` once past.
pub fn left(deadline: Option<Instant>) -> Option<Duration> {
    deadline.map(|d| d.saturating_duration_since(Instant::now()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_looks_start_short_and_grow_to_a_frame() {
        let mut s = Steps::new();
        let got: Vec<u64> = (0..7).map(|_| s.next().as_millis() as u64).collect();
        assert_eq!(got, [1, 2, 4, 8, 16, 16, 16]);
    }

    #[test]
    fn without_windows_the_waits_are_the_plain_ones() {
        // (no window shown in a unit test: nothing to serve)
        assert!(!serves());
        assert_eq!(off_thread(|| 41 + 1), 42);
        #[cfg(unix)]
        {
            let mut c = std::process::Command::new("sh").arg("-c").arg("exit 3").spawn().unwrap();
            assert_eq!(child(&mut c).unwrap().code(), Some(3));
        }
    }
}
