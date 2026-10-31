//! The headless host: no OS windows, no event loop, no NSApplication. A
//! pump runs the program's window commands (there's nothing to show them
//! on) and sleeps until its timeout; the screen is a fixed 1920 × 1080 at
//! the forced scale (`RAPIDR_SCALE`, else 1). GUI tests run on it, without
//! a desktop session or a GPU; their captures are byte-identical to a
//! windowed host's (Stage 0).

use std::sync::Arc;
use std::task::{Wake, Waker};
use std::time::Duration;

use rapidr_ui_kernel::Store;

use crate::{Desktop, Host, HostCmd, HEADLESS_SCREEN};

pub struct HeadlessHost {
    scale: f64,
    /// The thread that pumps (the one that made the host): a waker
    /// unparks it ([`Host::waker`]).
    pumper: std::thread::Thread,
    // (timers during native menu tracking)
    /// runtime-core's tick, and a GUI test's pretend hold of the next pump
    /// (`__hold_ms`: a native menu the user keeps open).
    hook: Option<crate::tracking::Hook>,
    hold: Option<Duration>,
}

impl HeadlessHost {
    pub fn new(scale: f64) -> Self {
        HeadlessHost { scale, pumper: std::thread::current(), hook: None, hold: None }
    }
}

/// Wakes a pump waiting on the headless host: its thread unparked.
struct Unpark(std::thread::Thread);

impl Wake for Unpark {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.unpark();
    }
}

impl Host for HeadlessHost {
    fn pump(&mut self, timeout: Option<Duration>, desk: &mut Desktop, store: &dyn Store) {
        for cmd in std::mem::take(&mut desk.cmds) {
            match cmd {
                HostCmd::Show(f) => {
                    if let Some(form) = desk.form(&f) {
                        form.scale = self.scale;
                        form.state = form.spec.state;
                    }
                }
                // (the WindowState lane's: there's no window to maximize —
                // runtime-core moves the form itself, as the work area)
                HostCmd::State(f) => {
                    if let Some(form) = desk.form(&f) {
                        form.state = form.spec.state;
                    }
                }
                // (the input lane's: a size grip's drag resizes at once, as
                // the user's drag of the border would)
                HostCmd::Resize { form, w, h } => desk.resized(&form, w, h),
                _ => {}
            }
        }
        if !desk.events.is_empty() {
            return;
        }
        // (a pretend hold: the pump comes back once it's over, the program's
        // timers ticking meanwhile as on a real host's held pump)
        if let Some(hold) = self.hold.take() {
            let first = timeout.map(|t| std::time::Instant::now() + t);
            let hook = &mut self.hook;
            crate::tracking::simulate_hold(hold, first, || hook.as_mut().map_or_else(Default::default, |h| h(desk, store)));
            return;
        }
        match timeout {
            // (until the timeout, or a waker's call: work from another
            // thread — a Studio session's program said something — wants
            // the loop's turn now; an early turn only loops once more)
            // (waiting: the hang watchdog's "idle", rapidr-crash)
            Some(t) => {
                rapidr_crash::watchdog::idle();
                std::thread::park_timeout(t);
                rapidr_crash::watchdog::busy();
            }
            None => {
                // (no user can ever come: only a test's script or a timer would)
                eprintln!("[rapidr] headless host: the program waits for input and nothing is scheduled (no timer, no test step)");
                std::process::exit(3);
            }
        }
    }

    fn screen(&self) -> (i64, i64) {
        // (demo mode: the script's frame — docs/video-pipeline.md; a test's
        // small screen: RAPIDR_TEST_SCREEN)
        rapidr_ui_app::demo::screen().or_else(rapidr_ui_app::testhooks::screen).unwrap_or(HEADLESS_SCREEN)
    }

    fn default_scale(&self) -> f64 {
        self.scale
    }

    fn waker(&self) -> Waker {
        Waker::from(Arc::new(Unpark(self.pumper.clone())))
    }

    fn headless(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "headless"
    }

    fn set_tracking_hook(&mut self, hook: crate::tracking::Hook) {
        self.hook = Some(hook);
    }

    fn hold(&mut self, hold: Duration) {
        self.hold = Some(hold);
    }
}
