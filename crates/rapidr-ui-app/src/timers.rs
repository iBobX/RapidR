//! QTIMER: one heap of deadlines for every timer the program made (moved
//! from runtime-core's `ui/kernel.rs`). A timer ticks only while the
//! program waits — a modal form, DOEVENTS, INPUT$, a dialog
//! — as RapidQ's WM_TIMERs come only while its message loop runs: the
//! runtime fires what's due between its host's turns ([`fire_due`]) and
//! waits no longer than the next deadline ([`next_due`]). Interval and
//! Enabled are read again at each tick; a timer is armed again from when
//! its handler has run.
//!
//! The desktop's step loop and tracking tick drive it; the web will from one
//! `setTimeout` for the earliest deadline (docs/web-host-plan.md §3.2).

use std::cell::{Cell, RefCell};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashSet};
use std::rc::Rc;
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
    /// The timers due wait for handlers queued ahead of them ([`hold_back`]).
    held_back: bool,
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
/// fires while the program waits: a modal form, DOEVENTS).
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

/// Whether a timer that was due at `since` hasn't fired yet (DOEVENTS
/// fires them all before it returns).
pub fn due_since(since: Instant) -> bool {
    tm(|s| s.heap.peek().is_some_and(|Reverse((at, _, _))| *at <= since))
}

/// The timers due wait for the handlers queued ahead of them (an
/// interpreter's, which run after this turn): the next turn fires them
/// first ([`take_held_back`]).
pub fn hold_back() {
    tm(|s| s.held_back = true);
}

/// Whether the timers due wait for handlers queued ahead of them.
pub fn held_back() -> bool {
    tm(|s| s.held_back)
}

/// [`held_back`], cleared: this turn fires them.
pub fn take_held_back() -> bool {
    tm(|s| std::mem::take(&mut s.held_back))
}

/// The timers due, each fired then armed again — once its handler has run
/// — with its Interval as it is then; one disabled meanwhile stops. `true`
/// when one's handler was left queued (an interpreter's): the others due
/// fire once it has run ([`hold_back`]).
pub fn fire_due<P: Program>(p: P) -> bool {
    fire_due_then(p, || ())
}

/// [`fire_due`], `then` run after each one's handler was fired (a
/// tracking tick: the VM's handler run before the next timer's).
///
/// A timer is armed again when its handler has *returned* (its event's
/// continuation, `Program::fire_then`): at once in a native build, where
/// the handler runs as it's fired; when the VM has run it in an
/// interpreter. So a timer whose handler waits — for a dialog, a modal form
/// — doesn't fire again until that handler is over. And the next timer due
/// fires only once the handler before it has run (or waits: then in that
/// wait's turns): one left queued ends the round, as a native build's
/// handler runs before the next timer is taken — a timer's handler is
/// never queued behind another that then waits for a dialog (silent all
/// through it).
pub fn fire_due_then<P: Program>(p: P, then: impl Fn()) -> bool {
    loop {
        let now = p.now();
        let due = tm(|s| match s.heap.peek() {
            Some(Reverse((at, _, _))) if *at <= now => s.heap.pop().map(|Reverse((_, _, n))| n),
            _ => None,
        });
        let Some(name) = due else { return false };
        if p.get(&name, "enabled").to_i64() == 0 {
            tm(|s| s.scheduled.remove(&name));
            continue;
        }
        // (a QDXTIMER with ActiveOnly while the program isn't active: not
        // fired, due again an Interval on)
        if !p.timer_firing(&name) {
            rearm(p, name);
            continue;
        }
        let ran = Rc::new(Cell::new(false));
        let (done, again) = (ran.clone(), name.clone());
        p.fire_then(
            &name,
            "ontimer",
            &[],
            Box::new(move |_| {
                done.set(true);
                rearm(p, again);
            }),
        );
        then();
        if !ran.get() {
            hold_back();
            return true;
        }
    }
}

/// Timer `name` (its handler over) due again an Interval from now.
fn rearm<P: Program>(p: P, name: String) {
    let at = p.now() + interval(p, &name);
    tm(|s| {
        s.gen += 1;
        let g = s.gen;
        s.heap.push(Reverse((at, g, name)));
    });
}
