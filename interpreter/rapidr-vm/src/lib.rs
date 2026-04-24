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
use rapidr_value::{v_bool, v_int, v_null, v_str};

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
    Halted,
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
            VmError::Halted => write!(f, "halted"),
        }
    }
}

impl std::error::Error for VmError {}

/// One activation frame.
struct Frame {
    fn_index: u32,
    locals: Vec<Value>,
    /// Saved instruction pointer in the calling function.
    ret_ip: usize,
    /// True if caller wanted a value (CallFunc), false for CallSub.
    wants_value: bool,
}

/// The interpreter.
pub struct Vm<'h, H: Host + ?Sized> {
    host: &'h mut H,
    stack: Vec<Value>,
    frames: Vec<Frame>,
    /// Globals — name-keyed Value slots (created lazily on first STORE).
    globals: std::collections::HashMap<String, Value>,
}

impl<'h, H: Host + ?Sized> Vm<'h, H> {
    pub fn new(host: &'h mut H) -> Self {
        Self { host, stack: Vec::with_capacity(64), frames: Vec::with_capacity(8), globals: Default::default() }
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
    fn call(&mut self, module: &Module, fn_index: u32, argc: u8, wants_value: bool) -> Result<(), VmError> {
        let f = module.functions.get(fn_index as usize)
            .ok_or(VmError::BadFunctionIndex(fn_index))?;
        let mut locals: Vec<Value> = (0..f.n_locals).map(|_| v_null()).collect();
        // Pop args off the stack into the first `argc` locals.
        for i in (0..argc as usize).rev() {
            let v = self.pop()?;
            if i < locals.len() {
                locals[i] = v;
            }
        }
        let ret_ip = self.frames.last().map(|fr| fr.locals.len() /* unused */ ).unwrap_or(0);
        // ret_ip placeholder — replaced by exec() loop's saved ip on push.
        self.frames.push(Frame { fn_index, locals, ret_ip, wants_value });
        let _ = ret_ip;
        Ok(())
    }

    fn exec(&mut self, module: &Module) -> Result<(), VmError> {
        // Per-frame instruction pointer; we keep it on the Rust stack for hot loop.
        let mut ip: usize = 0;
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
                if !self.return_frame(false)? {
                    return Ok(());
                }
                let top = self.frames.last().unwrap();
                ip = top.ret_ip;
                refresh!();
                continue;
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
                    self.frames.last_mut().unwrap().ret_ip = ip;
                    self.call(module, fi, argc, false)?;
                    ip = 0;
                    refresh!();
                }
                Op::CallFunc => {
                    let fi = read_u32(code, &mut ip)?;
                    let argc = read_u8(code, &mut ip)?;
                    self.frames.last_mut().unwrap().ret_ip = ip;
                    self.call(module, fi, argc, true)?;
                    ip = 0;
                    refresh!();
                }
                Op::Ret => {
                    if !self.return_frame(false)? { return Ok(()); }
                    ip = self.frames.last().unwrap().ret_ip;
                    refresh!();
                }
                Op::RetVal => {
                    if !self.return_frame(true)? { return Ok(()); }
                    ip = self.frames.last().unwrap().ret_ip;
                    refresh!();
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
                    let n = self.pop()?.to_i64().max(0) as usize;
                    // Represent arrays as a Value::String JSON-encoded for now.
                    // (Phase 2+ may replace with Value::Array if we introduce it.)
                    let s: String = (0..n).map(|_| "").collect::<Vec<_>>().join(",");
                    self.stack.push(v_str(&s));
                }
                Op::AGet => {
                    let idx = self.pop()?;
                    let arr = self.pop()?;
                    self.stack.push(arr.rp_index(&idx));
                }
                Op::ASet => {
                    // For now: read array as comma-separated, replace, write back.
                    // Phase 2+ will introduce a proper Value::Array variant.
                    let val = self.pop()?;
                    let idx = self.pop()?.to_i64();
                    let arr = self.pop()?;
                    let s = arr.to_string_val();
                    let mut parts: Vec<String> = s.split(',').map(|x| x.to_string()).collect();
                    if (idx as usize) < parts.len() {
                        parts[idx as usize] = val.to_string_val();
                    }
                    self.stack.push(v_str(&parts.join(",")));
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
                    let v = self.pop()?;
                    self.host.print(&v.to_string_val()).map_err(VmError::HostError)?;
                }
                Op::PrintLn => {
                    let v = self.pop()?;
                    let mut s = v.to_string_val();
                    s.push('\n');
                    self.host.print(&s).map_err(VmError::HostError)?;
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
    fn return_frame(&mut self, with_value: bool) -> Result<bool, VmError> {
        let ret = if with_value { self.pop()? } else { v_null() };
        let frame = self.frames.pop().ok_or(VmError::StackUnderflow)?;
        if self.frames.is_empty() {
            return Ok(false);
        }
        if frame.wants_value {
            self.stack.push(ret);
        }
        Ok(true)
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
        for a in args.iter().rev() {
            // Note: original `args` order matters; push in forward order so they pop correctly
            let _ = a;
        }
        for a in args.iter() { self.stack.push(a.clone()); }
        let argc = args.len() as u8;
        // Save state and run a nested execution.
        // For simplicity, we just push a frame and re-enter exec(). Since exec()
        // returns when the frame stack is empty, we need a scoped exec.
        let saved = std::mem::take(&mut self.frames);
        self.call(module, fn_index, argc, true)?;
        self.exec(module)?;
        let r = self.stack.pop().unwrap_or(v_null());
        self.frames = saved;
        Ok(r)
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
