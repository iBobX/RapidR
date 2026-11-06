//! Where a compiler of hot code plugs into the VM (docs/jit-design.md):
//! the VM counts loop turns and calls, and at the places compiled code can
//! start — a loop's head reached by a backward jump, a SUB / FUNCTION just
//! called — lends a [`JitView`] of itself to its [`Jit`]. The JIT runs its
//! code there, and leaves the frames, the stack and the globals as the
//! interpreter would have them at the instruction it stopped before; the
//! VM goes on from the top frame's `ip`.
//!
//! The VM itself has no compiled code and no `unsafe` (`rapidr-jit` has
//! both, in one module): without a JIT installed — the web runtime, a
//! platform without one, `RAPIDR_JIT=off` — nothing here costs more than a
//! flag tested at a backward jump and a call.

use std::sync::atomic::AtomicBool;

use rapidr_bytecode::Module;
use rapidr_value::Value;

use crate::Frame;

/// The VM lent to the JIT where compiled code may start.
pub struct JitView<'v> {
    pub module: &'v Module,
    pub frames: &'v mut Vec<Frame>,
    /// The operand stack (the top frame's part is from its `stack_base`).
    pub stack: &'v mut Vec<Value>,
    /// Globals by their name's string id ([`crate::Vm::globals`]).
    pub globals: &'v mut Vec<Option<Value>>,
    /// Under the debugger, the flag its commands raise: compiled code
    /// checks it at every backward jump and every call it makes, and comes
    /// back to the interpreter (which serves the pause) when it is set.
    /// `None` outside debug mode: nothing is waiting for the program there.
    pub interrupt: Option<&'v AtomicBool>,
    /// How many more frames may be pushed before the "stack overflow"
    /// run-time error ([`crate::MAX_CALL_DEPTH`]).
    pub depth_room: usize,
    /// The host lets the VM call the pure maths builtins itself
    /// ([`crate::Host::DIRECT_MATHS`]): compiled code may too.
    pub direct_maths: bool,
}

/// What the JIT did at a place compiled code may start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JitAnswer {
    /// Compiled code ran: the VM goes on from the top frame's `ip` (the
    /// frames, the stack and the globals are as the interpreter would have
    /// them there).
    pub ran: bool,
    /// Ask again after this many more turns of the function's loops (or
    /// calls of it): 1 = every time, `u32::MAX` = never.
    pub again_after: u32,
}

impl JitAnswer {
    pub const NOT_YET: JitAnswer = JitAnswer { ran: false, again_after: u32::MAX };
}

/// A compiler of hot code (rapidr-jit).
pub trait Jit {
    /// The top frame's function reached its loop head `ip` by a backward
    /// jump, `turns` turns after the last time it asked: compile the loop
    /// if it is hot, run its code if there is some.
    fn loop_head(&mut self, view: JitView<'_>, ip: usize, turns: u32) -> JitAnswer;
    /// A frame for its function was just pushed (`ip` 0, its arguments in
    /// its locals), `calls` calls after the last time it asked.
    fn called(&mut self, view: JitView<'_>, calls: u32) -> JitAnswer;
    /// Turns and calls before the first question for a function (`always`:
    /// 1, to compile everything it can at once — the tests' way).
    fn first_ask(&self) -> (u32, u32);
    /// What it did, for `rapidr run --verbose` (compiled functions, the
    /// times it went back to the interpreter).
    fn report(&self) -> String {
        String::new()
    }
    /// [`Jit::report`], each place it names with its statement: `text`
    /// gives the compiled program's line N (the runtime has it when it
    /// compiled the program itself; a `.rrbc` has none).
    fn report_with(&self, text: &dyn Fn(u32) -> Option<String>) -> String {
        let _ = text;
        self.report()
    }
}

/// The countdowns of one function: loop turns and calls before the VM
/// asks the JIT again (and what each was set to: the JIT is told how many
/// went by).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Countdown {
    pub(crate) turns: u32,
    turns_from: u32,
    pub(crate) calls: u32,
    calls_from: u32,
}

impl Countdown {
    pub(crate) fn new(turns: u32, calls: u32) -> Self {
        let (turns, calls) = (turns.max(1), calls.max(1));
        Countdown { turns, turns_from: turns, calls, calls_from: calls }
    }

    pub(crate) fn turns_set(&self) -> u32 {
        self.turns_from
    }

    pub(crate) fn set_turns(&mut self, n: u32) {
        self.turns = n.max(1);
        self.turns_from = self.turns;
    }

    pub(crate) fn calls_set(&self) -> u32 {
        self.calls_from
    }

    pub(crate) fn set_calls(&mut self, n: u32) {
        self.calls = n.max(1);
        self.calls_from = self.calls;
    }
}
