//! RapidR bytecode VM — host-agnostic interpreter.
//!
//! The VM is a stack machine. All side effects (I/O, GUI, builtins,
//! component creation, event registration) flow through the [`Host`] trait,
//! which is implemented separately for the desktop runtime
//! (`rapidr-vm-host-native`) and the web runtime (`rapidr-vm-host-web`).
//!
//! # Example
//!
//! ```
//! use rapidr_vm::{Vm, StubHost};
//! use rapidr_bytecode::{Module, Function, Const, Op};
//!
//! let mut m = Module::new();
//! let c_hello = m.add_const(Const::Str("hello".into()));
//! let mut f = Function::default();
//! f.name = "__main".into();
//! f.code.push(Op::LoadConst as u8);
//! f.code.extend_from_slice(&c_hello.to_le_bytes());
//! f.code.push(Op::PrintLn as u8);
//! f.code.push(Op::Halt as u8);
//! m.entry = m.add_function(f);
//!
//! let mut host = StubHost::default();
//! let mut vm = Vm::new(&mut host);
//! vm.run(&m).unwrap();
//! assert_eq!(host.output, "hello\n");
//! ```

#![forbid(unsafe_code)]

pub mod host;

pub use host::{Host, StubHost};
pub use rapidr_bytecode as bytecode;
pub use rapidr_value::Value;

use rapidr_bytecode::{Module, Op};
use rapidr_value::events::QueuedEvent;
use rapidr_value::{v_array, v_bool, v_int, v_null, v_str};

#[derive(Debug)]
pub enum VmError {
    StackUnderflow,
    BadOpcode(u8),
    BadOperand,
    BadFunctionIndex(u32),
    BadConstIndex(u32),
    BadStringIndex(u32),
    BadLocalSlot(u16),
    Truncated,
    HostError(String),
    /// A BASIC run-time error (e.g. "Subscript out of range").
    Runtime(String),
    Halted,
    Paused,
    /// The host asked to wait (an in-page dialog on the web): the VM kept
    /// its state; continue with [`Vm::resume_with`] and the host's answer.
    Suspended,
    /// The program has had its time slice ([`Host::yield_now`]): the VM
    /// kept its state; continue with [`Vm::resume`] once the host has had a
    /// turn (the web page repaints, takes clicks).
    Yielded,
    /// An error, and the source line it happened at: the file and line
    /// (`Module::source_map`) or just the compiled line.
    At { error: Box<VmError>, file: Option<String>, line: u32 },
}

impl std::fmt::Display for VmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VmError::At { error, file: Some(file), line } => write!(f, "{error} (at {file} line {line})"),
            VmError::At { error, file: None, line } => write!(f, "{error} (at line {line})"),
            VmError::StackUnderflow => write!(f, "stack underflow"),
            VmError::BadOpcode(b) => write!(f, "unknown opcode 0x{b:02X}"),
            VmError::BadOperand => write!(f, "operand decode failed"),
            VmError::BadFunctionIndex(i) => write!(f, "bad function index {i}"),
            VmError::BadConstIndex(i) => write!(f, "bad const index {i}"),
            VmError::BadStringIndex(i) => write!(f, "bad string index {i}"),
            VmError::BadLocalSlot(s) => write!(f, "bad local slot {s}"),
            VmError::Truncated => write!(f, "truncated bytecode"),
            // (a builtin's error is the program's run-time error, worded as
            // native builds word it)
            VmError::HostError(s) => write!(f, "run-time error: {s}"),
            VmError::Runtime(s) => write!(f, "run-time error: {s}"),
            VmError::Halted => write!(f, "halted"),
            VmError::Paused => write!(f, "paused"),
            VmError::Suspended => write!(f, "suspended"),
            VmError::Yielded => write!(f, "yielded"),
        }
    }
}

impl std::error::Error for VmError {}

/// Deepest nesting of SUB/FUNCTION calls, so runaway recursion stops with
/// an error instead of exhausting memory.
pub const MAX_CALL_DEPTH: usize = 100_000;

/// Most dimensions an array access may have.
const MAX_DIMS: usize = 8;

/// Jumps and calls between two [`Host::yield_now`] questions (hosts that
/// yield only).
const YIELD_CHECK_EVERY: u32 = 256;

/// Width of a PRINT zone (`PRINT a, b`), as in QBasic and VB.
pub const PRINT_ZONE_WIDTH: usize = 14;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepMode {
    None,
    Into,
    Over { target_depth: usize },
    Out { target_depth: usize },
}

/// Why the VM stopped for the debugger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StopReason {
    /// A breakpoint's line was reached.
    #[default]
    Breakpoint,
    /// A step (in, over, out) ended.
    Step,
    /// The debugger asked it to pause ([`Vm::request_pause`]).
    Pause,
    /// A run-time error, with [`Vm::break_on_error`] on: stopped at the
    /// faulting statement, before the error unwinds anything.
    Exception,
}

/// Where and why the VM stopped (handed to a [`Debugger`]).
#[derive(Debug, Clone)]
pub struct StopInfo {
    pub reason: StopReason,
    /// The compiled line (map it with `Module::source_map`).
    pub line: Option<u32>,
    /// The error, when `reason` is [`StopReason::Exception`].
    pub error: Option<String>,
}

/// How a [`Debugger`] lets the VM go on after a stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resume {
    Continue,
    StepIn,
    StepOver,
    StepOut,
    /// End the program (as END does).
    Terminate,
}

/// A debugger that serves stops where they happen: the VM calls it at the
/// stop, with the VM and the program to inspect (stack, variables,
/// [`Vm::evaluate`]), and goes on as it says. A host that can block (the
/// desktop, its commands arriving on another thread) installs one; a host
/// that can't (the web page) has none, and the VM returns
/// [`VmError::Paused`] instead, to be continued with [`Vm::resume`] & co.
pub trait Debugger<H: Host + ?Sized> {
    /// The VM stopped: inspect it, then say how to go on.
    fn stopped(&mut self, vm: &mut Vm<'_, H>, module: &Module, stop: &StopInfo) -> Resume;
    /// [`Vm::interrupt`] was raised (a command waits: new breakpoints, a
    /// pause): handle what's pending; `true` stops here (a pause).
    fn interrupted(&mut self, vm: &mut Vm<'_, H>, module: &Module) -> bool;
}

/// What [`Vm::debug_point`] decided.
enum DebugAction {
    Run,
    Pause,
    Halt,
}

/// Instructions an evaluation may run before it's stopped
/// ([`Vm::evaluate`]): a watch with an endless loop must not hang the IDE.
pub const EVAL_FUEL: u64 = 5_000_000;

/// One activation frame.
#[derive(Debug, Clone)]
pub struct Frame {
    pub fn_index: u32,
    pub locals: Vec<Value>,
    /// Saved instruction pointer in the calling function.
    pub ret_ip: usize,
    /// True if caller wanted a value (CallFunc), false for CallSub.
    pub wants_value: bool,
    /// Current instruction pointer in this frame.
    pub ip: usize,
    /// Return addresses of active GOSUBs in this frame.
    pub gosub: Vec<usize>,
    /// An event handler's entry frame ([`Vm::invoke_function`]): when it
    /// returns, the VM stops and gives control back to whoever ran the
    /// handler. The frames below it are what the handler interrupted.
    pub stop: bool,
    /// Suspended in a builtin or INPUT, waiting for [`Vm::resume_with`].
    pub waiting: bool,
    /// An event handler's entry frame: what the runtime does once it has
    /// run (`rapidr_value::events`), handed to the host when it returns.
    pub then: Vec<u32>,
    /// An event handler's entry frame run right after a host operation
    /// ([`Vm::after_host`]): the frame below was running, and continues
    /// once the handler has returned.
    pub nested: bool,
    /// Height of the value stack when the frame started (its arguments
    /// taken): what's left once it's gone (a failed handler, unwound after
    /// the debugger stopped at its error).
    pub stack_base: usize,
}

/// One turn of a host's wait ([`Vm::pump_wait`]).
enum WaitTurn {
    /// [`Host::pump`]'s answer: `None` while the wait goes on.
    Pumped(Option<Value>),
    /// A handler run during the turn ENDed the program.
    Ended,
}

/// How a returning frame leaves the VM.
#[derive(PartialEq, Eq)]
enum Returned {
    /// Back in the caller: keep executing.
    Continue,
    /// The entry frame or an event handler's frame finished: stop.
    Stop,
}

/// The interpreter.
pub struct Vm<'h, H: Host + ?Sized> {
    pub host: &'h mut H,
    pub stack: Vec<Value>,
    pub frames: Vec<Frame>,
    /// Globals by the index of their name in the module's string table
    /// (`LoadGlobal`/`StoreGlobal`'s operand): a vector index, not a lookup
    /// by name. `None` = never assigned.
    pub globals: Vec<Option<Value>>,
    /// Parameter values of the most recently returned frame, read by
    /// `LoadArgOut` right after a call to write BYREF arguments back.
    pub arg_out: Vec<Value>,
    /// Column of the output cursor (chars since the last newline), for
    /// `PrintZone`.
    pub print_col: usize,

    // Debugger state
    pub debug_mode: bool,
    /// Compiled lines to stop at: the union of [`Self::set_breakpoints`]'
    /// and every file's [`Self::set_file_breakpoints`].
    pub breakpoints: std::collections::HashSet<u32>,
    /// Breakpoints by file (its name, lower case): compiled lines.
    file_breakpoints: Vec<(String, std::collections::HashSet<u32>)>,
    /// Compiled lines set directly ([`Self::set_breakpoints`]).
    line_breakpoints: std::collections::HashSet<u32>,
    pub step_mode: StepMode,
    pub last_line: u32,
    /// Raised by another thread (or a message handler) for the VM's
    /// attention: at its next instruction (in debug mode) it calls the
    /// [`Debugger`]'s `interrupted`, or without one, pauses.
    pub interrupt: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// The debugger that serves stops in place (see [`Debugger`]).
    pub debugger: Option<Box<dyn Debugger<H>>>,
    /// Stop at a run-time error's statement (debug mode), before it
    /// unwinds: [`StopReason::Exception`].
    pub break_on_error: bool,
    /// Why the VM last stopped.
    pub stop_reason: StopReason,
    /// The error of the last [`StopReason::Exception`] stop.
    pub stop_error: Option<String>,
    /// The error the VM stopped at, without a [`Debugger`]: it unwinds as
    /// it would have when the program goes on ([`Self::resume`]).
    pending_error: Option<VmError>,
    /// Instructions left to an evaluation ([`Self::evaluate`]); `Some`
    /// while one runs.
    eval_fuel: Option<u64>,
    /// Locals vectors of returned frames, reused by the next calls (no
    /// allocation per SUB/FUNCTION call).
    spare_locals: Vec<Vec<Value>>,
    /// Offset of the instruction being run, for [`Self::error_line`].
    fault_ip: usize,
    /// The source line (of the compiled, preprocessed program) of the
    /// instruction a run-time error stopped at (also in the error:
    /// [`VmError::At`]).
    pub error_line: Option<u32>,
    /// Jumps and calls left before the next [`Host::yield_now`].
    ticks: u32,
    /// Whether the event handler that last returned ran right after a host
    /// operation (its frame's `nested`).
    returned_nested: bool,
    /// Events queued behind a handler that yielded, by the depth of the
    /// frame it interrupted: they run once it returns (then that frame
    /// continues), as they would have without the yield.
    yield_rest: Vec<(usize, Vec<QueuedEvent>)>,
}

impl<'h, H: Host + ?Sized> Vm<'h, H> {
    pub fn new(host: &'h mut H) -> Self {
        Self {
            host,
            stack: Vec::with_capacity(64),
            frames: Vec::with_capacity(8),
            globals: Vec::new(),
            arg_out: Vec::new(),
            print_col: 0,
            debug_mode: false,
            breakpoints: Default::default(),
            file_breakpoints: Vec::new(),
            line_breakpoints: Default::default(),
            step_mode: StepMode::None,
            last_line: 0,
            interrupt: Default::default(),
            debugger: None,
            break_on_error: false,
            stop_reason: StopReason::default(),
            stop_error: None,
            pending_error: None,
            eval_fuel: None,
            spare_locals: Vec::new(),
            fault_ip: 0,
            error_line: None,
            ticks: YIELD_CHECK_EVERY,
            returned_nested: false,
            yield_rest: Vec::new(),
        }
    }

