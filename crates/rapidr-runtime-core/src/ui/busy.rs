//! `RAPIDR_TEST_BUSY=<file>` (a GUI test's hook, SEC-24 gated as the
//! others): how long the UI thread stays busy at a stretch — what a key
//! pressed at that moment waits for before it's handled (docs/speed-audit-
//! 2026-11.md, "Studio: the edit pause"). A stretch is the time between two
//! waits of the window loop that really waited (≥ 1 ms: a turn that found
//! more to do at once goes on the same stretch); each one of 1 ms or more is
//! written as `busy <events played> <start ms> <length ms> <thread CPU ms>`
//! — the last the UI thread's own CPU time so far, at the stretch's end (a
//! worker's isn't in it) —, and at the end `thread-cpu <ms>`. The benchmark
//! (`tools/bench/ui/studio_caret.py`) takes the stretches after its lead's
//! events, and the CPU between them in the same run.

use std::cell::RefCell;
use std::time::{Duration, Instant};

struct Busy {
    path: String,
    /// The process's first wait (stretches' starts are from it).
    origin: Instant,
    /// When the current stretch started (the last real wait's end).
    since: Instant,
    /// The waits being timed: when this one started.
    wait_from: Option<Instant>,
    lines: Vec<String>,
}

thread_local! {
    static BUSY: RefCell<Option<Busy>> = const { RefCell::new(None) };
    static ASKED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// A wait — the host's turn — starts.
pub fn wait_starts() {
    if !ASKED.with(|a| a.replace(true)) {
        if let Some(path) = rapidr_ui_app::testhooks::var("RAPIDR_TEST_BUSY").filter(|p| !p.is_empty()) {
            let now = Instant::now();
            BUSY.with(|b| *b.borrow_mut() = Some(Busy { path, origin: now, since: now, wait_from: None, lines: Vec::new() }));
        }
    }
    BUSY.with(|b| {
        if let Some(b) = b.borrow_mut().as_mut() {
            b.wait_from = Some(Instant::now());
        }
    });
}

/// The wait ended: a real one ends the stretch before it.
pub fn wait_ended() {
    BUSY.with(|b| {
        let mut b = b.borrow_mut();
        let Some(b) = b.as_mut() else { return };
        let Some(from) = b.wait_from.take() else { return };
        let now = Instant::now();
        if now.duration_since(from) < Duration::from_millis(1) {
            return;
        }
        let len = from.duration_since(b.since);
        if len >= Duration::from_millis(1) {
            let played = rapidr_ui_app::script::events_played();
            b.lines.push(format!("busy {played} {:.1} {:.3} {:.3}", b.since.duration_since(b.origin).as_secs_f64() * 1000.0, len.as_secs_f64() * 1000.0, thread_cpu_ms()));
        }
        b.since = now;
    });
}

/// The program ends: the stretches written.
pub fn finish() {
    let Some(b) = BUSY.with(|b| b.borrow_mut().take()) else { return };
    let mut text = b.lines.join("\n");
    text.push_str(&format!("\nthread-cpu {:.1}\n", thread_cpu_ms()));
    if let Err(e) = std::fs::write(&b.path, text) {
        eprintln!("[rapidr] can't write {}: {e}", b.path);
    }
}

/// This thread's CPU time so far (milliseconds; 0 where it can't be read).
fn thread_cpu_ms() -> f64 {
    #[cfg(unix)]
    {
        let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
        // SAFETY: clock_gettime only writes the timespec it is given.
        if unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut ts) } == 0 {
            return ts.tv_sec as f64 * 1000.0 + ts.tv_nsec as f64 / 1e6;
        }
    }
    0.0
}
