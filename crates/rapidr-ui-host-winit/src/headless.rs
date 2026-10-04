//! The headless host: no OS windows, no event loop, no NSApplication. A
//! pump runs the program's window commands (there's nothing to show them
//! on) and sleeps until its timeout; the screen is a fixed 1920 × 1080 at
//! the forced scale (`RAPIDR_SCALE`, else 1). GUI tests run on it, without
//! a desktop session or a GPU; their captures are byte-identical to a
//! windowed host's (Stage 0).

use std::task::Waker;
use std::time::Duration;

use rapidr_ui_kernel::Store;

use crate::{Desktop, Host, HostCmd, HEADLESS_SCREEN};

pub struct HeadlessHost {
    scale: f64,
}

impl HeadlessHost {
    pub fn new(scale: f64) -> Self {
        HeadlessHost { scale }
    }
}

impl Host for HeadlessHost {
    fn pump(&mut self, timeout: Option<Duration>, desk: &mut Desktop, _store: &dyn Store) {
        for cmd in std::mem::take(&mut desk.cmds) {
            if let HostCmd::Show(f) = cmd {
                if let Some(form) = desk.form(&f) {
                    form.scale = self.scale;
                }
            }
        }
        if !desk.events.is_empty() {
            return;
        }
        match timeout {
            Some(t) => std::thread::sleep(t),
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
        Waker::noop().clone()
    }

    fn headless(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "headless"
    }
}