    /// The globals that have been assigned, by name (debuggers).
    pub fn global_values<'m>(&'m self, module: &'m Module) -> impl Iterator<Item = (&'m str, &'m Value)> {
        self.globals.iter().enumerate().filter_map(|(i, v)| Some((module.strings.get(i)?.as_str(), v.as_ref()?)))
    }

    /// Borrow the host (e.g. to inspect output or registered events).
    pub fn host(&self) -> &H { self.host }

    /// Mutable borrow of the host.
    pub fn host_mut(&mut self) -> &mut H { self.host }

    pub fn run(&mut self, module: &Module) -> Result<(), VmError> {
        let entry = module.entry;
        self.call(module, entry, 0, false)?;
        self.exec(module)
    }

    /// Push a new frame for `fn_index`. Pops `argc` values from the stack
    /// to seed the parameter slots (in reverse order — last pushed = last
    /// parameter).
    pub fn call(&mut self, module: &Module, fn_index: u32, argc: u8, wants_value: bool) -> Result<(), VmError> {
        if self.frames.len() >= MAX_CALL_DEPTH {
            return Err(VmError::Runtime(format!("stack overflow: more than {MAX_CALL_DEPTH} nested calls (a SUB or FUNCTION calling itself without end?)")));
        }
        let f = module.functions.get(fn_index as usize)
            .ok_or(VmError::BadFunctionIndex(fn_index))?;
        let mut locals = self.spare_locals.pop().unwrap_or_default();
        locals.clear();
        locals.resize(f.n_locals as usize, Value::Null);
        // Pop args off the stack into the parameter slots. Extra arguments
        // (e.g. the Sender a host passes to a handler that declares no
        // parameters) are dropped rather than overwriting the first locals.
        let n_params = f.params.len().min(locals.len());
        for i in (0..argc as usize).rev() {
            let v = self.pop()?;
            if i < n_params {
                locals[i] = v;
            }
        }
        let ret_ip = self.frames.last().map(|fr| fr.locals.len() /* unused */ ).unwrap_or(0);
        // ret_ip placeholder — replaced by exec() loop's saved ip on push.
        let stack_base = self.stack.len();
        self.frames.push(Frame { fn_index, locals, ret_ip, wants_value, ip: 0, gosub: Vec::new(), stop: false, waiting: false, then: Vec::new(), nested: false, stack_base });
        let _ = ret_ip;
        Ok(())
    }

    /// Runs from the top frame until the program ends (`Ok` with no frames
    /// left), an event handler's frame returns (`Ok` with its value pushed
    /// and the interrupted frames below), or it pauses/suspends/fails.
    /// Runs from the top frame (see [`Self::exec_loop`]); a run-time error
    /// records its line in [`Self::error_line`].
    fn exec(&mut self, module: &Module) -> Result<(), VmError> {
        match self.exec_loop(module) {
            Err(e) if !matches!(e, VmError::Paused | VmError::Suspended | VmError::Yielded | VmError::At { .. }) => {
                self.error_line = self
                    .frames
                    .last()
                    .and_then(|f| module.functions.get(f.fn_index as usize))
                    .and_then(|f| f.get_line_for_ip(self.fault_ip));
                let error = match self.error_line {
                    None => e,
                    Some(line) => {
                        let (file, line) = match module.source_map.locate(line) {
                            Some((file, l)) => (Some(file.to_string()), l),
                            None => (None, line),
                        };
                        VmError::At { error: Box::new(e), file, line }
                    }
                };
                // The debugger stops at the faulting statement first, with
                // every frame still there.
                if self.debug_mode && self.break_on_error && self.eval_fuel.is_none() && !self.frames.is_empty() {
                    let fault_ip = self.fault_ip;
                    self.frames.last_mut().unwrap().ip = fault_ip;
                    let description = match &error {
                        VmError::At { error, .. } => error.to_string(),
                        other => other.to_string(),
                    };
                    self.stop_reason = StopReason::Exception;
                    self.stop_error = Some(description.clone());
                    self.step_mode = StepMode::None;
                    if let Some(mut debugger) = self.debugger.take() {
                        let stop = StopInfo { reason: StopReason::Exception, line: self.error_line, error: Some(description) };
                        let _ = debugger.stopped(self, module, &stop);
                        self.debugger = Some(debugger);
                    } else {
                        self.pending_error = Some(error);
                        return Err(VmError::Paused);
                    }
                }
                self.yield_rest.clear();
                Err(error)
            }
            other => other,
        }
    }

    /// Stops the debugger's way: [`Self::debug_mode`]'s check before an
    /// instruction — the interrupt, then (on a new line) a breakpoint or the
    /// end of a step. With a [`Debugger`], it serves the stop here.
    fn debug_point(&mut self, module: &Module, ip: usize) -> DebugAction {
        let top = self.frames.last().unwrap();
        let at = module.functions[top.fn_index as usize].line_at(ip);
        let line = at.map(|(line, _)| line);
        let mut reason = None;
        if self.interrupt.load(std::sync::atomic::Ordering::Relaxed) {
            self.interrupt.store(false, std::sync::atomic::Ordering::Relaxed);
            self.frames.last_mut().unwrap().ip = ip;
            let pause = match self.debugger.take() {
                Some(mut debugger) => {
                    let pause = debugger.interrupted(self, module);
                    self.debugger = Some(debugger);
                    pause
                }
                None => true,
            };
            if pause {
                reason = Some(StopReason::Pause);
            }
        }
        if let Some((line, start)) = at {
            if self.last_line != line {
                if reason.is_none() {
                    let depth = self.frames.len();
                    // (at a statement's start: not where a call returns
                    // into the middle of the line that made it)
                    if start && self.breakpoints.contains(&line) {
                        reason = Some(StopReason::Breakpoint);
                    } else if match self.step_mode {
                        StepMode::Into => true,
                        StepMode::Over { target_depth } => depth <= target_depth,
                        StepMode::Out { target_depth } => depth < target_depth,
                        StepMode::None => false,
                    } {
                        reason = Some(StopReason::Step);
                    }
                }
                self.last_line = line;
            }
        }
        let Some(reason) = reason else { return DebugAction::Run };
        self.frames.last_mut().unwrap().ip = ip;
        self.step_mode = StepMode::None;
        self.stop_reason = reason;
        self.stop_error = None;
        let Some(mut debugger) = self.debugger.take() else { return DebugAction::Pause };
        let resume = debugger.stopped(self, module, &StopInfo { reason, line, error: None });
        self.debugger = Some(debugger);
        if resume == Resume::Terminate {
            return DebugAction::Halt;
        }
        self.apply_resume(resume);
        DebugAction::Run
    }

    /// How the program goes on from a stop.
    fn apply_resume(&mut self, resume: Resume) {
        let depth = self.frames.len();
        self.step_mode = match resume {
            Resume::Continue | Resume::Terminate => StepMode::None,
            Resume::StepIn => StepMode::Into,
            Resume::StepOver => StepMode::Over { target_depth: depth },
            Resume::StepOut => StepMode::Out { target_depth: depth },
        };
    }

