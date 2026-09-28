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
            VmError::HostError(s) => write!(f, "host error: {s}"),
            VmError::Runtime(s) => write!(f, "run-time error: {s}"),
            VmError::Halted => write!(f, "halted"),
            VmError::Paused => write!(f, "paused"),
            VmError::Suspended => write!(f, "suspended"),
        }
    }
}

impl std::error::Error for VmError {}

/// Deepest nesting of SUB/FUNCTION calls, so runaway recursion stops with
/// an error instead of exhausting memory.
pub const MAX_CALL_DEPTH: usize = 100_000;

/// Most dimensions an array access may have.
const MAX_DIMS: usize = 8;

/// Width of a PRINT zone (`PRINT a, b`), as in QBasic and VB.
pub const PRINT_ZONE_WIDTH: usize = 14;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepMode {
    None,
    Into,
    Over { target_depth: usize },
    Out { target_depth: usize },
}

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
    pub breakpoints: std::collections::HashSet<u32>,
    pub step_mode: StepMode,
    pub last_line: u32,
    /// Locals vectors of returned frames, reused by the next calls (no
    /// allocation per SUB/FUNCTION call).
    spare_locals: Vec<Vec<Value>>,
    /// Offset of the instruction being run, for [`Self::error_line`].
    fault_ip: usize,
    /// The source line (of the compiled, preprocessed program) of the
    /// instruction a run-time error stopped at (also in the error:
    /// [`VmError::At`]).
    pub error_line: Option<u32>,
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
            step_mode: StepMode::None,
            last_line: 0,
            spare_locals: Vec::new(),
            fault_ip: 0,
            error_line: None,
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
        self.frames.push(Frame { fn_index, locals, ret_ip, wants_value, ip: 0, gosub: Vec::new(), stop: false, waiting: false });
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
            Err(e) if !matches!(e, VmError::Paused | VmError::Suspended | VmError::At { .. }) => {
                self.error_line = self
                    .frames
                    .last()
                    .and_then(|f| module.functions.get(f.fn_index as usize))
                    .and_then(|f| f.get_line_for_ip(self.fault_ip));
                let Some(line) = self.error_line else { return Err(e) };
                let (file, line) = match module.source_map.locate(line) {
                    Some((file, l)) => (Some(file.to_string()), l),
                    None => (None, line),
                };
                Err(VmError::At { error: Box::new(e), file, line })
            }
            other => other,
        }
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
                let current_fn = &module.functions[self.frames.last().unwrap().fn_index as usize];
                if let Some(line) = current_fn.get_line_for_ip(ip) {
                    if self.last_line != line {
                        let depth = self.frames.len();
                        let should_pause = self.breakpoints.contains(&line) || match self.step_mode {
                            StepMode::Into => true,
                            StepMode::Over { target_depth } => depth <= target_depth,
                            StepMode::Out { target_depth } => depth < target_depth,
                            StepMode::None => false,
                        };
                        if should_pause {
                            self.frames.last_mut().unwrap().ip = ip;
                            self.step_mode = StepMode::None;
                            self.last_line = line;
                            return Err(VmError::Paused);
                        }
                        self.last_line = line;
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
                    self.frames.clear();
                    self.stack.clear();
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
                Op::IDiv => { let b = self.pop()?; let a = self.pop()?; self.stack.push(a.int_div(&b)); }
                Op::Mod => { let b = self.pop()?; let a = self.pop()?; self.stack.push(&a % &b); }
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
                Op::Jump => { let t = read_u32(code, &mut ip)?; ip = t as usize; }
                Op::JumpIf => {
                    let t = read_u32(code, &mut ip)?;
                    let v = self.pop()?;
                    if v.to_bool() { ip = t as usize; }
                }
                Op::JumpIfNot => {
                    let t = read_u32(code, &mut ip)?;
                    let v = self.pop()?;
                    if !v.to_bool() { ip = t as usize; }
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
                }
                Op::CallFunc => {
                    let fi = read_u32(code, &mut ip)?;
                    let argc = read_u8(code, &mut ip)?;
                    self.frames.last_mut().unwrap().ip = ip;
                    self.frames.last_mut().unwrap().ret_ip = ip;
                    self.call(module, fi, argc, true)?;
                    ip = 0;
                    refresh!();
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
                    let kind = rapidr_value::numeric::NumKind::from_code(read_u8(code, &mut ip)?).ok_or(VmError::BadOperand)?;
                    let v = self.pop()?;
                    self.stack.push(rapidr_value::numeric::convert(&v, kind));
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
                    let s = self.pop()?.to_string_val();
                    self.emit_output(&s)?;
                }
                Op::PrintLn => {
                    let mut s = self.pop()?.to_string_val();
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
            let events = self.host.take_events();
            if !events.is_empty() && !self.run_events(module, ip, events)? {
                return Ok(false);
            }
            if !waiting {
                return Ok(true);
            }
            if let Some(result) = self.host.pump() {
                if has_result {
                    self.pop()?;
                    self.stack.push(result);
                }
                // Once more for the events the last pump queued.
                waiting = false;
            }
        }
    }

    /// Runs `events` one after the other, each to completion, on top of the
    /// current frame (whose `ip` is saved first, so a handler that suspends
    /// leaves this code ready to continue; the events after it go back to
    /// the host). Returns false if a handler ENDed the program.
    fn run_events(&mut self, module: &Module, ip: usize, events: Vec<(u32, Vec<Value>)>) -> Result<bool, VmError> {
        let mut events = events.into_iter();
        while let Some((fn_index, args)) = events.next() {
            if let Some(top) = self.frames.last_mut() {
                top.ip = ip;
            }
            if let Err(e) = self.invoke_function(module, fn_index, args) {
                let rest: Vec<_> = events.collect();
                if !rest.is_empty() {
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
            if frame.wants_value {
                self.stack.push(ret);
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
        let (base_frames, base_stack) = (self.frames.len(), self.stack.len());
        let argc = u8::try_from(args.len()).map_err(|_| VmError::Runtime("too many event arguments".into()))?;
        self.stack.extend(args);
        if let Err(e) = self.call(module, fn_index, argc, true) {
            self.stack.truncate(base_stack);
            return Err(e);
        }
        self.frames.last_mut().unwrap().stop = true;
        match self.exec(module) {
            // The handler returned (its value is on top), or ENDed.
            Ok(()) if self.frames.len() == base_frames => Ok(self.stack.pop().unwrap_or(Value::Null)),
            Ok(()) => Ok(Value::Null),
            Err(e @ (VmError::Suspended | VmError::Paused)) => Err(e),
            Err(e) => {
                // A failed handler leaves nothing behind.
                self.frames.truncate(base_frames);
                self.stack.truncate(base_stack);
                Err(e)
            }
        }
    }

    /// Continues after a pause or suspension until the program ends, waits
    /// again, or pauses/suspends. A finished event handler's value is
    /// dropped (whoever ran it is gone) and the code it interrupted
    /// continues, unless that code itself waits for a dialog.
    fn continue_run(&mut self, module: &Module) -> Result<(), VmError> {
        loop {
            self.exec(module)?;
            let Some(top) = self.frames.last() else { return Ok(()) };
            let waiting = top.waiting;
            // exec stopped at an event handler's frame: drop its value.
            self.stack.pop();
            if waiting {
                return Ok(());
            }
        }
    }

    pub fn is_paused(&self) -> bool {
        !self.frames.is_empty()
    }

    pub fn set_breakpoints(&mut self, bps: std::collections::HashSet<u32>) {
        self.breakpoints = bps;
    }

    pub fn add_breakpoint(&mut self, line: u32) {
        self.breakpoints.insert(line);
    }

    pub fn remove_breakpoint(&mut self, line: u32) {
        self.breakpoints.remove(&line);
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
        queue: std::collections::VecDeque<(u32, Vec<Value>)>,
        asking: bool,
        wait: Option<u32>,
        started: bool,
    }

    impl Host for QueueHost {
        fn call_builtin(&mut self, name: &str, args: &[Value]) -> Result<Value, String> {
            match name {
                "FIRE" => self.queue.push_back((args[0].to_i64() as u32, vec![v_str("sender")])),
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
        fn take_events(&mut self) -> Vec<(u32, Vec<Value>)> { self.queue.drain(..).collect() }
        fn wait_started(&mut self) -> bool { std::mem::take(&mut self.started) }
        fn pump(&mut self) -> Option<Value> {
            match self.wait {
                Some(0) | None => {
                    self.wait = None;
                    Some(v_int(7))
                }
                Some(n) => {
                    self.wait = Some(n - 1);
                    self.queue.push_back((1, vec![]));
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
        h.queue.push_back((0, vec![v_str("s")]));
        h.queue.push_back((1, vec![]));
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
