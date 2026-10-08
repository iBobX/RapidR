//! Time in the kernel: what must happen later without input — the caret's
//! blink, a scroll bar's arrow repeating while held, a component's own
//! timer — as **deadlines** the host's step loop honours. The host asks
//! [`FormUi::next_wake`] (runtime-core's `step` pumps no longer than
//! that), then calls [`FormUi::tick`] when it's due; what changed is drawn
//! again and what the program must hear about is queued, as for input.
//!
//! A component asks for a tick by setting its `NodeUi::wake`; its
//! [`ComponentKind::tick`](crate::ComponentKind::tick) runs then. The
//! clock is [`now`] (a test can set it: [`set_test_now`]).

use std::cell::Cell;
use std::time::Duration;

/// The kernel's instants: std's everywhere but in a browser
/// (wasm32-unknown-unknown has no OS clock: std's `Instant::now` panics
/// there), where `web_time`'s reads `performance.now()` — the web host's
/// (docs/web-host-plan.md). On every other target it *is*
/// `std::time::Instant`, so the desktop host and runtime-core are
/// unchanged.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub use web_time::Instant;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub use std::time::Instant;

use crate::store::Store;
use crate::text::TextSystem;
use crate::tree::FormUi;

/// Windows' caret blink (GetCaretBlinkTime's default).
pub const BLINK: Duration = Duration::from_millis(530);
/// Windows' double-click time and distance (GetDoubleClickTime,
/// SM_CXDOUBLECLK / 2).
pub const DOUBLE_CLICK: Duration = Duration::from_millis(500);
pub const DOUBLE_CLICK_DISTANCE: f64 = 4.0;
/// A held scroll bar arrow repeats after this, then every [`REPEAT`].
pub const REPEAT_DELAY: Duration = Duration::from_millis(400);
pub const REPEAT: Duration = Duration::from_millis(50);

thread_local! {
    static TEST_NOW: Cell<Option<Instant>> = const { Cell::new(None) };
}

/// The kernel's clock.
pub fn now() -> Instant {
    TEST_NOW.with(Cell::get).unwrap_or_else(Instant::now)
}

/// Stops the clock at `at` (`None`: real time again) — for tests.
pub fn set_test_now(at: Option<Instant>) {
    TEST_NOW.with(|t| t.set(at));
}

/// A form's deadlines besides its components' (theirs are `NodeUi::wake`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Wakes {
    /// The caret's next blink (an editor has the focus).
    pub caret: Option<Instant>,
    /// A held scroll bar part's next repeat (`components::scrollbox`).
    pub bars: Option<Instant>,
}

impl FormUi {
    /// The caret shows again, its blink starting over (input, focus).
    pub fn reset_caret(&mut self) {
        self.caret_on = true;
        self.wakes.caret = (self.blinks && self.editor_focused()).then(|| now() + BLINK);
    }

    /// The caret's blink armed if an editor has the focus and it isn't yet
    /// (an editor focused before it was first drawn: after painting).
    pub(crate) fn arm_caret(&mut self) {
        if self.blinks && self.wakes.caret.is_none() && self.editor_focused() {
            self.wakes.caret = Some(now() + BLINK);
        }
    }

    /// Whether the focused component shows a caret.
    pub fn editor_focused(&self) -> bool {
        self.focus.is_some_and(|f| self.nodes[f].ui.edit.is_some() || self.nodes[f].ui.code.is_some())
    }

    /// When something is due next (`None`: nothing waits).
    pub fn next_wake(&self) -> Option<Instant> {
        let caret = self.wakes.caret.filter(|_| self.blinks && self.editor_focused());
        let nodes = self.nodes.iter().filter_map(|n| n.ui.wake);
        caret.into_iter().chain(self.wakes.bars).chain(self.tip_wake()).chain(nodes).min()
    }

    /// Runs what's due at `now`: the caret blinks, held scroll bars repeat,
    /// components' ticks.
    pub fn tick(&mut self, store: &dyn Store, ts: &mut TextSystem, now: Instant) {
        if let Some(at) = self.wakes.caret {
            if at <= now {
                if self.blinks && self.editor_focused() {
                    self.caret_on = !self.caret_on;
                    self.dirty = true;
                    self.wakes.caret = Some(now + BLINK);
                } else {
                    self.wakes.caret = None;
                }
            }
        } else if self.blinks && self.editor_focused() {
            self.wakes.caret = Some(now + BLINK);
        }
        // (a tooltip shows or goes: tooltip.rs)
        self.tip_tick(now);
        if self.wakes.bars.is_some_and(|at| at <= now) {
            self.wakes.bars = None;
            crate::components::scrollbox::bars_repeat(self, store);
        }
        for i in 0..self.nodes.len() {
            if self.nodes[i].ui.wake.is_some_and(|at| at <= now) {
                self.nodes[i].ui.wake = None;
                self.dirty = true;
                self.with_cx(store, ts, i, |k, cx| k.tick(cx));
            }
        }
    }
}