    fn exec_loop(&mut self, module: &Module) -> Result<(), VmError> {
        if self.frames.is_empty() {
            return Ok(());
        }
        // Per-frame instruction pointer; we keep it on the Rust stack for hot loop.
        let mut ip = self.frames.last().map(|f| f.ip).unwrap_or(0);
        // The currently executing function's code, refreshed on call/ret.
        let mut code: &[u8] = &module.functions[self.frames.last().unwrap().fn_index as usize].code;

        macro_rules! refresh {
            () => {
                code = &module.functions[self.frames.last().unwrap().fn_index as usize].code;
            };
        }
        // Where a loop or a recursion can run on (a jump, a call): a host
        // that yields ([`Host::YIELDS`], compiled out elsewhere) is asked
        // every so often whether the program has had its time slice; then
        // the VM stops here, ready to continue at `ip`.
        macro_rules! tick {
            () => {
                if H::YIELDS && self.eval_fuel.is_none() {
                    self.ticks -= 1;
                    if self.ticks == 0 {
                        self.ticks = YIELD_CHECK_EVERY;
                        if self.host.yield_now() {
                            self.frames.last_mut().unwrap().ip = ip;
                            return Err(VmError::Yielded);
                        }
                    }
                }
            };
        }
        // After a host operation: run the events it queued (and serve a
        // wait it started); stop here if a handler ENDed the program.
        macro_rules! after_host {
            ($has_result:expr) => {
                if !self.after_host(module, ip, $has_result)? {
                    return Ok(());
                }
            };
        }

        loop {
            if ip >= code.len() {
                // Implicit return for missing trailing Halt.
                if self.return_frame(module, false)? == Returned::Stop {
                    return Ok(());
                }
                let top = self.frames.last().unwrap();
                ip = top.ip;
                refresh!();
                continue;
            }

            if self.debug_mode {
                if let Some(fuel) = &mut self.eval_fuel {
                    // (an evaluation: no stops, a bounded number of steps)
                    if *fuel == 0 {
                        return Err(VmError::Runtime("the evaluation ran too long and was stopped".into()));
                    }
                    *fuel -= 1;
                } else {
                    match self.debug_point(module, ip) {
                        DebugAction::Run => {
                            // (a debugger served a stop here: the frames are
                            // as they were, an evaluation came and went)
                            refresh!();
                        }
                        DebugAction::Pause => return Err(VmError::Paused),
                        DebugAction::Halt => {
                            self.frames.clear();
                            self.stack.clear();
                            self.yield_rest.clear();
                            return Ok(());
                        }
                    }
                }
            }

            self.fault_ip = ip;
            let opbyte = code[ip];
            ip += 1;
            let op = Op::from_u8(opbyte).ok_or(VmError::BadOpcode(opbyte))?;
            match op {
                Op::Nop => {}
                // END (and the main program's last instruction): the whole
                // program stops, including code an event handler interrupted.
                Op::Halt => {
                    if self.eval_fuel.is_some() {
                        return Err(VmError::Runtime("END can't run in an evaluation".into()));
                    }
                    self.frames.clear();
                    self.stack.clear();
                    self.yield_rest.clear();
                    return Ok(());
                }

                // ----- constants / stack -----
                Op::LoadConst => {
                    let i = read_u32(code, &mut ip)?;
                    let c = module.consts.get(i as usize).ok_or(VmError::BadConstIndex(i))?;
                    self.stack.push(c.to_value());
                }
                Op::LoadNull => self.stack.push(v_null()),
                Op::LoadTrue => self.stack.push(v_bool(true)),
                Op::LoadFalse => self.stack.push(v_bool(false)),
                Op::Pop => { self.pop()?; }
                Op::Dup => {
                    let v = self.peek()?.clone();
                    self.stack.push(v);
                }

                // ----- locals / globals -----
                Op::LoadLocal => {
                    let s = read_u16(code, &mut ip)?;
                    let frame = self.frames.last().unwrap();
                    let v = frame.locals.get(s as usize).cloned()
                        .ok_or(VmError::BadLocalSlot(s))?;
                    self.stack.push(v);
                }
                Op::StoreLocal => {
                    let s = read_u16(code, &mut ip)?;
                    let v = self.pop()?;
                    let frame = self.frames.last_mut().unwrap();
                    let slot = frame.locals.get_mut(s as usize).ok_or(VmError::BadLocalSlot(s))?;
                    *slot = v;
                }
                Op::LoadGlobal => {
                    let i = read_u32(code, &mut ip)? as usize;
                    let v = self.globals.get(i).and_then(|v| v.clone()).unwrap_or(Value::Null);
                    self.stack.push(v);
                }
                Op::StoreGlobal => {
                    let i = read_u32(code, &mut ip)? as usize;
                    if i >= module.strings.len() {
                        return Err(VmError::BadStringIndex(i as u32));
                    }
                    let v = self.pop()?;
                    if i >= self.globals.len() {
                        self.globals.resize(module.strings.len(), None);
                    }
                    self.globals[i] = Some(v);
                }

                // ----- arithmetic -----
                Op::Add => { let b = self.pop()?; let a = self.pop()?; self.stack.push(&a + &b); }
                Op::Sub => { let b = self.pop()?; let a = self.pop()?; self.stack.push(&a - &b); }
                Op::Mul => { let b = self.pop()?; let a = self.pop()?; self.stack.push(&a * &b); }
                Op::Div => { let b = self.pop()?; let a = self.pop()?; self.stack.push(&a / &b); }
                Op::IDiv => { let b = self.pop()?; let a = self.pop()?; self.stack.push(a.checked_int_div(&b).map_err(|e| VmError::Runtime(e.into()))?); }
                Op::Mod => { let b = self.pop()?; let a = self.pop()?; self.stack.push(a.checked_mod(&b).map_err(|e| VmError::Runtime(e.into()))?); }
                Op::Pow => { let b = self.pop()?; let a = self.pop()?; self.stack.push(a.power(&b)); }
                Op::Neg => { let a = self.pop()?; self.stack.push(-&a); }
                Op::Concat => { let b = self.pop()?; let a = self.pop()?; self.stack.push(a.concat(&b)); }

                // ----- comparison -----
                Op::Eq => { let b = self.pop()?; let a = self.pop()?; self.stack.push(a.rp_eq(&b)); }
                Op::Ne => { let b = self.pop()?; let a = self.pop()?; self.stack.push(a.rp_ne(&b)); }
                Op::Lt => { let b = self.pop()?; let a = self.pop()?; self.stack.push(a.rp_lt(&b)); }
                Op::Le => { let b = self.pop()?; let a = self.pop()?; self.stack.push(a.rp_le(&b)); }
                Op::Gt => { let b = self.pop()?; let a = self.pop()?; self.stack.push(a.rp_gt(&b)); }
                Op::Ge => { let b = self.pop()?; let a = self.pop()?; self.stack.push(a.rp_ge(&b)); }

                // ----- logical -----
                Op::And => { let b = self.pop()?; let a = self.pop()?; self.stack.push(a.and(&b)); }
                Op::Or  => { let b = self.pop()?; let a = self.pop()?; self.stack.push(a.or(&b)); }
                Op::Xor => { let b = self.pop()?; let a = self.pop()?; self.stack.push(a.xor(&b)); }
                Op::Not => { let a = self.pop()?; self.stack.push(a.not()); }

                // ----- bitwise -----
                Op::BAnd => { let b = self.pop()?.to_i64(); let a = self.pop()?.to_i64(); self.stack.push(v_int(a & b)); }
                Op::BOr  => { let b = self.pop()?.to_i64(); let a = self.pop()?.to_i64(); self.stack.push(v_int(a | b)); }
                Op::BXor => { let b = self.pop()?.to_i64(); let a = self.pop()?.to_i64(); self.stack.push(v_int(a ^ b)); }
                Op::BNot => { let a = self.pop()?.to_i64(); self.stack.push(v_int(!a)); }
                Op::Shl  => { let b = self.pop()?.to_i64(); let a = self.pop()?.to_i64(); self.stack.push(v_int(a.wrapping_shl(b as u32))); }
                Op::Shr  => { let b = self.pop()?.to_i64(); let a = self.pop()?.to_i64(); self.stack.push(v_int(a.wrapping_shr(b as u32))); }

                // ----- control flow -----
                Op::Jump => { let t = read_u32(code, &mut ip)?; ip = t as usize; tick!(); }
                Op::JumpIf => {
                    let t = read_u32(code, &mut ip)?;
                    let v = self.pop()?;
                    if v.to_bool() { ip = t as usize; tick!(); }
                }
                Op::JumpIfNot => {
                    let t = read_u32(code, &mut ip)?;
                    let v = self.pop()?;
                    if !v.to_bool() { ip = t as usize; tick!(); }
                }

                // ----- calls -----
                Op::CallSub => {
                    let fi = read_u32(code, &mut ip)?;
                    let argc = read_u8(code, &mut ip)?;
                    self.frames.last_mut().unwrap().ip = ip;
                    self.frames.last_mut().unwrap().ret_ip = ip;
                    self.call(module, fi, argc, false)?;
                    ip = 0;
                    refresh!();
                    tick!();
                }
                Op::CallFunc => {
                    let fi = read_u32(code, &mut ip)?;
                    let argc = read_u8(code, &mut ip)?;
                    self.frames.last_mut().unwrap().ip = ip;
                    self.frames.last_mut().unwrap().ret_ip = ip;
                    self.call(module, fi, argc, true)?;
                    ip = 0;
                    refresh!();
                    tick!();
                }
                Op::Ret => {
                    if self.return_frame(module, false)? == Returned::Stop { return Ok(()); }
                    ip = self.frames.last().unwrap().ip;
                    refresh!();
                }
                Op::RetVal => {
                    if self.return_frame(module, true)? == Returned::Stop { return Ok(()); }
                    ip = self.frames.last().unwrap().ip;
                    refresh!();
                }
                Op::Gosub => {
                    let target = read_u32(code, &mut ip)? as usize;
                    self.frames.last_mut().unwrap().gosub.push(ip);
                    ip = target;
                    tick!();
                }
                Op::GosubRet => {
                    if let Some(back) = self.frames.last_mut().unwrap().gosub.pop() {
                        ip = back;
                    }
                }
                Op::LoadArgOut => {
                    let k = read_u8(code, &mut ip)? as usize;
                    self.stack.push(self.arg_out.get(k).cloned().unwrap_or_else(v_null));
                }
                Op::CallBuiltin => {
                    let name_i = read_u32(code, &mut ip)?;
                    let argc = read_u8(code, &mut ip)? as usize;
                    let name = module.strings.get(name_i as usize).ok_or(VmError::BadStringIndex(name_i))?.clone();
                    let mut args = Vec::with_capacity(argc);
                    for _ in 0..argc { args.push(self.pop()?); }
                    args.reverse();
                    let r = self.host.call_builtin(&name, &args).map_err(VmError::HostError)?;
                    if self.host.suspend_requested() {
                        let top = self.frames.last_mut().unwrap();
                        top.ip = ip;
                        top.waiting = true;
                        return Err(VmError::Suspended);
                    }
                    self.stack.push(r);
                    after_host!(true);
                }

                // ----- components -----
                Op::CreateComp => {
                    let kind_i = read_u32(code, &mut ip)?;
                    let id_i = read_u32(code, &mut ip)?;
                    let kind = module.strings.get(kind_i as usize).ok_or(VmError::BadStringIndex(kind_i))?.clone();
                    let id = module.strings.get(id_i as usize).ok_or(VmError::BadStringIndex(id_i))?.clone();
                    let r = self.host.create_comp(&kind, &id).map_err(VmError::HostError)?;
                    self.stack.push(r);
                    after_host!(true);
                }
                Op::SetProp => {
                    let id_i = read_u32(code, &mut ip)?;
                    let prop_i = read_u32(code, &mut ip)?;
                    let id = module.strings.get(id_i as usize).ok_or(VmError::BadStringIndex(id_i))?.clone();
                    let prop = module.strings.get(prop_i as usize).ok_or(VmError::BadStringIndex(prop_i))?.clone();
                    let v = self.pop()?;
                    self.host.set_prop(&id, &prop, v).map_err(VmError::HostError)?;
                    after_host!(false);
                }
                Op::GetProp => {
                    let id_i = read_u32(code, &mut ip)?;
                    let prop_i = read_u32(code, &mut ip)?;
                    let id = module.strings.get(id_i as usize).ok_or(VmError::BadStringIndex(id_i))?.clone();
                    let prop = module.strings.get(prop_i as usize).ok_or(VmError::BadStringIndex(prop_i))?.clone();
                    let v = self.host.get_prop(&id, &prop).map_err(VmError::HostError)?;
                    self.stack.push(v);
                    after_host!(true);
                }
                Op::CallMethod => {
                    let id_i = read_u32(code, &mut ip)?;
                    let m_i = read_u32(code, &mut ip)?;
                    let argc = read_u8(code, &mut ip)? as usize;
                    let id = module.strings.get(id_i as usize).ok_or(VmError::BadStringIndex(id_i))?.clone();
                    let m = module.strings.get(m_i as usize).ok_or(VmError::BadStringIndex(m_i))?.clone();
                    let mut args = Vec::with_capacity(argc);
                    for _ in 0..argc { args.push(self.pop()?); }
                    args.reverse();
                    let r = self.host.call_method(&id, &m, &args).map_err(VmError::HostError)?;
                    // A method that waits (the web's ShowModal): suspended
                    // until the host resumes with its result.
                    if self.host.suspend_requested() {
                        let top = self.frames.last_mut().unwrap();
                        top.ip = ip;
                        top.waiting = true;
                        return Err(VmError::Suspended);
                    }
                    self.stack.push(r);
                    after_host!(true);
                }
                Op::GetPropDyn => {
                    let prop_i = read_u32(code, &mut ip)?;
                    let prop = module.strings.get(prop_i as usize).ok_or(VmError::BadStringIndex(prop_i))?.clone();
                    let id = self.pop_object_id()?;
                    let v = self.host.get_prop(&id, &prop).map_err(VmError::HostError)?;
                    self.stack.push(v);
                    after_host!(true);
                }
                Op::SetPropDyn => {
                    let prop_i = read_u32(code, &mut ip)?;
                    let prop = module.strings.get(prop_i as usize).ok_or(VmError::BadStringIndex(prop_i))?.clone();
                    let id = self.pop_object_id()?;
                    let v = self.pop()?;
                    self.host.set_prop(&id, &prop, v).map_err(VmError::HostError)?;
                    after_host!(false);
                }
                Op::GetField => {
                    let slot = read_u16(code, &mut ip)? as usize;
                    match self.pop()? {
                        Value::Object(o) => self.stack.push(o.get(slot)),
                        other => return Err(VmError::Runtime(format!("this variable does not refer to an object (reading a field of {:?})", other.to_string_val()))),
                    }
                }
                Op::SetField => {
                    let slot = read_u16(code, &mut ip)? as usize;
                    let value = self.pop()?;
                    match self.pop()? {
                        Value::Object(o) => o.set(slot, value),
                        other => return Err(VmError::Runtime(format!("this variable does not refer to an object (setting a field of {:?})", other.to_string_val()))),
                    }
                }
                Op::ToNum => {
                    let operand = read_u8(code, &mut ip)?;
                    let v = self.pop()?;
                    if operand == rapidr_value::numeric::ARG_ROUND {
                        self.stack.push(rapidr_value::numeric::arg_round(&v));
                    } else {
                        let kind = rapidr_value::numeric::NumKind::from_code(operand).ok_or(VmError::BadOperand)?;
                        self.stack.push(rapidr_value::numeric::convert(&v, kind));
                    }
                }
                Op::CallIndirect => {
                    let argc = read_u8(code, &mut ip)?;
                    let at = self.stack.len().checked_sub(argc as usize + 1).ok_or(VmError::StackUnderflow)?;
                    let ptr = self.stack.remove(at).to_i64();
                    let fi = u32::try_from(ptr - 1)
                        .ok()
                        .filter(|&fi| (fi as usize) < module.functions.len())
                        .ok_or_else(|| VmError::Runtime(format!("CALLFUNC: {ptr} is not a function pointer (use BIND or CODEPTR)")))?;
                    self.frames.last_mut().unwrap().ip = ip;
                    self.frames.last_mut().unwrap().ret_ip = ip;
                    self.call(module, fi, argc, true)?;
                    ip = 0;
                    refresh!();
                    tick!();
                }
                Op::CallMethodDyn => {
                    let m_i = read_u32(code, &mut ip)?;
                    let argc = read_u8(code, &mut ip)? as usize;
                    let m = module.strings.get(m_i as usize).ok_or(VmError::BadStringIndex(m_i))?.clone();
                    let mut args = Vec::with_capacity(argc);
                    for _ in 0..argc { args.push(self.pop()?); }
                    args.reverse();
                    let id = self.pop_object_id()?;
                    let r = self.host.call_method(&id, &m, &args).map_err(VmError::HostError)?;
                    // A method that waits (the web's ShowModal): suspended
                    // until the host resumes with its result.
                    if self.host.suspend_requested() {
                        let top = self.frames.last_mut().unwrap();
                        top.ip = ip;
                        top.waiting = true;
                        return Err(VmError::Suspended);
                    }
                    self.stack.push(r);
                    after_host!(true);
                }
                Op::RegisterEvent => {
                    let id_i = read_u32(code, &mut ip)?;
                    let ev_i = read_u32(code, &mut ip)?;
                    let fi = read_u32(code, &mut ip)?;
                    let id = module.strings.get(id_i as usize).ok_or(VmError::BadStringIndex(id_i))?.clone();
                    let ev = module.strings.get(ev_i as usize).ok_or(VmError::BadStringIndex(ev_i))?.clone();
                    self.host.register_event(&id, &ev, fi).map_err(VmError::HostError)?;
                }

                // ----- arrays -----
                Op::NewArray => {
                    let n = read_u8(code, &mut ip)? as usize;
                    let mut bounds = vec![(0i64, 0i64); n];
                    for b in bounds.iter_mut().rev() {
                        let upper = self.pop()?.to_i64();
                        let lower = self.pop()?.to_i64();
                        *b = (lower, upper);
                    }
                    let fill = self.pop()?;
                    let arr = v_array(bounds, fill).map_err(VmError::Runtime)?;
                    self.stack.push(arr);
                }
                Op::AGet => {
                    let n = read_u8(code, &mut ip)? as usize;
                    let (buf, n) = self.pop_indices(n)?;
                    let indices = &buf[..n];
                    let target = self.pop()?;
                    let v = match &target {
                        Value::Array(a) => a.borrow().get(indices).map_err(VmError::Runtime)?,
                        // Legacy: indexing a comma-separated string.
                        other if n == 1 => other.rp_index(&v_int(indices[0])),
                        _ => return Err(VmError::Runtime("indexing a value that is not an array".into())),
                    };
                    self.stack.push(v);
                }
                Op::ASet => {
                    let n = read_u8(code, &mut ip)? as usize;
                    let val = self.pop()?;
                    let (buf, n) = self.pop_indices(n)?;
                    match self.pop()? {
                        Value::Array(a) => a.borrow_mut().set(&buf[..n], val).map_err(VmError::Runtime)?,
                        _ => return Err(VmError::Runtime(
                            "assigning to an element of a variable that is not an array (DIM it with a size first)".into(),
                        )),
                    }
                }
                Op::Redim => {
                    let s = read_u16(code, &mut ip)?;
                    let n = read_i32(code, &mut ip)?.max(0) as usize;
                    let frame = self.frames.last_mut().unwrap();
                    let slot = frame.locals.get_mut(s as usize).ok_or(VmError::BadLocalSlot(s))?;
                    let parts: Vec<String> = (0..n).map(|_| String::new()).collect();
                    *slot = v_str(&parts.join(","));
                }

                // ----- I/O -----
                Op::Print => {
                    let s = rapidr_value::format::print_text(&self.pop()?);
                    self.emit_output(&s)?;
                }
                Op::PrintLn => {
                    let mut s = rapidr_value::format::print_text(&self.pop()?);
                    s.push('\n');
                    self.emit_output(&s)?;
                }
                Op::PrintZone => {
                    let pad = PRINT_ZONE_WIDTH - self.print_col % PRINT_ZONE_WIDTH;
                    self.emit_output(&" ".repeat(pad))?;
                }
                Op::Input => {
                    let s = self.host.input().map_err(VmError::HostError)?;
                    if self.host.suspend_requested() {
                        let top = self.frames.last_mut().unwrap();
                        top.ip = ip;
                        top.waiting = true;
                        return Err(VmError::Suspended);
                    }
                    self.stack.push(v_str(&s));
                    after_host!(true);
                }
            }
        }
    }

