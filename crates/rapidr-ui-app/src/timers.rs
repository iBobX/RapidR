//! QTIMER: one heap of deadlines for every timer the program made (moved
//! from runtime-core's `ui/kernel.rs`). A timer ticks only while the
//! program waits — a modal form, the main loop, DOEVENTS, INPUT$, a dialog
//! — as RapidQ's WM_TIMERs come only while its message loop runs: the
//! runtime fires what's due between its host's turns ([`fire_due`]) and
//! waits no longer than the next deadline ([`next_due`]). Interval and
//! Enabled are read again at each tick; a timer is armed again from when
//! its handler has run.
//!
//! The desktop's step loop and tracking tick drive it; the web will from one
//! `setTimeout` for the earliest deadline (docs/web-host-plan.md §3.2).

use std::cell::RefCell;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashSet};
use std::time::Duration;

use rapidr_ui_kernel::tick::Instant;

use crate::{Program, Windows};

#[derive(Default)]
struct Timers {
    /// QTIMERs the program made, those ticking, and when they're due.
    timers: Vec<String>,
    scheduled: HashSet<String>,
    heap: BinaryHeap<Reverse<(Instant, u64, String)>>,
    gen: u64,
}

thread_local! {
    static TIMERS: RefCell<Timers> = RefCell::new(Timers::default());
}

fn tm<R>(f: impl FnOnce(&mut Timers) -> R) -> R {
    TIMERS.with(|t| f(&mut t.borrow_mut()))
}

/// A QTIMER the program made (generated programs call it).
pub fn register(name: &str) {
    let name = name.to_lowercase();
    tm(|s| {
        if !s.timers.contains(&name) {
            s.timers.push(name);
        }
    });
}

/// Every timer the program made starts ticking (if enabled, and not
/// already): the program waits.
pub fn start_all<P: Program>(p: P) {
    for t in tm(|s| s.timers.clone()) {
        schedule(p, &t);
    }
}

fn interval<P: Program>(p: P, name: &str) -> Duration {
    if let Some(d) = p.timer_period(name) {
        return d;
    }
    let ms = p.get(name, "interval").to_i64();
    Duration::from_millis(if ms > 0 { ms as u64 } else { 1000 })
}

/// Starts timer `name` ticking if it's enabled and isn't already (it
/// fires while the program waits: a modal form, the main loop, DOEVENTS).
pub fn schedule<P: Program>(p: P, name: &str) {
    let name = name.to_lowercase();
    if p.get(&name, "enabled").to_i64() == 0 || !tm(|s| s.scheduled.insert(name.clone())) {
        return;
    }
    let at = p.now() + interval(p, &name);
    tm(|s| {
        s.gen += 1;
        let g = s.gen;
        s.heap.push(Reverse((at, g, name)));
    });
}

/// A timer's Enabled or Interval changed: it ticks if it's enabled now.
pub fn changed<R: Program + Windows>(rt: R, name: &str) {
    if rt.started() {
        schedule(rt, name);
    }
}

/// When the next timer is due.
pub fn next_due() -> Option<Instant> {
    tm(|s| s.heap.peek().map(|Reverse((at, _, _))| *at))
}

/// The timers due, each fired then armed again from now with its Interval
/// as it is then; one disabled meanwhile stops.
pub fn fire_due<P: Program>(p: P) {
    fire_due_then(p, || ());
}

/// [`fire_due`], `then` run after each one's handler was fired (a
/// tracking tick: the VM's handler run before the next timer's).
pub fn fire_due_then<P: Program>(p: P, then: impl Fn()) {
    loop {
        let now = p.now();
        let due = tm(|s| match s.heap.peek() {
            Some(Reverse((at, _, _))) if *at <= now => s.heap.pop().map(|Reverse((_, _, n))| n),
            _ => None,
        });
        let Some(name) = due else { break };
        if p.get(&name, "enabled").to_i64() == 0 {
            tm(|s| s.scheduled.remove(&name));
            continue;
        }
        if p.timer_firing(&name) {
            p.fire(&name, "ontimer");
            then();
        }
        let at = p.now() + interval(p, &name);
        tm(|s| {
            s.gen += 1;
            let g = s.gen;
            s.heap.push(Reverse((at, g, name)));
        });
    }
}
