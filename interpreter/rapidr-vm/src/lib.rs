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
}

impl std::fmt::Display for VmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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
        }
    }
}

impl std::error::Error for VmError {}

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
}

/// The interpreter.
pub struct Vm<'h, H: Host + ?Sized> {
    pub host: &'h mut H,
    pub stack: Vec<Value>,
    pub frames: Vec<Frame>,
    /// Globals — name-keyed Value slots (created lazily on first STORE).
    pub globals: std::collections::HashMap<String, Value>,
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
}

impl<'h, H: Host + ?Sized> Vm<'h, H> {
    pub fn new(host: &'h mut H) -> Self {
        Self {
            host,
            stack: Vec::with_capacity(64),
            frames: Vec::with_capacity(8),
            globals: Default::default(),
            arg_out: Vec::new(),
            print_col: 0,
            debug_mode: false,
            breakpoints: Default::default(),
            step_mode: StepMode::None,
            last_line: 0,
        }
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
        let f = module.functions.get(fn_index as usize)
            .ok_or(VmError::BadFunctionIndex(fn_index))?;
        let mut locals: Vec<Value> = (0..f.n_locals).map(|_| v_null()).collect();
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
        self.frames.push(Frame { fn_index, locals, ret_ip, wants_value, ip: 0, gosub: Vec::new() });
        let _ = ret_ip;
        Ok(())
    }

