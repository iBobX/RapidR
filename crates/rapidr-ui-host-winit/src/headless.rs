//! The headless host: no OS windows, no event loop, no NSApplication. A
//! pump runs the program's window commands (there's nothing to show them
//! on) and sleeps until its timeout; the screen is a fixed 1920 × 1080 at
//! the forced scale (`RAPIDR_SCALE`, else 1). GUI tests run on it, without
//! a desktop session or a GPU; their captures are byte-identical to a
//! windowed host's (Stage 0).

use std::sync::{Arc, Condvar, Mutex};
use std::task::{Wake, Waker};
use std::time::Duration;

use rapidr_ui_kernel::Store;

use crate::{Desktop, Host, HostCmd, HEADLESS_SCREEN};

pub struct HeadlessHost {
    scale: f64,
    // (timers during native menu tracking)
    /// runtime-core's tick, and a GUI test's pretend hold of the next pump
    /// (`__hold_ms`: a native menu the user keeps open).
    hook: Option<crate::tracking::Hook>,
    hold: Option<Duration>,
    /// What wakes a pump before its timeout (another thread's [`Host::waker`]:
    /// a debugger's request reaching a program that waits).
    alarm: Arc<Alarm>,
}

/// A pump's alarm: rung by a waker from any thread.
#[derive(Default)]
struct Alarm {
    rung: Mutex<bool>,
    bell: Condvar,
}

impl Wake for Alarm {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        if let Ok(mut r) = self.rung.lock() {
            *r = true;
            self.bell.notify_all();
        }
    }
}

impl HeadlessHost {
    pub fn new(scale: f64) -> Self {
        HeadlessHost { scale, hook: None, hold: None, alarm: Arc::default() }
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
            // (until the timeout, or until a waker rings)
            Some(t) => {
                let mut rung = self.alarm.rung.lock().unwrap_or_else(|e| e.into_inner());
                if !*rung {
                    rung = match self.alarm.bell.wait_timeout(rung, t) {
                        Ok((g, _)) => g,
                        Err(e) => e.into_inner().0,
                    };
                }
                *rung = false;
            }
            None => {
                // (no user can ever come: only a test's script or a timer would)
                eprintln!("[rapidr] headless host: the program waits for input and nothing is scheduled (no timer, no test step)");
                std::process::exit(3);
            }
        }
    }

    fn screen(&self) -> (i64, i64) {
        HEADLESS_SCREEN
    }

    fn default_scale(&self) -> f64 {
        self.scale
    }

    fn waker(&self) -> Waker {
        Waker::from(self.alarm.clone())
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