    /// Runs the event handlers a host operation fired, then serves a wait
    /// it started (see [`Host::wait_started`]), replacing the operation's
    /// result (on the stack when `has_result`) with the wait's. Returns
    /// false if a handler ENDed the program.
    #[inline]
    fn after_host(&mut self, module: &Module, ip: usize, has_result: bool) -> Result<bool, VmError> {
        let mut waiting = self.host.wait_started();
        loop {
            // Until none is left (a continuation queued behind a handler
            // runs once the handler has: rapidr_value::events).
            loop {
                let events = self.host.take_events();
                if events.is_empty() {
                    break;
                }
                if !self.run_events(module, ip, events)? {
                    return Ok(false);
                }
            }
            if !waiting {
                return Ok(true);
            }
            let turn = match self.pump_wait(module, ip)? {
                WaitTurn::Pumped(turn) => turn,
                WaitTurn::Ended => return Ok(false),
            };
            if let Some(result) = turn {
                if has_result {
                    self.pop()?;
                    self.stack.push(result);
                }
                // Once more for the events the last pump queued.
                waiting = false;
            }
        }
    }

    /// One turn of the host's wait ([`Host::pump`]): `None` while it goes
    /// on, `Some` with its result when it's over. A host that can serve the
    /// program from inside the window system's own loop
    /// ([`Host::pump_serving`]) gets the VM lent for it: the events queued
    /// meanwhile run there as they would after the turn, on top of the
    /// current frame (`ip` saved, as [`Self::run_events`] does). A handler
    /// that ENDs the program or fails there stops the VM once the turn is
    /// over.
    fn pump_wait(&mut self, module: &Module, ip: usize) -> Result<WaitTurn, VmError> {
        let Some(pump) = self.host.pump_serving() else { return Ok(WaitTurn::Pumped(self.host.pump())) };
        let mut stop: Option<Result<(), VmError>> = None;
        let result = pump(&mut || {
            if stop.is_some() {
                return;
            }
            loop {
                let events = self.host.take_events();
                if events.is_empty() {
                    return;
                }
                match self.run_events(module, ip, events) {
                    Ok(true) => {}
                    Ok(false) => return stop = Some(Ok(())),
                    Err(e) => return stop = Some(Err(e)),
                }
            }
        });
        match stop {
            Some(Ok(())) => Ok(WaitTurn::Ended),
            Some(Err(e)) => Err(e),
            None => Ok(WaitTurn::Pumped(result)),
        }
    }