    fn exec(&mut self, module: &Module) -> Result<(), VmError> {
        // Per-frame instruction pointer; we keep it on the Rust stack for hot loop.
        let mut ip = self.frames.last().map(|f| f.ip).unwrap_or(0);
        // The currently executing function's code, refreshed on call/ret.
        let mut code: &[u8] = &module.functions[self.frames.last().unwrap().fn_index as usize].code;

        macro_rules! refresh {
            () => {
                code = &module.functions[self.frames.last().unwrap().fn_index as usize].code;
            };
        }

        loop {
            if ip >= code.len() {
                // Implicit return for missing trailing Halt.
                if !self.return_frame(module, false)? {
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

            let opbyte = code[ip];
            ip += 1;
            let op = Op::from_u8(opbyte).ok_or(VmError::BadOpcode(opbyte))?;
            match op {
                Op::Nop => {}
                Op::Halt => return Ok(()),

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
                    let i = read_u32(code, &mut ip)?;
                    let name = module.strings.get(i as usize).ok_or(VmError::BadStringIndex(i))?;
                    let v = self.globals.get(name).cloned().unwrap_or(v_null());
                    self.stack.push(v);
                }
                Op::StoreGlobal => {
                    let i = read_u32(code, &mut ip)?;
                    let name = module.strings.get(i as usize).ok_or(VmError::BadStringIndex(i))?.clone();
                    let v = self.pop()?;
                    self.globals.insert(name, v);
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
                    if !self.return_frame(module, false)? { return Ok(()); }
                    ip = self.frames.last().unwrap().ip;
                    refresh!();
                }
                Op::RetVal => {
                    if !self.return_frame(module, true)? { return Ok(()); }
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
                    self.stack.push(r);
                }

                // ----- components -----
                Op::CreateComp => {
                    let kind_i = read_u32(code, &mut ip)?;
                    let id_i = read_u32(code, &mut ip)?;
                    let kind = module.strings.get(kind_i as usize).ok_or(VmError::BadStringIndex(kind_i))?.clone();
                    let id = module.strings.get(id_i as usize).ok_or(VmError::BadStringIndex(id_i))?.clone();
                    let r = self.host.create_comp(&kind, &id).map_err(VmError::HostError)?;
                    self.stack.push(r);
                }
                Op::SetProp => {
                    let id_i = read_u32(code, &mut ip)?;
                    let prop_i = read_u32(code, &mut ip)?;
                    let id = module.strings.get(id_i as usize).ok_or(VmError::BadStringIndex(id_i))?.clone();
                    let prop = module.strings.get(prop_i as usize).ok_or(VmError::BadStringIndex(prop_i))?.clone();
                    let v = self.pop()?;
                    self.host.set_prop(&id, &prop, v).map_err(VmError::HostError)?;
                }
                Op::GetProp => {
                    let id_i = read_u32(code, &mut ip)?;
                    let prop_i = read_u32(code, &mut ip)?;
                    let id = module.strings.get(id_i as usize).ok_or(VmError::BadStringIndex(id_i))?.clone();
                    let prop = module.strings.get(prop_i as usize).ok_or(VmError::BadStringIndex(prop_i))?.clone();
                    let v = self.host.get_prop(&id, &prop).map_err(VmError::HostError)?;
                    self.stack.push(v);
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
                    self.stack.push(r);
                }
                Op::GetPropDyn => {
                    let prop_i = read_u32(code, &mut ip)?;
                    let prop = module.strings.get(prop_i as usize).ok_or(VmError::BadStringIndex(prop_i))?.clone();
                    let id = self.pop_object_id()?;
                    let v = self.host.get_prop(&id, &prop).map_err(VmError::HostError)?;
                    self.stack.push(v);
                }
                Op::SetPropDyn => {
                    let prop_i = read_u32(code, &mut ip)?;
                    let prop = module.strings.get(prop_i as usize).ok_or(VmError::BadStringIndex(prop_i))?.clone();
                    let id = self.pop_object_id()?;
                    let v = self.pop()?;
                    self.host.set_prop(&id, &prop, v).map_err(VmError::HostError)?;
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
                    self.stack.push(r);
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
                    let indices = self.pop_indices(n)?;
                    let target = self.pop()?;
                    let v = match &target {
                        Value::Array(a) => a.borrow().get(&indices).map_err(VmError::Runtime)?,
                        // Legacy: indexing a comma-separated string.
                        other if n == 1 => other.rp_index(&v_int(indices[0])),
                        _ => return Err(VmError::Runtime("indexing a value that is not an array".into())),
                    };
                    self.stack.push(v);
                }
                Op::ASet => {
                    let n = read_u8(code, &mut ip)? as usize;
                    let val = self.pop()?;
                    let indices = self.pop_indices(n)?;
                    match self.pop()? {
                        Value::Array(a) => a.borrow_mut().set(&indices, val).map_err(VmError::Runtime)?,
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
                    self.stack.push(v_str(&s));
                }
            }
        }
    }

    /// Pop the current frame and return a value (or Null) to the caller.
    /// Returns false if the popped frame was the entry — the VM must stop.
    fn return_frame(&mut self, module: &Module, with_value: bool) -> Result<bool, VmError> {
        let ret = if with_value { self.pop()? } else { v_null() };
        let frame = self.frames.pop().ok_or(VmError::StackUnderflow)?;
        let n_params = module
            .functions
            .get(frame.fn_index as usize)
            .map_or(0, |f| f.params.len())
            .min(frame.locals.len());
        self.arg_out = frame.locals[..n_params].to_vec();
        if self.frames.is_empty() {
            return Ok(false);
        }
        if frame.wants_value {
            self.stack.push(ret);
        }
        Ok(true)
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
    fn pop_indices(&mut self, n: usize) -> Result<Vec<i64>, VmError> {
        let mut indices = vec![0i64; n];
        for i in indices.iter_mut().rev() {
            *i = self.pop()?.to_i64();
        }
        Ok(indices)
    }

    fn pop(&mut self) -> Result<Value, VmError> {
        self.stack.pop().ok_or(VmError::StackUnderflow)
    }

    fn peek(&self) -> Result<&Value, VmError> {
        self.stack.last().ok_or(VmError::StackUnderflow)
    }

    /// Allow Hosts (event handlers) to invoke a function on this VM.
    /// Pushes args in order; any return value of a CallFunc target is left on
    /// the data stack. For event callbacks the caller typically discards it.
    pub fn invoke_function(&mut self, module: &Module, fn_index: u32, args: Vec<Value>) -> Result<Value, VmError> {
        for a in args.iter() { self.stack.push(a.clone()); }
        let argc = args.len() as u8;
        let saved = std::mem::take(&mut self.frames);
        self.call(module, fn_index, argc, true)?;
        match self.exec(module) {
            Ok(()) => {
                let r = self.stack.pop().unwrap_or(v_null());
                self.frames = saved;
                Ok(r)
            }
            Err(VmError::Paused) => {
                Err(VmError::Paused)
            }
            Err(e) => {
                self.frames = saved;
                Err(e)
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
        self.exec(module)
    }

    pub fn step_into(&mut self, module: &Module) -> Result<(), VmError> {
        self.step_mode = StepMode::Into;
        self.last_line = self.current_line(module).unwrap_or(0);
        self.exec(module)
    }

    pub fn step_over(&mut self, module: &Module) -> Result<(), VmError> {
        self.step_mode = StepMode::Over { target_depth: self.frames.len() };
        self.last_line = self.current_line(module).unwrap_or(0);
        self.exec(module)
    }

    pub fn step_out(&mut self, module: &Module) -> Result<(), VmError> {
        self.step_mode = StepMode::Out { target_depth: self.frames.len() };
        self.last_line = self.current_line(module).unwrap_or(0);
        self.exec(module)
    }

    pub fn current_line(&self, module: &Module) -> Option<u32> {
        let frame = self.frames.last()?;
        let func = module.functions.get(frame.fn_index as usize)?;
        func.get_line_for_ip(frame.ip)
    }
}

// ---------- operand decoders ----------

fn read_u8(code: &[u8], ip: &mut usize) -> Result<u8, VmError> {
    let b = *code.get(*ip).ok_or(VmError::Truncated)?;
    *ip += 1;
    Ok(b)
}
fn read_u16(code: &[u8], ip: &mut usize) -> Result<u16, VmError> {
    if *ip + 2 > code.len() { return Err(VmError::Truncated); }
    let v = u16::from_le_bytes([code[*ip], code[*ip + 1]]);
    *ip += 2; Ok(v)
}
fn read_u32(code: &[u8], ip: &mut usize) -> Result<u32, VmError> {
    if *ip + 4 > code.len() { return Err(VmError::Truncated); }
    let v = u32::from_le_bytes([code[*ip], code[*ip + 1], code[*ip + 2], code[*ip + 3]]);
    *ip += 4; Ok(v)
}
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