    /// Runs `events` one after the other, each to completion, on top of the
    /// current frame (whose `ip` is saved first, so a handler that suspends
    /// leaves this code ready to continue; the events after it go back to
    /// the host — or, when it yields, stay here and run once it returns:
    /// [`Self::continue_run`]). Returns false if a handler ENDed the program.
    fn run_events(&mut self, module: &Module, ip: usize, events: Vec<QueuedEvent>) -> Result<bool, VmError> {
        let mut events = events.into_iter();
        while let Some(event) = events.next() {
            if let Some(top) = self.frames.last_mut() {
                top.ip = ip;
            }
            let depth = self.frames.len();
            if let Err(e) = self.invoke(module, event, true) {
                let rest: Vec<_> = events.collect();
                if matches!(e, VmError::Yielded) {
                    if !rest.is_empty() {
                        self.yield_rest.push((depth, rest));
                    }
                } else if !rest.is_empty() {
                    self.host.defer_events(rest);
                }
                return Err(e);
            }
            if self.frames.is_empty() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Pop the current frame and return a value (or Null) to the caller.
    /// [`Returned::Stop`] when the popped frame was the entry or an event
    /// handler's — the VM must stop.
    fn return_frame(&mut self, module: &Module, with_value: bool) -> Result<Returned, VmError> {
        let ret = if with_value { self.pop()? } else { v_null() };
        let frame = self.frames.pop().ok_or(VmError::StackUnderflow)?;
        let n_params = module
            .functions
            .get(frame.fn_index as usize)
            .map_or(0, |f| f.params.len())
            .min(frame.locals.len());
        self.arg_out.clear();
        self.arg_out.extend_from_slice(&frame.locals[..n_params]);
        if self.spare_locals.len() < 64 {
            let mut spare = frame.locals;
            spare.clear();
            self.spare_locals.push(spare);
        }
        if frame.stop {
            self.returned_nested = frame.nested;
            if frame.wants_value {
                self.stack.push(ret);
            }
            if !frame.then.is_empty() {
                // (with the handler's parameters: RapidQ's event arguments
                // come back, e.g. OnClose's Action)
                let params = std::mem::take(&mut self.arg_out);
                self.host.event_finished(frame.then, &params);
                self.arg_out = params;
            }
            return Ok(Returned::Stop);
        }
        if self.frames.is_empty() {
            return Ok(Returned::Stop);
        }
        if frame.wants_value {
            self.stack.push(ret);
        }
        Ok(Returned::Continue)
    }

    fn emit_output(&mut self, s: &str) -> Result<(), VmError> {
        // Escape sequences (COLOR, LOCATE) don't take up columns.
        self.print_col = rapidr_value::console::advance(1, self.print_col + 1, s).1 - 1;
        self.host.print(s).map_err(VmError::HostError)
    }

    /// Pops an object reference (a component or TYPE instance id).
    fn pop_object_id(&mut self) -> Result<String, VmError> {
        match self.pop()? {
            Value::String(id) if !id.is_empty() => Ok(id),
            _ => Err(VmError::Runtime(
                "this variable does not refer to an object (component or TYPE instance)".into(),
            )),
        }
    }

    /// Pops `n` array indices pushed in order (first index deepest).
    /// Pops `n` array indices (first index deepest) into a stack buffer:
    /// no allocation per array access.
    #[inline]
    fn pop_indices(&mut self, n: usize) -> Result<([i64; MAX_DIMS], usize), VmError> {
        if n > MAX_DIMS {
            return Err(VmError::Runtime(format!("arrays have at most {MAX_DIMS} dimensions")));
        }
        let mut indices = [0i64; MAX_DIMS];
        for i in indices[..n].iter_mut().rev() {
            *i = self.pop()?.to_i64();
        }
        Ok((indices, n))
    }

    fn pop(&mut self) -> Result<Value, VmError> {
        self.stack.pop().ok_or(VmError::StackUnderflow)
    }

    fn peek(&self) -> Result<&Value, VmError> {
        self.stack.last().ok_or(VmError::StackUnderflow)
    }

    /// Runs an event handler: `fn_index` with `args`, on top of whatever
    /// it interrupts (nothing, a program waiting for a dialog, or — from
    /// [`Vm::after_host`] — the code whose host operation queued the event),
    /// and returns its value. If the handler suspends or pauses, its frames
    /// stay on top of the interrupted ones and [`Vm::resume_with`] /
    /// [`Vm::resume`] finish it and then continue what it interrupted.
    pub fn invoke_function(&mut self, module: &Module, fn_index: u32, args: Vec<Value>) -> Result<Value, VmError> {
        self.invoke_event(module, QueuedEvent::new(fn_index, args))
    }

    /// Runs a queued event's handler; its continuations go to the host when
    /// it returns (or fails).
    pub fn invoke_event(&mut self, module: &Module, event: QueuedEvent) -> Result<Value, VmError> {
        self.invoke(module, event, false)
    }

    /// [`Self::invoke_event`]; `nested`: run right after a host operation
    /// of the code below (see [`Frame::nested`]).
    fn invoke(&mut self, module: &Module, event: QueuedEvent, nested: bool) -> Result<Value, VmError> {
        let QueuedEvent { handler: fn_index, args, then } = event;
        let (base_frames, base_stack) = (self.frames.len(), self.stack.len());
        let argc = u8::try_from(args.len()).map_err(|_| VmError::Runtime("too many event arguments".into()))?;
        self.stack.extend(args);
        if let Err(e) = self.call(module, fn_index, argc, true) {
            self.stack.truncate(base_stack);
            self.host.event_finished(then, &[]);
            return Err(e);
        }
        let top = self.frames.last_mut().unwrap();
        top.stop = true;
        top.then = then;
        top.nested = nested;
        match self.exec(module) {
            // The handler returned (its value is on top), or ENDed.
            Ok(()) if self.frames.len() == base_frames => Ok(self.stack.pop().unwrap_or(Value::Null)),
            Ok(()) => Ok(Value::Null),
            Err(e @ (VmError::Suspended | VmError::Paused | VmError::Yielded)) => Err(e),
            Err(e) => {
                // A failed handler leaves nothing behind (its continuations run).
                if let Some(f) = self.frames.get_mut(base_frames) {
                    let then = std::mem::take(&mut f.then);
                    self.host.event_finished(then, &[]);
                }
                self.frames.truncate(base_frames);
                self.stack.truncate(base_stack);
                Err(e)
            }
        }
    }

    /// Continues after a pause, suspension or yield until the program
    /// ends, waits again, or pauses/suspends/yields. A finished event
    /// handler's value is dropped (whoever ran it is gone); if it ran right
    /// after a host operation, the events queued behind it run and then
    /// the code it interrupted continues. (A handler the host ran on top of
    /// waiting or paused code just ends.)
    fn continue_run(&mut self, module: &Module) -> Result<(), VmError> {
        // (stopped at an error: it unwinds now, as it would have)
        if let Some(error) = self.pending_error.take() {
            return Err(self.unwind_error(error));
        }
        loop {
            self.exec(module)?;
            let Some(top) = self.frames.last() else {
                // (the value of a handler that ran with nothing below)
                self.stack.clear();
                return Ok(());
            };
            let (waiting, ip) = (top.waiting, top.ip);
            // exec stopped at an event handler's frame: drop its value.
            self.stack.pop();
            if waiting || !self.returned_nested {
                return Ok(());
            }
            // As `after_host` would have gone on: the rest of the events it
            // took, then those queued since.
            if H::YIELDS {
                let depth = self.frames.len();
                if let Some(i) = self.yield_rest.iter().position(|(d, _)| *d == depth) {
                    let (_, rest) = self.yield_rest.remove(i);
                    if !self.run_events(module, ip, rest)? {
                        return Ok(());
                    }
                }
                if !self.after_host(module, ip, false)? {
                    return Ok(());
                }
            }
        }
    }

    pub fn is_paused(&self) -> bool {
        !self.frames.is_empty()
    }

    /// The error the VM stopped at (without a [`Debugger`]) does what it
    /// would have done had the VM not stopped: the innermost event handler
    /// the host ran fails and leaves nothing behind (as [`Self::invoke`]),
    /// and a handler that ran right after a host operation fails the code
    /// below it too; reaching the main program, it ends.
    fn unwind_error(&mut self, error: VmError) -> VmError {
        while let Some(i) = self.frames.iter().rposition(|f| f.stop) {
            let frame = &mut self.frames[i];
            let then = std::mem::take(&mut frame.then);
            let (nested, base) = (frame.nested, frame.stack_base);
            self.host.event_finished(then, &[]);
            self.frames.truncate(i);
            self.stack.truncate(base);
            let depth = self.frames.len();
            self.yield_rest.retain(|(d, _)| *d < depth);
            if !nested {
                return error;
            }
        }
        self.frames.clear();
        self.stack.clear();
        self.yield_rest.clear();
        error
    }

    /// Breakpoints at compiled lines (of the preprocessed program).
    pub fn set_breakpoints(&mut self, bps: std::collections::HashSet<u32>) {
        self.line_breakpoints = bps;
        self.rebuild_breakpoints();
    }

    pub fn add_breakpoint(&mut self, line: u32) {
        self.line_breakpoints.insert(line);
        self.rebuild_breakpoints();
    }

    pub fn remove_breakpoint(&mut self, line: u32) {
        self.line_breakpoints.remove(&line);
        self.rebuild_breakpoints();
    }

    /// Replaces the breakpoints of `file` (a name: the program's own file
    /// or an `$INCLUDE`d one, as `Module::source_map` names them; with no
    /// source map, the program) with `lines` of that file, through the
    /// source map. A line without code moves to the next line of the same
    /// file that has some; the answer says, per line asked, where it stops
    /// (`None`: nowhere — no code from there on, or no such file).
    pub fn set_file_breakpoints(&mut self, module: &Module, file: &str, lines: &[u32]) -> Vec<Option<u32>> {
        let code_lines = module.code_lines();
        let map = &module.source_map;
        let mut compiled = std::collections::HashSet::new();
        let placed = lines
            .iter()
            .map(|&line| {
                if map.runs.is_empty() {
                    // (no map: compiled lines are the program's own)
                    let at = code_lines.get(code_lines.partition_point(|&l| l < line)).copied()?;
                    compiled.insert(at);
                    return Some(at);
                }
                let file_index = map.file_index(file)?;
                let mut actual = None;
                for start in map.compiled_lines(file, line) {
                    // the first line with code from there on, in the same file
                    let next = code_lines[code_lines.partition_point(|&l| l < start)..]
                        .iter()
                        .copied()
                        .find(|&l| map.locate(l).is_some_and(|(f, _)| map.file_index(f) == Some(file_index)));
                    if let Some(at) = next {
                        compiled.insert(at);
                        actual = actual.or_else(|| map.locate(at).map(|(_, l)| l));
                    }
                }
                actual
            })
            .collect();
        let key = file.rsplit(['/', '\\']).next().unwrap_or(file).to_ascii_lowercase();
        self.file_breakpoints.retain(|(f, _)| *f != key);
        if !compiled.is_empty() {
            self.file_breakpoints.push((key, compiled));
        }
        self.rebuild_breakpoints();
        placed
    }

    /// Every file's breakpoints gone.
    pub fn clear_breakpoints(&mut self) {
        self.file_breakpoints.clear();
        self.line_breakpoints.clear();
        self.breakpoints.clear();
    }

    fn rebuild_breakpoints(&mut self) {
        self.breakpoints = self.line_breakpoints.clone();
        for (_, lines) in &self.file_breakpoints {
            self.breakpoints.extend(lines);
        }
    }

    /// Asks the VM to pause at its next instruction (debug mode). Safe from
    /// any thread through a clone of [`Self::interrupt`].
    pub fn request_pause(&self) {
        self.interrupt.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// The source location (file, line) of frame `index` (0 = the
    /// outermost), through the source map; the file is `None` without one.
    pub fn frame_location(&self, module: &Module, index: usize) -> Option<(Option<String>, u32)> {
        let frame = self.frames.get(index)?;
        // (a caller's ip is past the call — or the host operation that ran
        // a handler —: its line is the one before)
        let ip = if index + 1 < self.frames.len() { frame.ip.saturating_sub(1) } else { frame.ip };
        let line = module.functions.get(frame.fn_index as usize)?.get_line_for_ip(ip)?;
        Some(match module.source_map.locate(line) {
            Some((file, l)) => (Some(file.to_string()), l),
            None => (None, line),
        })
    }

    /// Runs `fn_index` of `module` — a snippet compiled against frame
    /// `frame`'s symbols (`rapidr_bcgen::compile_snippet`, whose module is
    /// the program's plus the snippet) — on top of the stopped program, and
    /// returns its value. The snippet sees the frame's locals (its first
    /// parameters are the frame's local slots) and, with `write_back`, what
    /// it assigns to them is written into the frame (the Immediate window,
    /// setting a variable). It can't stop at breakpoints, END the program
    /// or wait, and has `fuel` instructions; a failure leaves the program as
    /// it was.
    pub fn evaluate(&mut self, module: &Module, fn_index: u32, frame: Option<usize>, write_back: bool, fuel: u64) -> Result<Value, VmError> {
        let f = module.functions.get(fn_index as usize).ok_or(VmError::BadFunctionIndex(fn_index))?;
        let (base_frames, base_stack) = (self.frames.len(), self.stack.len());
        let mut locals = self.spare_locals.pop().unwrap_or_default();
        locals.clear();
        if let Some(i) = frame {
            let source = self.frames.get(i).ok_or_else(|| VmError::Runtime(format!("no frame {i}")))?;
            locals.extend(source.locals.iter().cloned());
        }
        locals.resize((f.n_locals as usize).max(locals.len()), Value::Null);
        self.frames.push(Frame {
            fn_index,
            locals,
            ret_ip: 0,
            wants_value: true,
            ip: 0,
            gosub: Vec::new(),
            stop: true,
            waiting: false,
            then: Vec::new(),
            nested: false,
            stack_base: base_stack,
        });
        let saved = (self.debug_mode, self.step_mode, self.last_line, self.eval_fuel.take(), self.returned_nested);
        self.debug_mode = true;
        self.eval_fuel = Some(fuel);
        let result = self.exec(module);
        (self.debug_mode, self.step_mode, self.last_line, self.eval_fuel, self.returned_nested) = saved;
        match result {
            Ok(()) if self.frames.len() == base_frames => {
                let value = self.stack.pop().unwrap_or(Value::Null);
                self.stack.truncate(base_stack);
                if let (true, Some(i)) = (write_back, frame) {
                    // (the snippet's parameters are the frame's slots)
                    let out = std::mem::take(&mut self.arg_out);
                    if let Some(target) = self.frames.get_mut(i) {
                        for (slot, v) in target.locals.iter_mut().zip(out.iter()) {
                            *slot = v.clone();
                        }
                    }
                    self.arg_out = out;
                }
                Ok(value)
            }
            other => {
                self.frames.truncate(base_frames);
                self.stack.truncate(base_stack);
                Err(match other {
                    Ok(()) => VmError::Runtime("the evaluation ended the program".into()),
                    Err(VmError::Suspended | VmError::Yielded | VmError::Paused) => {
                        VmError::Runtime("the evaluation would wait (a dialog or a form): not while the program is stopped".into())
                    }
                    Err(VmError::At { error, .. }) => *error,
                    Err(e) => e,
                })
            }
        }
    }

    /// Sets local slot `slot` of frame `frame` (0 = the outermost).
    pub fn set_local(&mut self, frame: usize, slot: usize, value: Value) -> Result<(), VmError> {
        let f = self.frames.get_mut(frame).ok_or_else(|| VmError::Runtime(format!("no frame {frame}")))?;
        let s = f.locals.get_mut(slot).ok_or(VmError::BadLocalSlot(slot as u16))?;
        *s = value;
        Ok(())
    }

    /// Sets the global variable `name` (any case; the spelling the program
    /// uses). Returns false if the program has no such global yet.
    pub fn set_global(&mut self, module: &Module, name: &str, value: Value) -> bool {
        let Some(i) = self.global_index(module, name) else { return false };
        if i >= self.globals.len() {
            self.globals.resize(module.strings.len().max(i + 1), None);
        }
        self.globals[i] = Some(value);
        true
    }

    /// The slot of global `name`: the string the program assigned it under
    /// (the VM keys globals by the index of their name's spelling).
    pub fn global_index(&self, module: &Module, name: &str) -> Option<usize> {
        let mut first = None;
        for (i, s) in module.strings.iter().enumerate() {
            if s.eq_ignore_ascii_case(name) {
                if self.globals.get(i).is_some_and(Option::is_some) {
                    return Some(i);
                }
                first.get_or_insert(i);
            }
        }
        first
    }

    pub fn resume(&mut self, module: &Module) -> Result<(), VmError> {
        self.continue_run(module)
    }

    /// Continues after [`VmError::Suspended`], with `value` as the result of
    /// the builtin (or INPUT) that suspended. Dialogs are answered in the
    /// order they're stacked: the innermost waiting code gets `value`.
    pub fn resume_with(&mut self, module: &Module, value: Value) -> Result<(), VmError> {
        let Some(top) = self.frames.last_mut() else { return Ok(()) };
        top.waiting = false;
        self.stack.push(value);
        self.continue_run(module)
    }

    /// Whether the program (or an event handler) waits for a dialog.
    pub fn is_waiting(&self) -> bool {
        self.frames.iter().any(|f| f.waiting)
    }

    pub fn step_into(&mut self, module: &Module) -> Result<(), VmError> {
        self.step_mode = StepMode::Into;
        self.last_line = self.current_line(module).unwrap_or(0);
        self.continue_run(module)
    }

    pub fn step_over(&mut self, module: &Module) -> Result<(), VmError> {
        self.step_mode = StepMode::Over { target_depth: self.frames.len() };
        self.last_line = self.current_line(module).unwrap_or(0);
        self.continue_run(module)
    }

    pub fn step_out(&mut self, module: &Module) -> Result<(), VmError> {
        self.step_mode = StepMode::Out { target_depth: self.frames.len() };
        self.last_line = self.current_line(module).unwrap_or(0);
        self.continue_run(module)
    }

    pub fn current_line(&self, module: &Module) -> Option<u32> {
        let frame = self.frames.last()?;
        let func = module.functions.get(frame.fn_index as usize)?;
        func.get_line_for_ip(frame.ip)
    }
}

// ---------- operand decoders ----------

#[inline(always)]
fn read_u8(code: &[u8], ip: &mut usize) -> Result<u8, VmError> {
    let b = *code.get(*ip).ok_or(VmError::Truncated)?;
    *ip += 1;
    Ok(b)
}
#[inline(always)]
fn read_u16(code: &[u8], ip: &mut usize) -> Result<u16, VmError> {
    if *ip + 2 > code.len() { return Err(VmError::Truncated); }
    let v = u16::from_le_bytes([code[*ip], code[*ip + 1]]);
    *ip += 2; Ok(v)
}
#[inline(always)]
fn read_u32(code: &[u8], ip: &mut usize) -> Result<u32, VmError> {
    if *ip + 4 > code.len() { return Err(VmError::Truncated); }
    let v = u32::from_le_bytes([code[*ip], code[*ip + 1], code[*ip + 2], code[*ip + 3]]);
    *ip += 4; Ok(v)
}
#[inline(always)]
fn read_i32(code: &[u8], ip: &mut usize) -> Result<i32, VmError> {
    Ok(read_u32(code, ip)? as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rapidr_bytecode::{Const, Function, Module};

    fn emit_print_hello() -> Module {
        let mut m = Module::new();
        let c = m.add_const(Const::Str("hello".into()));
        let mut f = Function::default();
        f.name = "__main".into();
        f.code.push(Op::LoadConst as u8);
        f.code.extend_from_slice(&c.to_le_bytes());
        f.code.push(Op::PrintLn as u8);
        f.code.push(Op::Halt as u8);
        m.entry = m.add_function(f);
        m
    }

    #[test]
    fn print_hello() {
        let m = emit_print_hello();
        let mut h = StubHost::default();
        let mut vm = Vm::new(&mut h);
        vm.run(&m).unwrap();
        assert_eq!(h.output, "hello\n");
    }

    #[test]
    fn arithmetic() {
        // 3 + 4 * 2 → push 3, push 4, push 2, mul, add, print
        let mut m = Module::new();
        let c3 = m.add_const(Const::Int(3));
        let c4 = m.add_const(Const::Int(4));
        let c2 = m.add_const(Const::Int(2));
        let mut f = Function::default();
        f.code.push(Op::LoadConst as u8); f.code.extend_from_slice(&c3.to_le_bytes());
        f.code.push(Op::LoadConst as u8); f.code.extend_from_slice(&c4.to_le_bytes());
        f.code.push(Op::LoadConst as u8); f.code.extend_from_slice(&c2.to_le_bytes());
        f.code.push(Op::Mul as u8);
        f.code.push(Op::Add as u8);
        f.code.push(Op::PrintLn as u8);
        f.code.push(Op::Halt as u8);
        m.entry = m.add_function(f);
        let mut h = StubHost::default();
        let mut vm = Vm::new(&mut h);
        vm.run(&m).unwrap();
        assert_eq!(h.output, "11\n");
    }

    #[test]
    fn jump_if_not() {
        // if false then print "yes" else print "no"
        let mut m = Module::new();
        let cy = m.add_const(Const::Str("yes".into()));
        let cn = m.add_const(Const::Str("no".into()));
        let mut f = Function::default();
        f.code.push(Op::LoadFalse as u8);
        // JumpIfNot to else branch
        f.code.push(Op::JumpIfNot as u8);
        let jt_pos = f.code.len();
        f.code.extend_from_slice(&[0,0,0,0]); // placeholder
        // then:
        f.code.push(Op::LoadConst as u8); f.code.extend_from_slice(&cy.to_le_bytes());
        f.code.push(Op::PrintLn as u8);
        // jump end
        f.code.push(Op::Jump as u8);
        let je_pos = f.code.len();
        f.code.extend_from_slice(&[0,0,0,0]);
        // else:
        let else_off = f.code.len() as u32;
        f.code.push(Op::LoadConst as u8); f.code.extend_from_slice(&cn.to_le_bytes());
        f.code.push(Op::PrintLn as u8);
        // end:
        let end_off = f.code.len() as u32;
        f.code.push(Op::Halt as u8);
        // patch
        f.code[jt_pos..jt_pos+4].copy_from_slice(&else_off.to_le_bytes());
        f.code[je_pos..je_pos+4].copy_from_slice(&end_off.to_le_bytes());
        m.entry = m.add_function(f);
        let mut h = StubHost::default();
        let mut vm = Vm::new(&mut h);
        vm.run(&m).unwrap();
        assert_eq!(h.output, "no\n");
    }

    #[test]
    fn call_func_with_args() {
        // FUNCTION add(a, b) = a + b
        let mut m = Module::new();
        let c1 = m.add_const(Const::Int(10));
        let c2 = m.add_const(Const::Int(32));
        // add: LoadLocal 0; LoadLocal 1; Add; RetVal
        let mut add = Function::default();
        add.name = "add".into();
        add.params.push(rapidr_bytecode::Param { name: "a".into(), by_ref: false });
        add.params.push(rapidr_bytecode::Param { name: "b".into(), by_ref: false });
        add.n_locals = 2;
        add.code.push(Op::LoadLocal as u8); add.code.extend_from_slice(&0u16.to_le_bytes());
        add.code.push(Op::LoadLocal as u8); add.code.extend_from_slice(&1u16.to_le_bytes());
        add.code.push(Op::Add as u8);
        add.code.push(Op::RetVal as u8);
        let add_idx = m.add_function(add);
        // main: push 10, push 32, callfunc add 2, println, halt
        let mut main = Function::default();
        main.name = "__main".into();
        main.code.push(Op::LoadConst as u8); main.code.extend_from_slice(&c1.to_le_bytes());
        main.code.push(Op::LoadConst as u8); main.code.extend_from_slice(&c2.to_le_bytes());
        main.code.push(Op::CallFunc as u8); main.code.extend_from_slice(&add_idx.to_le_bytes()); main.code.push(2);
        main.code.push(Op::PrintLn as u8);
        main.code.push(Op::Halt as u8);
        m.entry = m.add_function(main);
        let mut h = StubHost::default();
        let mut vm = Vm::new(&mut h);
        vm.run(&m).unwrap();
        assert_eq!(h.output, "42\n");
    }

    /// A host whose ASK builtin suspends the VM until the answer arrives.
    #[derive(Default)]
    struct AskHost {
        inner: StubHost,
        waiting: bool,
    }

    impl Host for AskHost {
        fn call_builtin(&mut self, name: &str, args: &[Value]) -> Result<Value, String> {
            self.waiting = name == "ASK";
            self.inner.call_builtin(name, args)
        }
        fn suspend_requested(&mut self) -> bool {
            std::mem::take(&mut self.waiting)
        }
        fn create_comp(&mut self, k: &str, id: &str) -> Result<Value, String> { self.inner.create_comp(k, id) }
        fn set_prop(&mut self, id: &str, n: &str, v: Value) -> Result<(), String> { self.inner.set_prop(id, n, v) }
        fn get_prop(&mut self, id: &str, n: &str) -> Result<Value, String> { self.inner.get_prop(id, n) }
        fn call_method(&mut self, id: &str, m: &str, a: &[Value]) -> Result<Value, String> { self.inner.call_method(id, m, a) }
        fn register_event(&mut self, id: &str, e: &str, f: u32) -> Result<(), String> { self.inner.register_event(id, e, f) }
        fn print(&mut self, s: &str) -> Result<(), String> { self.inner.print(s) }
        fn input(&mut self) -> Result<String, String> { self.inner.input() }
    }

    #[test]
    fn suspend_and_resume_inside_a_function() {
        // FUNCTION f: RETURN ASK() + 1 ; main: PRINT f() * 2
        let mut m = Module::new();
        let ask = m.add_string("ASK");
        let one = m.add_const(Const::Int(1));
        let two = m.add_const(Const::Int(2));
        let mut f = Function::default();
        f.name = "f".into();
        f.code.push(Op::CallBuiltin as u8); f.code.extend_from_slice(&ask.to_le_bytes()); f.code.push(0);
        f.code.push(Op::LoadConst as u8); f.code.extend_from_slice(&one.to_le_bytes());
        f.code.push(Op::Add as u8);
        f.code.push(Op::RetVal as u8);
        let fi = m.add_function(f);
        let mut main = Function::default();
        main.name = "__main".into();
        main.code.push(Op::CallFunc as u8); main.code.extend_from_slice(&fi.to_le_bytes()); main.code.push(0);
        main.code.push(Op::LoadConst as u8); main.code.extend_from_slice(&two.to_le_bytes());
        main.code.push(Op::Mul as u8);
        main.code.push(Op::PrintLn as u8);
        main.code.push(Op::Halt as u8);
        m.entry = m.add_function(main);
        let mut h = AskHost::default();
        let mut vm = Vm::new(&mut h);
        assert!(matches!(vm.run(&m), Err(VmError::Suspended)));
        vm.resume_with(&m, v_int(20)).unwrap();
        assert_eq!(h.inner.output, "42\n");
    }

    /// Queues events and waits like the desktop/web hosts: `FIRE(fn)` queues
    /// handler `fn`; `ASK` suspends; `MODAL` starts a wait that ends after
    /// two pumps (each queuing handler 1) with the result 7.
    #[derive(Default)]
    struct QueueHost {
        inner: StubHost,
        queue: std::collections::VecDeque<QueuedEvent>,
        asking: bool,
        wait: Option<u32>,
        started: bool,
    }

    impl Host for QueueHost {
        fn call_builtin(&mut self, name: &str, args: &[Value]) -> Result<Value, String> {
            match name {
                "FIRE" => self.queue.push_back(QueuedEvent::new(args[0].to_i64() as u32, vec![v_str("sender")])),
                "ASK" => self.asking = true,
                "MODAL" => {
                    self.wait = Some(2);
                    self.started = true;
                }
                _ => return self.inner.call_builtin(name, args),
            }
            Ok(v_null())
        }
        fn suspend_requested(&mut self) -> bool { std::mem::take(&mut self.asking) }
        fn take_events(&mut self) -> Vec<QueuedEvent> { self.queue.drain(..).collect() }
        fn wait_started(&mut self) -> bool { std::mem::take(&mut self.started) }
        fn pump(&mut self) -> Option<Value> {
            match self.wait {
                Some(0) | None => {
                    self.wait = None;
                    Some(v_int(7))
                }
                Some(n) => {
                    self.wait = Some(n - 1);
                    self.queue.push_back(QueuedEvent::new(1, vec![]));
                    None
                }
            }
        }
        fn create_comp(&mut self, k: &str, id: &str) -> Result<Value, String> { self.inner.create_comp(k, id) }
        fn set_prop(&mut self, id: &str, n: &str, v: Value) -> Result<(), String> { self.inner.set_prop(id, n, v) }
        fn get_prop(&mut self, id: &str, n: &str) -> Result<Value, String> { self.inner.get_prop(id, n) }
        fn call_method(&mut self, id: &str, m: &str, a: &[Value]) -> Result<Value, String> { self.inner.call_method(id, m, a) }
        fn register_event(&mut self, id: &str, e: &str, f: u32) -> Result<(), String> { self.inner.register_event(id, e, f) }
        fn print(&mut self, s: &str) -> Result<(), String> { self.inner.print(s) }
        fn input(&mut self) -> Result<String, String> { self.inner.input() }
    }

    fn emit_print(f: &mut Function, m: &mut Module, text: &str) {
        let c = m.add_const(Const::Str(text.into()));
        f.code.push(Op::LoadConst as u8); f.code.extend_from_slice(&c.to_le_bytes());
        f.code.push(Op::PrintLn as u8);
    }

    fn emit_builtin(f: &mut Function, m: &mut Module, name: &str, arg: Option<i64>) {
        let n = m.add_string(name);
        if let Some(a) = arg {
            let c = m.add_const(Const::Int(a));
            f.code.push(Op::LoadConst as u8); f.code.extend_from_slice(&c.to_le_bytes());
        }
        f.code.push(Op::CallBuiltin as u8); f.code.extend_from_slice(&n.to_le_bytes()); f.code.push(arg.is_some() as u8);
    }

    /// Module: 0 = handler A (prints "a1", optionally ASKs, prints "a2"),
    /// 1 = handler B (prints "b"), entry = main (see each test).
    fn event_module(handler_asks: bool, handler_ends: bool) -> (Module, Function) {
        let mut m = Module::new();
        let mut a = Function::default();
        a.name = "A".into();
        a.params = vec![rapidr_bytecode::Param { name: "Sender".into(), by_ref: false }];
        a.n_locals = 1;
        emit_print(&mut a, &mut m, "a1");
        if handler_asks {
            emit_builtin(&mut a, &mut m, "ASK", None);
            a.code.push(Op::PrintLn as u8);
        }
        if handler_ends {
            a.code.push(Op::Halt as u8);
        }
        emit_print(&mut a, &mut m, "a2");
        a.code.push(Op::Ret as u8);
        m.add_function(a);
        let mut b = Function::default();
        b.name = "B".into();
        emit_print(&mut b, &mut m, "b");
        b.code.push(Op::Ret as u8);
        m.add_function(b);
        let mut main = Function::default();
        main.name = "__main".into();
        (m, main)
    }

    #[test]
    fn an_event_a_builtin_fires_runs_right_after_it() {
        let (mut m, mut main) = event_module(false, false);
        emit_print(&mut main, &mut m, "m1");
        emit_builtin(&mut main, &mut m, "FIRE", Some(0));
        main.code.push(Op::Pop as u8);
        emit_print(&mut main, &mut m, "m2");
        main.code.push(Op::Halt as u8);
        m.entry = m.add_function(main);
        let mut h = QueueHost::default();
        let mut vm = Vm::new(&mut h);
        vm.run(&m).unwrap();
        assert!(vm.stack.is_empty());
        assert_eq!(h.inner.output, "m1\na1\na2\nm2\n");
    }

    #[test]
    fn queued_events_run_one_after_the_other_not_inside_each_other() {
        // Two events fired by one builtin: handler A prints a1, calls a
        // builtin (which must not start B), prints a2; then B.
        let (mut m, mut main) = event_module(false, false);
        let mut a = std::mem::take(&mut m.functions[0]);
        let n = m.add_string("NOP");
        a.code.insert(0, 0);
        a.code.splice(0..1, [Op::CallBuiltin as u8].into_iter().chain(n.to_le_bytes()).chain([0u8, Op::Pop as u8]));
        m.functions[0] = a;
        let fire = m.add_string("FIRE");
        for f in [0i64, 1] {
            let c = m.add_const(Const::Int(f));
            main.code.push(Op::LoadConst as u8); main.code.extend_from_slice(&c.to_le_bytes());
            main.code.push(Op::CallBuiltin as u8); main.code.extend_from_slice(&fire.to_le_bytes()); main.code.push(1);
            main.code.push(Op::Pop as u8);
        }
        main.code.push(Op::Halt as u8);
        m.entry = m.add_function(main);
        let mut h = QueueHost::default();
        // Queue both before running: as a UI pump would.
        h.queue.push_back(QueuedEvent::new(0, vec![v_str("s")]));
        h.queue.push_back(QueuedEvent::new(1, vec![]));
        let mut vm = Vm::new(&mut h);
        vm.run(&m).unwrap();
        assert_eq!(h.inner.output, "a1\na2\nb\na1\na2\nb\n");
    }

    #[test]
    fn a_handler_that_waits_for_a_dialog_resumes_then_what_it_interrupted_continues() {
        let (mut m, mut main) = event_module(true, false);
        emit_print(&mut main, &mut m, "m1");
        emit_builtin(&mut main, &mut m, "FIRE", Some(0));
        main.code.push(Op::Pop as u8);
        emit_print(&mut main, &mut m, "m2");
        main.code.push(Op::Halt as u8);
        m.entry = m.add_function(main);
        let mut h = QueueHost::default();
        let mut vm = Vm::new(&mut h);
        assert!(matches!(vm.run(&m), Err(VmError::Suspended)));
        assert!(vm.is_waiting());
        vm.resume_with(&m, v_str("answer")).unwrap();
        assert!(vm.frames.is_empty() && vm.stack.is_empty());
        assert_eq!(h.inner.output, "m1\na1\nanswer\na2\nm2\n");
    }

    #[test]
    fn an_idle_event_runs_on_top_of_a_program_waiting_for_a_dialog() {
        // main: PRINT ASK ; handler B runs while main waits.
        let (mut m, mut main) = event_module(false, false);
        emit_builtin(&mut main, &mut m, "ASK", None);
        main.code.push(Op::PrintLn as u8);
        main.code.push(Op::Halt as u8);
        m.entry = m.add_function(main);
        let mut h = QueueHost::default();
        let mut vm = Vm::new(&mut h);
        assert!(matches!(vm.run(&m), Err(VmError::Suspended)));
        vm.invoke_function(&m, 1, vec![]).unwrap();
        assert!(vm.is_waiting());
        vm.resume_with(&m, v_str("done")).unwrap();
        assert_eq!(h.inner.output, "b\ndone\n");
    }

    #[test]
    fn a_wait_the_host_starts_serves_events_and_gives_its_result() {
        // main: PRINT MODAL  (two pumps, each queuing handler B, then 7)
        let (mut m, mut main) = event_module(false, false);
        emit_builtin(&mut main, &mut m, "MODAL", None);
        main.code.push(Op::PrintLn as u8);
        main.code.push(Op::Halt as u8);
        m.entry = m.add_function(main);
        let mut h = QueueHost::default();
        let mut vm = Vm::new(&mut h);
        vm.run(&m).unwrap();
        assert_eq!(h.inner.output, "b\nb\n7\n");
    }

    #[test]
    fn end_in_an_event_handler_ends_the_program() {
        let (mut m, mut main) = event_module(false, true);
        emit_builtin(&mut main, &mut m, "FIRE", Some(0));
        main.code.push(Op::Pop as u8);
        emit_print(&mut main, &mut m, "never");
        main.code.push(Op::Halt as u8);
        m.entry = m.add_function(main);
        let mut h = QueueHost::default();
        let mut vm = Vm::new(&mut h);
        vm.run(&m).unwrap();
        assert!(vm.frames.is_empty());
        assert_eq!(h.inner.output, "a1\n");
    }

    #[test]
    fn a_failing_handler_leaves_the_vm_as_it_was() {
        let mut m = Module::new();
        let mut bad = Function::default();
        bad.code.push(Op::Pop as u8); // stack underflow
        let fi = m.add_function(bad);
        let mut h = StubHost::default();
        let mut vm = Vm::new(&mut h);
        assert!(vm.invoke_function(&m, fi, vec![]).is_err());
        assert!(vm.frames.is_empty() && vm.stack.is_empty());
        assert!(vm.invoke_function(&m, 99, vec![v_int(1)]).is_err());
        assert!(vm.stack.is_empty());
    }

    /// [`QueueHost`] that yields at every question (each
    /// [`YIELD_CHECK_EVERY`] jumps and calls).
    #[derive(Default)]
    struct YieldHost {
        q: QueueHost,
        yields: usize,
    }

    impl Host for YieldHost {
        const YIELDS: bool = true;
        fn yield_now(&mut self) -> bool {
            self.yields += 1;
            true
        }
        fn call_builtin(&mut self, name: &str, args: &[Value]) -> Result<Value, String> { self.q.call_builtin(name, args) }
        fn suspend_requested(&mut self) -> bool { self.q.suspend_requested() }
        fn take_events(&mut self) -> Vec<QueuedEvent> { self.q.take_events() }
        fn create_comp(&mut self, k: &str, id: &str) -> Result<Value, String> { self.q.create_comp(k, id) }
        fn set_prop(&mut self, id: &str, n: &str, v: Value) -> Result<(), String> { self.q.set_prop(id, n, v) }
        fn get_prop(&mut self, id: &str, n: &str) -> Result<Value, String> { self.q.get_prop(id, n) }
        fn call_method(&mut self, id: &str, m: &str, a: &[Value]) -> Result<Value, String> { self.q.call_method(id, m, a) }
        fn register_event(&mut self, id: &str, e: &str, f: u32) -> Result<(), String> { self.q.register_event(id, e, f) }
        fn print(&mut self, s: &str) -> Result<(), String> { self.q.print(s) }
        fn input(&mut self) -> Result<String, String> { self.q.input() }
    }

    /// A loop counting local `slot` down from `n` (a backward JumpIf).
    fn emit_spin(f: &mut Function, m: &mut Module, slot: u16, n: i64) {
        let (cn, one) = (m.add_const(Const::Int(n)), m.add_const(Const::Int(1)));
        f.n_locals = f.n_locals.max(u32::from(slot) + 1);
        f.code.push(Op::LoadConst as u8); f.code.extend_from_slice(&cn.to_le_bytes());
        f.code.push(Op::StoreLocal as u8); f.code.extend_from_slice(&slot.to_le_bytes());
        let top = f.code.len() as u32;
        f.code.push(Op::LoadLocal as u8); f.code.extend_from_slice(&slot.to_le_bytes());
        f.code.push(Op::LoadConst as u8); f.code.extend_from_slice(&one.to_le_bytes());
        f.code.push(Op::Sub as u8);
        f.code.push(Op::StoreLocal as u8); f.code.extend_from_slice(&slot.to_le_bytes());
        f.code.push(Op::LoadLocal as u8); f.code.extend_from_slice(&slot.to_le_bytes());
        f.code.push(Op::JumpIf as u8); f.code.extend_from_slice(&top.to_le_bytes());
    }

    /// Runs (or continues) until the VM stops for something else than a
    /// yield; the number of yields.
    fn run_through_yields(vm: &mut Vm<'_, YieldHost>, m: &Module, mut result: Result<(), VmError>) -> (Result<(), VmError>, usize) {
        let mut n = 0;
        while matches!(result, Err(VmError::Yielded)) {
            n += 1;
            result = vm.resume(m);
        }
        (result, n)
    }

    #[test]
    fn a_busy_main_yields_and_continues_where_it_was() {
        // main: PRINT "a" ; spin 5000 ; PRINT f() * 2 where f spins and returns 21
        let mut m = Module::new();
        let c21 = m.add_const(Const::Int(21));
        let two = m.add_const(Const::Int(2));
        let mut f = Function::default();
        f.name = "f".into();
        emit_spin(&mut f, &mut m, 0, 3000);
        f.code.push(Op::LoadConst as u8); f.code.extend_from_slice(&c21.to_le_bytes());
        f.code.push(Op::RetVal as u8);
        let fi = m.add_function(f);
        let mut main = Function::default();
        main.name = "__main".into();
        emit_print(&mut main, &mut m, "a");
        emit_spin(&mut main, &mut m, 0, 5000);
        main.code.push(Op::CallFunc as u8); main.code.extend_from_slice(&fi.to_le_bytes()); main.code.push(0);
        main.code.push(Op::LoadConst as u8); main.code.extend_from_slice(&two.to_le_bytes());
        main.code.push(Op::Mul as u8);
        main.code.push(Op::PrintLn as u8);
        main.code.push(Op::Halt as u8);
        m.entry = m.add_function(main);
        let mut h = YieldHost::default();
        let mut vm = Vm::new(&mut h);
        let first = vm.run(&m);
        let (result, n) = run_through_yields(&mut vm, &m, first);
        result.unwrap();
        assert!(n >= 20, "yielded {n} times");
        assert!(vm.frames.is_empty() && vm.stack.is_empty());
        assert_eq!(h.q.inner.output, "a\n42\n");
    }

    #[test]
    fn a_handler_that_yields_finishes_then_the_events_behind_it_then_the_code_it_interrupted() {
        // Handler A: a1, spin, a2; B: b. main: m1, NOP (runs A then B), m2.
        let (mut m, mut main) = event_module(false, false);
        let mut a = std::mem::take(&mut m.functions[0]);
        let a2 = a.code.split_off(a.code.len() - 7); // LoadConst "a2", PrintLn, Ret
        emit_spin(&mut a, &mut m, 1, 2000);
        a.code.extend(a2);
        m.functions[0] = a;
        emit_print(&mut main, &mut m, "m1");
        emit_builtin(&mut main, &mut m, "NOP", None);
        main.code.push(Op::Pop as u8);
        emit_print(&mut main, &mut m, "m2");
        main.code.push(Op::Halt as u8);
        m.entry = m.add_function(main);
        let mut h = YieldHost::default();
        h.q.queue.push_back(QueuedEvent::new(0, vec![v_str("s")]));
        h.q.queue.push_back(QueuedEvent::new(1, vec![]));
        let mut vm = Vm::new(&mut h);
        let first = vm.run(&m);
        let (result, n) = run_through_yields(&mut vm, &m, first);
        result.unwrap();
        assert!(n > 0);
        assert!(vm.frames.is_empty() && vm.stack.is_empty() && vm.yield_rest.is_empty());
        assert_eq!(h.q.inner.output, "m1\na1\na2\nb\nm2\n");
    }

    #[test]
    fn a_handler_run_on_top_of_waiting_code_yields_and_the_waiting_code_still_waits() {
        // main: PRINT ASK ; handler A (spins) runs while main waits.
        let (mut m, mut main) = event_module(false, false);
        let mut a = std::mem::take(&mut m.functions[0]);
        a.code.pop(); // Ret
        emit_spin(&mut a, &mut m, 1, 2000);
        a.code.push(Op::Ret as u8);
        m.functions[0] = a;
        emit_builtin(&mut main, &mut m, "ASK", None);
        main.code.push(Op::PrintLn as u8);
        main.code.push(Op::Halt as u8);
        m.entry = m.add_function(main);
        let mut h = YieldHost::default();
        let mut vm = Vm::new(&mut h);
        assert!(matches!(vm.run(&m), Err(VmError::Suspended)));
        let first = vm.invoke_function(&m, 0, vec![v_str("s")]).map(drop);
        let (result, n) = run_through_yields(&mut vm, &m, first);
        result.unwrap();
        assert!(n > 0);
        assert!(vm.is_waiting());
        assert_eq!(vm.stack.len(), 0);
        vm.resume_with(&m, v_str("done")).unwrap();
        assert!(vm.frames.is_empty());
        assert_eq!(h.q.inner.output, "a1\na2\ndone\n");
    }

    #[test]
    fn call_builtin_via_host() {
        // Call HOSTUPPER on "hello" — StubHost returns uppercase.
        let mut m = Module::new();
        let c = m.add_const(Const::Str("hello".into()));
        let n = m.add_string("HOSTUPPER");
        let mut f = Function::default();
        f.code.push(Op::LoadConst as u8); f.code.extend_from_slice(&c.to_le_bytes());
        f.code.push(Op::CallBuiltin as u8); f.code.extend_from_slice(&n.to_le_bytes()); f.code.push(1);
        f.code.push(Op::PrintLn as u8);
        f.code.push(Op::Halt as u8);
        m.entry = m.add_function(f);
        let mut h = StubHost::default();
        let mut vm = Vm::new(&mut h);
        vm.run(&m).unwrap();
        assert_eq!(h.output, "HELLO\n");
    }
}
