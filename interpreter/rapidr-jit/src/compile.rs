//! From the bytecode to the code generator's IR (docs/jit-design.md §2,
//! §4): one *form* of a function at a time.
//!
//! - An **entry form** is entered from the interpreter: at a loop's head
//!   (on-stack replacement, the loop's instructions its region) or at the
//!   function's start (the whole function). It reads the frame's locals it
//!   uses from a buffer the VM filled, and leaves at every place it can't
//!   go on — an instruction outside the compiled subset, a jump out of its
//!   region, its function's `Ret`, a run-time error about to happen, the
//!   debugger's interrupt — writing the frame back first (an *exit*: its
//!   record says where each value goes and of what kind).
//! - A **callee form** is called by compiled code, machine call to machine
//!   call, with its parameters in registers, specialised for the kinds of
//!   its arguments. It returns (0, its value); when it can't go on, it
//!   writes its whole frame out (the VM will make it a real frame) and
//!   returns 1, and every caller up to the entry form does the same at its
//!   call — so the interpreter finds every frame where it would have been.
//!
//! What a value is at each point is worked out from the bytecode and the
//! kinds the form was entered with: a local or the stack entry is Null, a
//! boolean, an Integer (`i64`) or a Double (`f64`) — a variable of the
//! code generator's per (slot, kind). Every operation compiled here is the
//! interpreter's own operation on those kinds (`rapidr_vm::fast`), written
//! out; anything else leaves to the interpreter, which runs the very
//! instruction compiled code stopped before.
//!
//! What compiled code dereferences: the context's fields, the locals
//! buffer, the globals buffer, its own stack slots — every offset a
//! constant checked here against the module the verifier passed. Arrays are
//! reached only through the helpers (`native.rs`), bounds checked there.

use std::collections::{BTreeSet, HashMap};

use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{self, types, AbiParam, Block, BlockArg, InstBuilder, MemFlagsData, Signature, StackSlotData, StackSlotKind, UserFuncName, Value as V};
use cranelift_codegen::isa::CallConv;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::JITModule;
use cranelift_module::{FuncId, Linkage, Module as _};

use rapidr_bytecode::disasm::decode;
use rapidr_bytecode::{Bin, Cmp, Const, Mode, Module, Op, NO_KIND};
use rapidr_value::numeric::{NumKind, ARG_ROUND};
use rapidr_value::Value;

use crate::kinds::K;

/// The context's fields compiled code reads (`native::Ctx`, `repr(C)`).
pub const CTX_GLOBALS: i32 = 0;
pub const CTX_INTERRUPT: i32 = 8;
pub const CTX_STACK_LIMIT: i32 = 16;
pub const CTX_TURNS: i32 = 24;

/// A function's array handles: the module's global arrays, or the entry
/// frame's local ones.
pub const TABLE_GLOBAL: i64 = 0;
pub const TABLE_LOCAL: i64 = 1;

const NO: &[BlockArg] = &[];

/// Why compiled code went back to the interpreter at an exit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// An instruction compiled code doesn't run.
    Planned,
    /// The kinds differ where two paths meet (the second path leaves).
    Conflict,
    /// A jump out of a loop form's region.
    LoopExit,
    /// The entry form's function returns (the interpreter returns).
    Return,
    /// A value of another kind than the code was compiled for.
    Guard,
    /// A run-time error about to happen: the interpreter raises it.
    Error,
    /// The debugger's interrupt.
    Interrupt,
    /// Too deep: the interpreter makes the call (and its error).
    Depth,
    /// The callee went back to the interpreter: this frame does too, at
    /// the instruction after the call.
    AfterCall { wants_value: bool },
    /// A call whose callee isn't worked out yet (only in a batch's early
    /// rounds, never in code that runs).
    Pending,
}

impl Why {
    /// It makes a loop not worth compiling when inside its region.
    fn unclean(self) -> bool {
        matches!(self, Why::Planned | Why::Conflict)
    }
}

/// An exit: where the interpreter goes on, and where each value of the
/// buffer compiled code hands over goes.
#[derive(Debug, Clone)]
pub struct ExitRecord {
    pub fn_index: u32,
    pub ip: u32,
    /// Locals written back (in order; a Null has no value in the buffer).
    pub locals: Vec<(u16, K)>,
    /// The operand stack (the frame's part), bottom first.
    pub stack: Vec<K>,
    /// The stack entries that are a global array (`K::Arr`, `LoadGlobal`
    /// of one before its `AGet`): their depth and the global's name — the
    /// array the global holds then (compiled code never makes a global
    /// hold another array).
    pub arrays: Vec<(u16, u32)>,
    pub why: Why,
    /// The form it belongs to (the engine's index; set when committed).
    pub form: usize,
    /// The program's line there (the compiled program's: the module's
    /// source map gives the file's), 0 when not known; and, where it
    /// leaves at an instruction compiled code doesn't run, that
    /// instruction (`rapidr run --verbose`).
    pub line: u32,
    pub what: String,
}

/// What kind of form is built.
#[derive(Debug, Clone)]
pub enum FormKind {
    /// Entered from the interpreter at `ip`; jumps leaving `region` exit.
    Entry { ip: usize, region: (usize, usize) },
    /// Called by compiled code with parameters of these kinds.
    Callee { params: Vec<K> },
}

/// Whether a callee form may be called (worked out over a batch).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Callable {
    Unknown,
    Yes,
    No,
}

/// A callee form, by its function and parameter kinds.
#[derive(Debug, Clone)]
pub struct CalleeInfo {
    pub func: FuncId,
    pub ret: Option<K>,
    pub callable: Callable,
    /// The globals and global arrays it (and what it calls) uses.
    pub globals: BTreeSet<usize>,
    pub garrays: BTreeSet<usize>,
    /// Its machine code exists (defined in an earlier batch).
    pub defined: bool,
    pub calls: BTreeSet<(u32, Vec<K>)>,
}

/// What the VM's values were when a form was compiled: the kinds it is
/// specialised for.
pub struct Observed<'a> {
    pub globals: &'a [Option<Value>],
    /// The entry frame's locals (entry forms).
    pub locals: Option<&'a [Value]>,
}

/// The module-wide part: globals, global arrays and callee forms, shared
/// by every form of the module.
pub struct Env {
    /// The globals compiled code uses, by their index in the buffer: the
    /// name's string id and the kind every form assumes.
    pub globals: Vec<(u32, K)>,
    gindex: HashMap<u32, usize>,
    /// The global arrays compiled code indexes (handle = index): name and
    /// the kind its elements were when first compiled.
    pub garrays: Vec<(u32, K)>,
    gaindex: HashMap<u32, usize>,
    pub callees: HashMap<(u32, Vec<K>), CalleeInfo>,
    /// The kinds the blocks of each form take where paths meet (`K::Dyn`
    /// locals), found by its walks: a form built again — a batch builds
    /// each a few times — starts from them (they only ever add a `Dyn`
    /// where two paths of that form, entered with those kinds, met with
    /// different ones).
    joins: HashMap<JoinKey, HashMap<usize, Vec<K>>>,
    /// Each global's declared kind (a `StoreGlobalNum` of it anywhere).
    declared: HashMap<u32, K>,
    names: u32,
    /// The host lets compiled code call the pure maths builtins.
    pub direct_maths: bool,
}

impl Env {
    pub fn new(m: &Module) -> Env {
        let mut declared = HashMap::new();
        for f in &m.functions {
            let mut ip = 0;
            while let Some(insn) = decode(&f.code, ip) {
                if insn.op == Op::StoreGlobalNum {
                    let kind = NumKind::from_code(insn.operands[1] as u8);
                    let k = match kind {
                        Some(NumKind::Double | NumKind::Single) => K::Dbl,
                        // (an INT64 / UINT32 holds an INT64, a UINT64 one)
                        Some(NumKind::UInt64) => K::U64,
                        Some(k) if k.is_wide() => K::I64,
                        Some(_) => K::Int,
                        None => K::Other,
                    };
                    declared.insert(insn.operands[0] as u32, k);
                }
                ip = insn.next;
            }
        }
        Env { globals: Vec::new(), gindex: HashMap::new(), garrays: Vec::new(), gaindex: HashMap::new(), callees: HashMap::new(), joins: HashMap::new(), declared, names: 0, direct_maths: false }
    }

    /// Global `name`'s buffer index and kind (fixed the first time).
    fn global(&mut self, name: u32, obs: &Observed) -> (usize, K) {
        if let Some(&i) = self.gindex.get(&name) {
            return (i, self.globals[i].1);
        }
        let seen = obs.globals.get(name as usize).and_then(|v| v.as_ref()).map_or(K::Null, K::of);
        let k = match (seen, self.declared.get(&name)) {
            (K::Null, Some(&d)) if d.scalar() => d,
            (seen, _) => seen,
        };
        self.globals.push((name, k));
        let i = self.globals.len() - 1;
        self.gindex.insert(name, i);
        (i, k)
    }

    /// Global array `name`'s handle and element kind, when it holds an
    /// array now.
    fn garray(&mut self, name: u32, obs: &Observed) -> Option<(usize, K)> {
        if let Some(&i) = self.gaindex.get(&name) {
            return Some((i, self.garrays[i].1));
        }
        let Some(Some(Value::Array(a))) = obs.globals.get(name as usize) else { return None };
        let elem = element_kind(&a.borrow())?;
        self.garrays.push((name, elem));
        let i = self.garrays.len() - 1;
        self.gaindex.insert(name, i);
        Some((i, elem))
    }

    pub fn next_name(&mut self) -> String {
        self.names += 1;
        format!("rr{}", self.names)
    }
}

/// The kind of a one-dimension array's elements (its first one), when it
/// is a scalar.
fn element_kind(a: &rapidr_value::BasicArray) -> Option<K> {
    if a.bounds.len() != 1 {
        return None;
    }
    let k = a.data.first().map_or(K::Null, K::of);
    k.scalar().then_some(k)
}

/// The helpers compiled code calls (`native.rs`), declared once per
/// module of compiled code.
#[derive(Clone, Copy)]
pub struct Helpers {
    pub exit: FuncId,
    pub aget: FuncId,
    pub aset: FuncId,
    pub conv: FuncId,
    pub math: FuncId,
    pub math2: FuncId,
    pub powf: FuncId,
}

impl Helpers {
    pub fn declare(jm: &mut JITModule) -> Result<Helpers, String> {
        let cc = jm.isa().default_call_conv();
        let sig = |params: &[ir::Type], rets: &[ir::Type]| {
            let mut s = Signature::new(cc);
            s.params.extend(params.iter().map(|&t| AbiParam::new(t)));
            s.returns.extend(rets.iter().map(|&t| AbiParam::new(t)));
            s
        };
        use types::{F64, I32, I64};
        let mut d = |name: &str, s: Signature| jm.declare_function(name, Linkage::Import, &s).map_err(|e| e.to_string());
        Ok(Helpers {
            exit: d("rapidr_jit_exit", sig(&[I64, I32, I64, I32], &[]))?,
            aget: d("rapidr_jit_aget", sig(&[I64, I32, I32, I64, I32, I64], &[I32]))?,
            aset: d("rapidr_jit_aset", sig(&[I64, I32, I32, I64, I32, I64], &[I32]))?,
            conv: d("rapidr_jit_conv", sig(&[I32, I32, I64], &[I64]))?,
            math: d("rapidr_jit_math", sig(&[I32, I32, I64, I64], &[I32]))?,
            math2: d("rapidr_jit_math2", sig(&[I32, I32, I64, I32, I64, I64], &[I32]))?,
            powf: d("rapidr_jit_powf", sig(&[F64, F64], &[F64]))?,
        })
    }
}

/// The signature of an entry form: `(ctx, locals) -> u32`, as the VM's
/// glue calls it (`extern "C"`).
pub fn entry_signature(jm: &JITModule) -> Signature {
    let mut s = Signature::new(jm.isa().default_call_conv());
    s.params.push(AbiParam::new(types::I64));
    s.params.push(AbiParam::new(types::I64));
    s.returns.push(AbiParam::new(types::I32));
    s
}

/// The signature of a callee form: `(ctx, its non-Null parameters)
/// -> (status, value bits)` (the generator's own convention between its
/// functions).
pub fn callee_signature(params: &[K]) -> Signature {
    let mut s = Signature::new(CallConv::Tail);
    s.params.push(AbiParam::new(types::I64));
    for k in params {
        if let Some(t) = ty(*k) {
            s.params.push(AbiParam::new(t));
        }
    }
    s.returns.push(AbiParam::new(types::I32));
    s.returns.push(AbiParam::new(types::I64));
    s
}

/// The machine type of a kind's values.
fn ty(k: K) -> Option<ir::Type> {
    match k {
        K::Bool => Some(types::I8),
        K::Int | K::I64 | K::U64 => Some(types::I64),
        K::Dbl => Some(types::F64),
        _ => None,
    }
}

/// The kinds of the locals where two paths meet ([`K::meet`]): `None`
/// when one differs in a way compiled code can't hold.
fn meet_all(a: &[K], b: &[K]) -> Option<Vec<K>> {
    if a.len() != b.len() {
        return None;
    }
    a.iter().zip(b).map(|(x, y)| x.meet(*y)).collect()
}

/// The instruction at `ip` in a few words (`rapidr run --verbose`): its
/// name and the name it is about — `CallBuiltin MID$`, `LoadGlobal s`,
/// `Mul`.
pub fn what(m: &Module, f: &rapidr_bytecode::Function, ip: usize) -> String {
    let Some(insn) = decode(&f.code, ip) else { return String::new() };
    let mut text = format!("{:?}", insn.op);
    for (o, v) in insn.op.layout().iter().zip(&insn.operands) {
        let name = match o {
            rapidr_bytecode::Operand::Global | rapidr_bytecode::Operand::Str => m.strings.get(*v as usize).cloned(),
            rapidr_bytecode::Operand::Func => m.functions.get(*v as usize).map(|f| f.name.clone()),
            rapidr_bytecode::Operand::Local => f.local_names.get(*v as usize).filter(|n| !n.is_empty()).cloned(),
            // (a text constant: the text, short)
            rapidr_bytecode::Operand::Const => match m.consts.get(*v as usize) {
                Some(Const::Str(t)) => Some(format!("{:?}", if t.chars().count() > 24 { format!("{}...", t.chars().take(21).collect::<String>()) } else { t.clone() })),
                _ => None,
            },
            _ => None,
        };
        if let Some(n) = name {
            text.push(' ');
            text.push_str(&n);
            break;
        }
    }
    text
}

/// A kind `Value::bits` makes a 32-bit integer of for AND / OR / XOR
/// (not a 64-bit one: those are `wide::bitwise`'s).
fn basic(k: K) -> bool {
    matches!(k, K::Int | K::Bool | K::Dbl | K::Null)
}

/// Whether `insn` reads a local that is a `K::Dyn` here (a store into
/// one is fine: it becomes of the stored kind).
fn reads_dyn(insn: &rapidr_bytecode::disasm::Insn, kinds: &[K]) -> bool {
    let writes_first = matches!(insn.op, Op::StoreLocal | Op::StoreLocalNum | Op::MoveLocal);
    insn.op
        .layout()
        .iter()
        .zip(&insn.operands)
        .enumerate()
        .any(|(i, (o, v))| *o == rapidr_bytecode::Operand::Local && !(writes_first && i == 0) && kinds.get(*v as usize) == Some(&K::Dyn))
}

/// A path with locals of kinds `from` can go into a block built for
/// `to`: the same kinds, or a `K::Dyn` there for a scalar or a `Dyn`.
fn fits(from: &[K], to: &[K]) -> bool {
    from.len() == to.len() && from.iter().zip(to).all(|(&f, &t)| f == t || (t == K::Dyn && (f.scalar() || f == K::Dyn)))
}

/// What building a form found out.
#[derive(Debug, Default, Clone)]
pub struct FormOut {
    /// Exits inside the region that make it not worth compiling (an
    /// instruction outside the subset, kinds that disagree).
    pub unclean: Vec<(usize, Why)>,
    /// The callee forms it calls.
    pub calls: BTreeSet<(u32, Vec<K>)>,
    /// A call left to the interpreter because the callee wasn't worked
    /// out yet (another round of the batch may change it).
    pub pending: bool,
    /// The kinds its `RetVal`s return (callee forms).
    pub rets: BTreeSet<K>,
    /// The entry frame's locals it reads or writes (entry forms), with
    /// the kinds they had when it was entered.
    pub used_locals: BTreeSet<u16>,
    pub globals: BTreeSet<usize>,
    pub garrays: BTreeSet<usize>,
    /// The entry frame's array locals it indexes (handle = index) and
    /// their element kinds.
    pub larrays: Vec<(u16, K)>,
    pub records: Vec<ExitRecord>,
}

/// Something about to be built into a form wasn't (a bug guard: the form
/// is dropped, the interpreter runs).
#[derive(Debug)]
pub struct Unbuilt(pub String);

/// Builds one form of function `fi` into `func` (a function of the code
/// generator, signature set). `used_hint`: the locals the last round
/// found it uses (the final build loads and writes back only those).
#[allow(clippy::too_many_arguments)]
pub fn build(
    m: &Module,
    fi: u32,
    kind: &FormKind,
    entry_kinds: &[K],
    func: &mut ir::Function,
    fctx: &mut FunctionBuilderContext,
    jm: &mut JITModule,
    helpers: Helpers,
    env: &mut Env,
    obs: &Observed,
    used_hint: Option<&BTreeSet<u16>>,
    record_base: usize,
) -> Result<FormOut, Unbuilt> {
    let f = &m.functions[fi as usize];
    let code = &f.code;
    // (where blocks start: every jump target, the entry)
    let mut leaders = vec![false; code.len() + 1];
    let mut ip = 0;
    while ip < code.len() {
        let insn = decode(code, ip).ok_or_else(|| Unbuilt(format!("undecodable at {ip}")))?;
        for (o, v) in insn.op.layout().iter().zip(&insn.operands) {
            if *o == rapidr_bytecode::Operand::Target {
                leaders[*v as usize] = true;
            }
        }
        ip = insn.next;
    }
    let frontend = jm.isa().frontend_config();
    // (paths that meet with a local of different kinds — a DIM inside one
    // CASE branch — make it a `K::Dyn` there: found while walking, the
    // block was already built for the first path's kinds, so the walk is
    // made again with the kinds every block takes, until none changes)
    let key: JoinKey = (fi, match kind {
        FormKind::Entry { ip, .. } => Some(*ip),
        FormKind::Callee { .. } => None,
    }, match kind {
        FormKind::Entry { .. } => entry_kinds.to_vec(),
        FormKind::Callee { params } => params.clone(),
    });
    let mut joins: HashMap<usize, Vec<K>> = env.joins.get(&key).cloned().unwrap_or_default();
    let mut rounds = 0;
    loop {
        let b = FunctionBuilder::new(&mut *func, &mut *fctx);
        let mut w = Walk {
            m,
            f,
            fi,
            kind: kind.clone(),
            b,
            jm: &mut *jm,
            helpers,
            env: &mut *env,
            obs,
            used_hint,
            leaders: leaders.clone(),
            blocks: HashMap::new(),
            work: Vec::new(),
            vars: HashMap::new(),
            frefs: HashMap::new(),
            out: FormOut::default(),
            ctx: None,
            locals_ptr: None,
            record_base,
            larray_index: HashMap::new(),
            arr_vals: HashMap::new(),
            exit_slot: None,
            joins: std::mem::take(&mut joins),
            rejoin: false,
        };
        w.run(entry_kinds)?;
        let Walk { mut b, out, joins: found, rejoin, .. } = w;
        b.seal_all_blocks();
        b.finalize(frontend);
        if !rejoin || rounds >= MAX_REJOINS {
            env.joins.insert(key, found);
            return Ok(out);
        }
        rounds += 1;
        joins = found;
        *func = ir::Function::with_name_signature(func.name.clone(), func.signature.clone());
    }
}

/// A form, for the kinds its blocks take: its routine, its loop head (an
/// entry form) and the kinds it is entered with.
type JoinKey = (u32, Option<usize>, Vec<K>);

/// Walks made again for paths that meet with other kinds (each makes
/// more locals `K::Dyn`, never fewer: a few at most); past them the
/// second path leaves, as before.
const MAX_REJOINS: usize = 8;

/// A value on the abstract operand stack: the code generator's value (a
/// dummy for Null) and its kind.
type Entry = (V, K);

struct Walk<'a, 'b> {
    m: &'a Module,
    f: &'a rapidr_bytecode::Function,
    fi: u32,
    kind: FormKind,
    b: FunctionBuilder<'b>,
    jm: &'a mut JITModule,
    helpers: Helpers,
    env: &'a mut Env,
    obs: &'a Observed<'a>,
    used_hint: Option<&'a BTreeSet<u16>>,
    leaders: Vec<bool>,
    /// The block of each instruction offset reached, and the kinds there.
    blocks: HashMap<usize, (Block, Vec<K>, Vec<K>)>,
    work: Vec<usize>,
    /// A variable per (slot, kind); the stack's entries at block starts
    /// are slots from 0x1_0000 on.
    vars: HashMap<(u32, K), Variable>,
    frefs: HashMap<FuncId, ir::FuncRef>,
    out: FormOut,
    ctx: Option<V>,
    locals_ptr: Option<V>,
    record_base: usize,
    larray_index: HashMap<u16, usize>,
    /// The global arrays on the stack (`K::Arr` entries: a `LoadGlobal` of
    /// one, then its `AGet 1` / `ASet 1`): the global's name and the handle
    /// and element kind compiled code reaches it by.
    arr_vals: HashMap<V, (u32, (i64, usize, K))>,
    /// The buffer every exit hands its values over in (one for the form:
    /// an exit ends its run, so no two are ever in use at once), and how
    /// many values it holds.
    exit_slot: Option<(ir::StackSlot, usize)>,
    /// The kinds of the locals at the blocks where paths meet with
    /// different ones (`K::Dyn` there), from the walk before.
    joins: HashMap<usize, Vec<K>>,
    /// A meeting found that the blocks built don't allow for: walk again.
    rejoin: bool,
}

/// The stack entry slot of depth `d` (its variables beside the locals').
const STACK_VARS: u32 = 0x1_0000;
/// A `K::Dyn` local's kind code, in a variable beside its bits'.
const DYN_TAGS: u32 = 0x2_0000;
/// The variable counting the loop turns of a run.
const TURNS_VAR: u32 = 0x3_0000;

impl Walk<'_, '_> {
    fn is_entry(&self) -> bool {
        matches!(self.kind, FormKind::Entry { .. })
    }

    fn in_region(&self, ip: usize) -> bool {
        match &self.kind {
            FormKind::Entry { region, .. } => region.0 <= ip && ip < region.1,
            FormKind::Callee { .. } => true,
        }
    }

    fn var(&mut self, slot: u32, k: K) -> Variable {
        if let Some(v) = self.vars.get(&(slot, k)) {
            return *v;
        }
        // (a `K::Dyn`'s bits: 64 of them, whatever its kind)
        let t = if k == K::Dyn { types::I64 } else { ty(k).expect("a variable of a scalar kind") };
        let v = self.b.declare_var(t);
        self.vars.insert((slot, k), v);
        v
    }

    /// The loop turns this run of the form has made (an entry form's exit
    /// writes them to `Ctx::turns`).
    fn turns_var(&mut self) -> Variable {
        self.var(TURNS_VAR, K::Int)
    }

    /// A `K::Dyn` local's two variables: its bits and its kind's code.
    fn dyn_vars(&mut self, slot: u32) -> (Variable, Variable) {
        (self.var(slot, K::Dyn), self.var(DYN_TAGS + slot, K::Int))
    }

    /// Going from a point with locals of kinds `from` to a block whose
    /// locals are of kinds `to` (the same, or `K::Dyn` where paths meet):
    /// each local that becomes a `Dyn` there gets its bits and its kind.
    fn widen(&mut self, from: &[K], to: &[K]) {
        for (s, (&f, &t)) in from.iter().zip(to).enumerate() {
            if t != K::Dyn || f == K::Dyn {
                continue;
            }
            let bits = if f == K::Null {
                self.b.ins().iconst(types::I64, 0)
            } else {
                let var = self.var(s as u32, f);
                let v = self.b.use_var(var);
                self.value_to_bits(v, f)
            };
            let tag = self.b.ins().iconst(types::I64, i64::from(f.code()));
            let (bv, tv) = self.dyn_vars(s as u32);
            self.b.def_var(bv, bits);
            self.b.def_var(tv, tag);
        }
    }

    /// The kinds of the block at `target` for a path coming with `kinds`:
    /// the path's, met with what the walk before found there.
    fn block_kinds(&self, target: usize, kinds: &[K]) -> Vec<K> {
        match self.joins.get(&target) {
            Some(h) if h.len() == kinds.len() => meet_all(h, kinds).unwrap_or_else(|| kinds.to_vec()),
            _ => kinds.to_vec(),
        }
    }

    fn fref(&mut self, id: FuncId) -> ir::FuncRef {
        if let Some(r) = self.frefs.get(&id) {
            return *r;
        }
        let r = self.jm.declare_func_in_func(id, self.b.func);
        self.frefs.insert(id, r);
        r
    }

    fn dummy(&mut self) -> V {
        self.b.ins().iconst(types::I64, 0)
    }

    fn ctx(&self) -> V {
        self.ctx.expect("ctx")
    }

    fn run(&mut self, entry_kinds: &[K]) -> Result<(), Unbuilt> {
        let first = self.b.create_block();
        self.b.append_block_params_for_function_params(first);
        self.b.switch_to_block(first);
        let params = self.b.block_params(first).to_vec();
        self.ctx = Some(params[0]);
        let turns = self.turns_var();
        let zero = self.b.ins().iconst(types::I64, 0);
        self.b.def_var(turns, zero);
        let mut kinds = entry_kinds.to_vec();
        let n_locals = self.f.n_locals as usize;
        kinds.resize(n_locals, K::Null);
        let start = match self.kind.clone() {
            FormKind::Entry { ip, .. } => {
                self.locals_ptr = Some(params[1]);
                // (the locals it uses, from the buffer the VM filled)
                for (s, &k) in kinds.iter().enumerate().take(n_locals) {
                    if ty(k).is_none() || self.used_hint.is_some_and(|u| !u.contains(&(s as u16))) {
                        continue;
                    }
                    let raw = self.b.ins().load(types::I64, MemFlagsData::trusted(), params[1], (s * 8) as i32);
                    let v = self.bits_to_value(raw, k);
                    let var = self.var(s as u32, k);
                    self.b.def_var(var, v);
                }
                ip
            }
            FormKind::Callee { params: pk } => {
                let mut next = 1;
                for (s, k) in pk.iter().enumerate() {
                    if let Some(_t) = ty(*k) {
                        let var = self.var(s as u32, *k);
                        self.b.def_var(var, params[next]);
                        next += 1;
                    }
                    kinds[s] = *k;
                }
                // (the debugger's interrupt, at the routine's start)
                let stack: Vec<Entry> = Vec::new();
                self.interrupt_check(0, &kinds, &stack);
                0
            }
        };
        let block = self.b.create_block();
        let bk = self.block_kinds(start, &kinds);
        self.widen(&kinds, &bk);
        self.blocks.insert(start, (block, bk, Vec::new()));
        self.b.ins().jump(block, NO);
        self.work.push(start);
        while let Some(ip) = self.work.pop() {
            self.walk_block(ip)?;
        }
        Ok(())
    }

    /// 64 raw bits as a value of kind `k`.
    fn bits_to_value(&mut self, raw: V, k: K) -> V {
        match k {
            K::Dbl => self.b.ins().bitcast(types::F64, MemFlagsData::new(), raw),
            K::Bool => self.b.ins().icmp_imm_s(IntCC::NotEqual, raw, 0),
            K::Int => raw,
            _ => raw,
        }
    }

    /// A value of kind `k` as 64 raw bits.
    fn value_to_bits(&mut self, v: V, k: K) -> V {
        match k {
            K::Dbl => self.b.ins().bitcast(types::I64, MemFlagsData::new(), v),
            K::Bool => self.b.ins().uextend(types::I64, v),
            K::Int | K::I64 | K::U64 => v,
            _ => self.b.ins().iconst(types::I64, 0),
        }
    }

    /// Leaves for the interpreter here: the frame written out (record and
    /// values), then back to the caller. Ends the current block.
    fn exit_now(&mut self, ip: usize, kinds: &[K], stack: &[Entry], why: Why) {
        if why.unclean() && self.in_region(ip) {
            self.out.unclean.push((ip, why));
        }
        let entry = self.is_entry();
        let mut locals = Vec::new();
        let mut vals = Vec::new();
        for (s, &k) in kinds.iter().enumerate() {
            if !k.scalar() && k != K::Dyn {
                continue;
            }
            // (an entry form writes back only what it took: the locals it
            // uses; a callee's frame is made whole)
            if entry && self.used_hint.is_some_and(|u| !u.contains(&(s as u16))) {
                continue;
            }
            if entry && self.used_hint.is_none() && !self.out.used_locals.contains(&(s as u16)) {
                continue;
            }
            locals.push((s as u16, k));
            if k == K::Dyn {
                // (its bits, then its kind's code: written back as it is)
                let (bv, tv) = self.dyn_vars(s as u32);
                let bits = self.b.use_var(bv);
                let tag = self.b.use_var(tv);
                vals.push(bits);
                vals.push(tag);
            } else if k != K::Null {
                let var = self.var(s as u32, k);
                let v = self.b.use_var(var);
                vals.push(self.value_to_bits(v, k));
            }
        }
        let mut arrays = Vec::new();
        for (d, &(v, k)) in stack.iter().enumerate() {
            if k == K::Arr {
                arrays.push((d as u16, self.arr_vals.get(&v).map_or(u32::MAX, |a| a.0)));
            } else if k.words() == 1 {
                vals.push(self.value_to_bits(v, k));
            }
        }
        let what = if why.unclean() { what(self.m, self.f, ip) } else { String::new() };
        let line = self.f.get_line_for_ip(ip).unwrap_or(0);
        let rec = ExitRecord { fn_index: self.fi, ip: ip as u32, locals, stack: stack.iter().map(|e| e.1).collect(), arrays, why, form: 0, line, what };
        let id = self.record_base + self.out.records.len();
        self.out.records.push(rec);
        let n = vals.len();
        // (the form's one buffer, room for every local twice — a `Dyn` is
        // two words — and a deep stack; an exit with more, a slot of its own)
        let cap = 2 * self.f.n_locals as usize + 64;
        let slot = match self.exit_slot {
            Some((slot, room)) if n <= room => slot,
            _ if n <= cap => {
                let slot = self.b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, (8 * cap) as u32, 3));
                self.exit_slot = Some((slot, cap));
                slot
            }
            _ => self.b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, (8 * n.max(1)) as u32, 3)),
        };
        let addr = self.b.ins().stack_addr(types::I64, slot, 0);
        for (i, v) in vals.into_iter().enumerate() {
            self.b.ins().store(MemFlagsData::trusted(), v, addr, (i * 8) as i32);
        }
        let fref = self.fref(self.helpers.exit);
        let ctx = self.ctx();
        if entry {
            let var = self.turns_var();
            let turns = self.b.use_var(var);
            self.b.ins().store(MemFlagsData::trusted(), turns, ctx, CTX_TURNS);
        }
        let idv = self.b.ins().iconst(types::I32, id as i64);
        let nv = self.b.ins().iconst(types::I32, n as i64);
        self.b.ins().call(fref, &[ctx, idv, addr, nv]);
        let one = self.b.ins().iconst(types::I32, 1);
        if entry {
            self.b.ins().return_(&[one]);
        } else {
            let zero = self.b.ins().iconst(types::I64, 0);
            self.b.ins().return_(&[one, zero]);
        }
    }

    /// Leaves for the interpreter when `cond` is true; goes on here
    /// otherwise.
    fn cond_exit(&mut self, cond: V, ip: usize, kinds: &[K], stack: &[Entry], why: Why) {
        let exit = self.b.create_block();
        let cont = self.b.create_block();
        self.b.set_cold_block(exit);
        self.b.ins().brif(cond, exit, NO, cont, NO);
        self.b.switch_to_block(exit);
        self.exit_now(ip, kinds, stack, why);
        self.b.switch_to_block(cont);
    }

    /// The debugger's interrupt raised: leave at `ip`.
    fn interrupt_check(&mut self, ip: usize, kinds: &[K], stack: &[Entry]) {
        let ctx = self.ctx();
        let p = self.b.ins().load(types::I64, MemFlagsData::trusted(), ctx, CTX_INTERRUPT);
        let flag = self.b.ins().load(types::I8, MemFlagsData::trusted(), p, 0);
        self.cond_exit(flag, ip, kinds, stack, Why::Interrupt);
    }

    /// Goes on at `target` with these kinds: a jump to its block (made the
    /// first time), or — out of the region, or kinds that disagree with
    /// the block's — an exit. Ends the current block.
    fn edge(&mut self, target: usize, at: usize, kinds: &[K], stack: &[Entry]) {
        if target <= at {
            self.interrupt_check(target, kinds, stack);
            // (a turn made, counted in a register; an entry form's exit
            // hands the count over: `Ctx::turns`)
            let var = self.turns_var();
            let n = self.b.use_var(var);
            let n = self.b.ins().iadd_imm_s(n, 1);
            self.b.def_var(var, n);
        }
        if !self.in_region(target) {
            return self.exit_now(target, kinds, stack, Why::LoopExit);
        }
        // (a global array on the stack doesn't go into another block: the
        // interpreter's from there)
        if stack.iter().any(|e| !e.1.scalar()) {
            return self.exit_now(target, kinds, stack, Why::Planned);
        }
        if target >= self.f.code.len() {
            // (the code's end: the routine returns)
            return self.ret(target, kinds, stack, None);
        }
        let skinds: Vec<K> = stack.iter().map(|e| e.1).collect();
        let (block, bk) = match self.blocks.get(&target) {
            Some((blk, k, s)) if *s == skinds && fits(kinds, k) => (*blk, k.clone()),
            Some((_, k, s)) => {
                // (kinds that meet: the block is built for both in the next
                // walk; this path leaves meanwhile)
                if *s == skinds {
                    if let Some(met) = meet_all(k, kinds) {
                        let met = match self.joins.get(&target) {
                            Some(h) if h.len() == met.len() => meet_all(h, &met).unwrap_or(met),
                            _ => met,
                        };
                        self.joins.insert(target, met);
                        self.rejoin = true;
                    }
                }
                return self.exit_now(target, kinds, stack, Why::Conflict);
            }
            None => {
                let blk = self.b.create_block();
                let bk = self.block_kinds(target, kinds);
                self.blocks.insert(target, (blk, bk.clone(), skinds));
                self.work.push(target);
                (blk, bk)
            }
        };
        self.widen(kinds, &bk);
        for (d, &(v, k)) in stack.iter().enumerate() {
            if k != K::Null {
                let var = self.var(STACK_VARS + d as u32, k);
                self.b.def_var(var, v);
            }
        }
        self.b.ins().jump(block, NO);
    }

    /// Two ways on: `then_ip` when `cond`, else `else_ip`.
    fn branch(&mut self, cond: V, then_ip: usize, else_ip: usize, at: usize, kinds: &[K], stack: &[Entry]) {
        let t = self.b.create_block();
        let e = self.b.create_block();
        self.b.ins().brif(cond, t, NO, e, NO);
        self.b.switch_to_block(t);
        self.edge(then_ip, at, kinds, stack);
        self.b.switch_to_block(e);
        self.edge(else_ip, at, kinds, stack);
    }

    /// A return: an entry form leaves (the interpreter returns); a callee
    /// returns to its caller. `value`: RetVal's.
    fn ret(&mut self, at: usize, kinds: &[K], stack: &[Entry], value: Option<Entry>) {
        if self.is_entry() {
            let mut st = stack.to_vec();
            if let Some(v) = value {
                st.push(v);
            }
            return self.exit_now(at, kinds, &st, Why::Return);
        }
        let (bits, k) = match value {
            Some((v, k)) => (self.value_to_bits(v, k), k),
            None => (self.b.ins().iconst(types::I64, 0), K::Null),
        };
        self.out.rets.insert(k);
        // (its callers were compiled for one kind of result: another
        // leaves, the interpreter returns it)
        let spec = self.callee_spec();
        let expected = self.env.callees.get(&spec).and_then(|c| c.ret);
        if expected.is_some_and(|e| e != k) {
            let mut st = stack.to_vec();
            if let Some(v) = value {
                st.push(v);
            }
            return self.exit_now(at, kinds, &st, Why::Guard);
        }
        let zero = self.b.ins().iconst(types::I32, 0);
        self.b.ins().return_(&[zero, bits]);
    }

    fn callee_spec(&self) -> (u32, Vec<K>) {
        match &self.kind {
            FormKind::Callee { params } => (self.fi, params.clone()),
            FormKind::Entry { .. } => (self.fi, Vec::new()),
        }
    }

    fn use_local(&mut self, s: u16) {
        self.out.used_locals.insert(s);
    }

    fn load_local(&mut self, s: u16, k: K) -> V {
        self.use_local(s);
        if k == K::Null {
            return self.dummy();
        }
        let var = self.var(u32::from(s), k);
        self.b.use_var(var)
    }

    fn store_local(&mut self, kinds: &mut [K], s: u16, (v, k): Entry) {
        self.use_local(s);
        if k != K::Null {
            let var = self.var(u32::from(s), k);
            self.b.def_var(var, v);
        }
        kinds[s as usize] = k;
    }

    fn global_ptr(&mut self) -> V {
        let ctx = self.ctx();
        self.b.ins().load(types::I64, MemFlagsData::trusted(), ctx, CTX_GLOBALS)
    }

    fn load_global(&mut self, name: u32) -> Option<Entry> {
        let (i, k) = self.env.global(name, self.obs);
        if !k.scalar() {
            return None;
        }
        self.out.globals.insert(i);
        if k == K::Null {
            return Some((self.dummy(), K::Null));
        }
        let p = self.global_ptr();
        let raw = self.b.ins().load(types::I64, MemFlagsData::trusted(), p, (i * 8) as i32);
        Some((self.bits_to_value(raw, k), k))
    }

    /// Stores into global `name` a value of its kind (`false`: another
    /// kind — the instruction is left to the interpreter).
    fn store_global(&mut self, name: u32, (v, k): Entry) -> bool {
        let (i, gk) = self.env.global(name, self.obs);
        if gk != k || !k.scalar() {
            return false;
        }
        self.out.globals.insert(i);
        if k != K::Null {
            let bits = self.value_to_bits(v, k);
            let p = self.global_ptr();
            self.b.ins().store(MemFlagsData::trusted(), bits, p, (i * 8) as i32);
        }
        true
    }

    /// A fused instruction's operand (`Mode`): a local, a global or a
    /// constant (`None`: not a value compiled code holds).
    fn leaf(&mut self, mode: Mode, operand: i64, kinds: &[K]) -> Option<Entry> {
        match mode {
            Mode::L => {
                let s = operand as u16;
                let k = kinds[s as usize];
                if !k.scalar() {
                    return None;
                }
                Some((self.load_local(s, k), k))
            }
            Mode::G => self.load_global(operand as u32),
            Mode::K => self.constant(operand as u32),
            Mode::S => None,
        }
    }

    fn constant(&mut self, k: u32) -> Option<Entry> {
        Some(match self.m.consts.get(k as usize)? {
            Const::Int(n) => (self.b.ins().iconst(types::I64, *n), K::Int),
            Const::Double(f) => (self.b.ins().f64const(*f), K::Dbl),
            Const::Bool(b) => (self.b.ins().iconst(types::I8, i64::from(*b)), K::Bool),
            Const::Null => (self.dummy(), K::Null),
            Const::Int64(n) => (self.b.ins().iconst(types::I64, *n), K::I64),
            Const::UInt64(n) => (self.b.ins().iconst(types::I64, *n as i64), K::U64),
            Const::Str(_) => return None,
        })
    }

    fn as_f64(&mut self, (v, k): Entry) -> V {
        match k {
            K::Dbl => v,
            K::U64 => self.b.ins().fcvt_from_uint(types::F64, v),
            // (`Value::to_f64`: a true -1, a false and Null 0)
            K::Bool => {
                let t = self.b.ins().f64const(-1.0);
                let f = self.b.ins().f64const(0.0);
                self.b.ins().select(v, t, f)
            }
            K::Null => self.b.ins().f64const(0.0),
            _ => self.b.ins().fcvt_from_sint(types::F64, v),
        }
    }

    /// The 32-bit integer AND / OR / XOR / NOT work on (`Value::bits`,
    /// RapidQ's FISTP): a true -1, an Integer as `int32_of`, a Double
    /// rounded half to even (a NaN or beyond 32 bits -2147483648), Null 0.
    fn basic_bits(&mut self, (v, k): Entry) -> V {
        match k {
            K::Int => self.int32_of(v),
            K::Bool => {
                let n = self.b.ins().uextend(types::I64, v);
                self.b.ins().ineg(n)
            }
            K::Dbl => {
                let r = self.b.ins().nearest(v);
                let lo = self.b.ins().f64const(-2147483648.0);
                let hi = self.b.ins().f64const(2147483647.0);
                let above = self.b.ins().fcmp(FloatCC::GreaterThanOrEqual, r, lo);
                let below = self.b.ins().fcmp(FloatCC::LessThanOrEqual, r, hi);
                let inside = self.b.ins().band(above, below);
                let n = self.b.ins().fcvt_to_sint_sat(types::I64, r);
                let min = self.b.ins().iconst(types::I64, i64::from(i32::MIN));
                self.b.ins().select(inside, n, min)
            }
            _ => self.b.ins().iconst(types::I64, 0),
        }
    }

    /// `c ? a : b` for two values of one kind (IIF's arm).
    fn pick(&mut self, c: V, a: Entry, b: Entry) -> Entry {
        if a.1 == K::Null {
            return (self.dummy(), K::Null);
        }
        (self.b.ins().select(c, a.0, b.0), a.1)
    }

    /// `rapidr_value::format::int32_of`: beyond 32 bits, -2147483648.
    fn int32_of(&mut self, n: V) -> V {
        let low = self.b.ins().ireduce(types::I32, n);
        let back = self.b.ins().sextend(types::I64, low);
        let same = self.b.ins().icmp(IntCC::Equal, back, n);
        let min = self.b.ins().iconst(types::I64, i64::from(i32::MIN));
        self.b.ins().select(same, n, min)
    }

    /// `a op b` for Add, Sub, Mul, Div (`fast::add` & co.): `None` for
    /// kinds it leaves to the interpreter.
    fn arith(&mut self, bin: Bin, a: Entry, b: Entry) -> Option<Entry> {
        let num = |k: K| matches!(k, K::Int | K::Dbl | K::I64 | K::U64);
        if !num(a.1) || !num(b.1) {
            // (a boolean or Null with a number that isn't 64-bit — `n * (a
            // < b)`, BASIC's way to count a comparison —: `Value`'s own
            // operation, on floats, a true -1: a Double)
            let plain = |k: K| matches!(k, K::Int | K::Dbl | K::Bool | K::Null);
            if !plain(a.1) || !plain(b.1) {
                return None;
            }
            let (x, y) = (self.as_f64(a), self.as_f64(b));
            let r = match bin {
                Bin::Add => self.b.ins().fadd(x, y),
                Bin::Sub => self.b.ins().fsub(x, y),
                Bin::Mul => self.b.ins().fmul(x, y),
                Bin::Div => self.b.ins().fdiv(x, y),
                _ => return None,
            };
            return Some((r, K::Dbl));
        }
        // (an INT64 / UINT64 with a whole number: 64-bit, wrapping; a UINT64
        // on either side makes one — rapidr_value::wide::arith)
        if (a.1.wide() || b.1.wide()) && a.1.whole() && b.1.whole() && bin != Bin::Div {
            let r = match bin {
                Bin::Add => self.b.ins().iadd(a.0, b.0),
                Bin::Sub => self.b.ins().isub(a.0, b.0),
                Bin::Mul => self.b.ins().imul(a.0, b.0),
                _ => return None,
            };
            return Some((r, if a.1 == K::U64 || b.1 == K::U64 { K::U64 } else { K::I64 }));
        }
        if bin == Bin::Div {
            let (x, y) = (self.as_f64(a), self.as_f64(b));
            return Some((self.b.ins().fdiv(x, y), K::Dbl));
        }
        if a.1 == K::Int && b.1 == K::Int {
            let r = match bin {
                Bin::Add => self.b.ins().iadd(a.0, b.0),
                Bin::Sub => self.b.ins().isub(a.0, b.0),
                Bin::Mul => self.b.ins().imul(a.0, b.0),
                _ => return None,
            };
            return Some((r, K::Int));
        }
        let (x, y) = (self.as_f64(a), self.as_f64(b));
        let r = match bin {
            Bin::Add => self.b.ins().fadd(x, y),
            Bin::Sub => self.b.ins().fsub(x, y),
            Bin::Mul => self.b.ins().fmul(x, y),
            _ => return None,
        };
        Some((r, K::Dbl))
    }

    /// The comparisons as `fast::eq` & co. make them: two numbers as
    /// floats (two Integers a Double holds exactly compare the same as
    /// integers, and beyond 2^53 the interpreter compares their floats
    /// too), a NaN on either side "less" and "equal" at once.
    fn compare(&mut self, cmp: Cmp, a: Entry, b: Entry) -> Option<V> {
        if a.1.wide() || b.1.wide() {
            return self.compare_wide(cmp, a, b);
        }
        if !matches!(a.1, K::Int | K::Dbl) || !matches!(b.1, K::Int | K::Dbl) {
            return None;
        }
        let (x, y) = (self.as_f64(a), self.as_f64(b));
        let cc = match cmp {
            Cmp::Eq => FloatCC::UnorderedOrEqual,
            Cmp::Ne => FloatCC::OrderedNotEqual,
            Cmp::Lt => FloatCC::UnorderedOrLessThan,
            Cmp::Le => FloatCC::UnorderedOrLessThanOrEqual,
            Cmp::Gt => FloatCC::GreaterThan,
            Cmp::Ge => FloatCC::GreaterThanOrEqual,
        };
        Some(self.b.ins().fcmp(cc, x, y))
    }

    /// A comparison with an INT64 / UINT64 side and a whole number on the
    /// other, exact (`rapidr_value::wide::compare`): signed, unsigned, or —
    /// one of each — a negative signed side is below any UINT64. Against a
    /// DOUBLE: the interpreter's (`None`).
    fn compare_wide(&mut self, cmp: Cmp, a: Entry, b: Entry) -> Option<V> {
        if !a.1.whole() || !b.1.whole() {
            return None;
        }
        let (ua, ub) = (a.1 == K::U64, b.1 == K::U64);
        let cc = |unsigned: bool| match (cmp, unsigned) {
            (Cmp::Eq, _) => IntCC::Equal,
            (Cmp::Ne, _) => IntCC::NotEqual,
            (Cmp::Lt, false) => IntCC::SignedLessThan,
            (Cmp::Le, false) => IntCC::SignedLessThanOrEqual,
            (Cmp::Gt, false) => IntCC::SignedGreaterThan,
            (Cmp::Ge, false) => IntCC::SignedGreaterThanOrEqual,
            (Cmp::Lt, true) => IntCC::UnsignedLessThan,
            (Cmp::Le, true) => IntCC::UnsignedLessThanOrEqual,
            (Cmp::Gt, true) => IntCC::UnsignedGreaterThan,
            (Cmp::Ge, true) => IntCC::UnsignedGreaterThanOrEqual,
        };
        if ua == ub {
            return Some(self.b.ins().icmp(cc(ua), a.0, b.0));
        }
        // (one signed, one unsigned: a negative signed side is the smaller)
        let signed = if ua { b.0 } else { a.0 };
        let negative = self.b.ins().icmp_imm_s(IntCC::SignedLessThan, signed, 0);
        let unsigned = self.b.ins().icmp(cc(true), a.0, b.0);
        let a_smaller = !ua;
        let when_negative = match cmp {
            Cmp::Eq => false,
            Cmp::Ne => true,
            Cmp::Lt | Cmp::Le => a_smaller,
            Cmp::Gt | Cmp::Ge => !a_smaller,
        };
        let fixed = self.b.ins().iconst(types::I8, i64::from(when_negative));
        Some(self.b.ins().select(negative, fixed, unsigned))
    }

    /// `Value::to_bool`: whether a jump on it is taken.
    fn truth(&mut self, (v, k): Entry) -> Option<V> {
        Some(match k {
            K::Bool => v,
            K::Int | K::I64 | K::U64 => self.b.ins().icmp_imm_s(IntCC::NotEqual, v, 0),
            K::Dbl => {
                let z = self.b.ins().f64const(0.0);
                self.b.ins().fcmp(FloatCC::NotEqual, v, z)
            }
            K::Null => self.b.ins().iconst(types::I8, 0),
            _ => return None,
        })
    }

    /// `Value::to_i64` (the bitwise operators' operand, an index).
    fn as_i64(&mut self, (v, k): Entry) -> Option<V> {
        Some(match k {
            // (`to_i64` of an INT64 its number, of a UINT64 its bits)
            K::Int | K::I64 | K::U64 => v,
            K::Dbl => self.b.ins().fcvt_to_sint_sat(types::I64, v),
            K::Bool => {
                let n = self.b.ins().uextend(types::I64, v);
                self.b.ins().ineg(n)
            }
            K::Null => self.b.ins().iconst(types::I64, 0),
            _ => return None,
        })
    }

    /// `ToNum code` on a value (`fast::to_num`): the common cases written
    /// out, the rest through the helper — the interpreter's own function.
    fn convert(&mut self, (v, k): Entry, code: u8) -> Option<Entry> {
        if !k.scalar() {
            return None;
        }
        if code == ARG_ROUND {
            return Some(match k {
                K::Dbl => (self.b.ins().nearest(v), K::Dbl),
                _ => (v, k),
            });
        }
        let kind = NumKind::from_code(code)?;
        // (a store into an INT64 / UINT32 makes an INT64, into a UINT64 a
        // UINT64: rapidr_value::wide's to_int64 & co.)
        let out = match kind {
            NumKind::UInt64 => K::U64,
            k if k.is_wide() => K::I64,
            k if k.is_integer() => K::Int,
            _ => K::Dbl,
        };
        match (kind, k) {
            (NumKind::Int64, K::Int | K::I64 | K::U64) => return Some((v, K::I64)),
            (NumKind::UInt64, K::Int | K::I64 | K::U64) => return Some((v, K::U64)),
            (NumKind::UInt32, K::Int | K::I64 | K::U64) => return Some((self.b.ins().band_imm_u(v, 0xFFFF_FFFF), K::I64)),
            (NumKind::Double, K::Dbl) => return Some((v, K::Dbl)),
            (NumKind::Double, K::Int | K::I64 | K::U64) => return Some((self.as_f64((v, k)), K::Dbl)),
            (NumKind::Single, K::Dbl | K::Int | K::I64 | K::U64) => {
                let f = self.as_f64((v, k));
                let s = self.b.ins().fdemote(types::F32, f);
                return Some((self.b.ins().fpromote(types::F64, s), K::Dbl));
            }
            // (an INT64 into a classic type: as an Integer of its number;
            // a UINT64 through the helper — its number may be past i64)
            (NumKind::Long | NumKind::Dword, K::Int | K::I64) => return Some((self.int32_of(v), K::Int)),
            (NumKind::Byte, K::Int | K::I64) => {
                let n = self.int32_of(v);
                return Some((self.b.ins().band_imm_u(n, 0xFF), K::Int));
            }
            (NumKind::Word, K::Int | K::I64) => {
                let n = self.int32_of(v);
                return Some((self.b.ins().band_imm_u(n, 0xFFFF), K::Int));
            }
            (NumKind::Short, K::Int | K::I64) => {
                let n = self.int32_of(v);
                let s = self.b.ins().ireduce(types::I16, n);
                return Some((self.b.ins().sextend(types::I64, s), K::Int));
            }
            _ => {}
        }
        let bits = self.value_to_bits(v, k);
        let fref = self.fref(self.helpers.conv);
        let c = self.b.ins().iconst(types::I32, i64::from(code));
        let kc = self.b.ins().iconst(types::I32, i64::from(k.code()));
        let call = self.b.ins().call(fref, &[c, kc, bits]);
        let raw = self.b.inst_results(call)[0];
        Some((self.bits_to_value(raw, out), out))
    }

    /// A pure maths builtin of one argument, or SHL / SHR of two
    /// (`fast::call_direct`): the interpreter's own function, through the
    /// helper; its result's kind is checked against the one compiled for.
    fn math(&mut self, code: u8, args: &[Entry], at: usize, kinds: &[K], pre: &[Entry]) -> Option<Entry> {
        if args.iter().any(|a| !a.1.scalar()) || args.is_empty() || args.len() > 2 {
            return None;
        }
        // (the kind it gives for these kinds of arguments, from samples)
        let samples: Vec<Value> = args
            .iter()
            .map(|a| match a.1 {
                K::Int => Value::Integer(3),
                K::I64 => Value::Int64(3),
                K::U64 => Value::UInt64(3),
                K::Dbl => Value::Double(2.5),
                K::Bool => Value::Boolean(true),
                _ => Value::Null,
            })
            .collect();
        let expect = K::of(&rapidr_vm::fast::call_direct(code, &samples));
        if !expect.scalar() {
            return None;
        }
        let arg = args[0];
        if args.len() == 2 {
            let b = args[1];
            let (x, y) = (self.value_to_bits(arg.0, arg.1), self.value_to_bits(b.0, b.1));
            let slot = self.b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
            let addr = self.b.ins().stack_addr(types::I64, slot, 0);
            let fref = self.fref(self.helpers.math2);
            let c = self.b.ins().iconst(types::I32, i64::from(code));
            let ka = self.b.ins().iconst(types::I32, i64::from(arg.1.code()));
            let kb = self.b.ins().iconst(types::I32, i64::from(b.1.code()));
            let call = self.b.ins().call(fref, &[c, ka, x, kb, y, addr]);
            let got = self.b.inst_results(call)[0];
            let wrong = self.b.ins().icmp_imm_s(IntCC::NotEqual, got, i64::from(expect.code()));
            self.cond_exit(wrong, at, kinds, pre, Why::Guard);
            let raw = self.b.ins().load(types::I64, MemFlagsData::trusted(), addr, 0);
            return Some((self.bits_to_value(raw, expect), expect));
        }
        // (ABS of a number: written out)
        if code == rapidr_vm::fast::direct_code("ABS") {
            match arg.1 {
                K::Dbl => return Some((self.b.ins().fabs(arg.0), K::Dbl)),
                K::Int | K::I64 => {
                    let neg = self.b.ins().ineg(arg.0);
                    let lt = self.b.ins().icmp_imm_s(IntCC::SignedLessThan, arg.0, 0);
                    return Some((self.b.ins().select(lt, neg, arg.0), arg.1));
                }
                K::U64 => return Some(arg),
                _ => {}
            }
        }
        let bits = self.value_to_bits(arg.0, arg.1);
        let slot = self.b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
        let addr = self.b.ins().stack_addr(types::I64, slot, 0);
        let fref = self.fref(self.helpers.math);
        let c = self.b.ins().iconst(types::I32, i64::from(code));
        let kc = self.b.ins().iconst(types::I32, i64::from(arg.1.code()));
        let call = self.b.ins().call(fref, &[c, kc, bits, addr]);
        let got = self.b.inst_results(call)[0];
        let wrong = self.b.ins().icmp_imm_s(IntCC::NotEqual, got, i64::from(expect.code()));
        self.cond_exit(wrong, at, kinds, pre, Why::Guard);
        let raw = self.b.ins().load(types::I64, MemFlagsData::trusted(), addr, 0);
        Some((self.bits_to_value(raw, expect), expect))
    }

    /// An array's handle: a global array, or one of the entry frame's
    /// locals (`None`: not an array compiled code indexes).
    fn array(&mut self, mode: Mode, operand: i64, kinds: &[K]) -> Option<(i64, usize, K)> {
        match mode {
            Mode::G => {
                let (h, elem) = self.env.garray(operand as u32, self.obs)?;
                self.out.garrays.insert(h);
                Some((TABLE_GLOBAL, h, elem))
            }
            Mode::L => {
                let s = operand as u16;
                if kinds[s as usize] != K::Arr || !self.is_entry() {
                    return None;
                }
                if let Some(&h) = self.larray_index.get(&s) {
                    return Some((TABLE_LOCAL, h, self.out.larrays[h].1));
                }
                let Some(Value::Array(a)) = self.obs.locals.and_then(|l| l.get(s as usize)) else { return None };
                let elem = element_kind(&a.borrow())?;
                self.out.larrays.push((s, elem));
                let h = self.out.larrays.len() - 1;
                self.larray_index.insert(s, h);
                self.use_local(s);
                Some((TABLE_LOCAL, h, elem))
            }
            _ => None,
        }
    }

    /// `arr(i)` through the helper; leaves when it says no (out of range,
    /// an element of another kind: the interpreter answers, or raises the
    /// error).
    fn aget(&mut self, arr: (i64, usize, K), index: V, at: usize, kinds: &[K], pre: &[Entry]) -> Entry {
        let (table, h, elem) = arr;
        let slot = self.b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
        let addr = self.b.ins().stack_addr(types::I64, slot, 0);
        let fref = self.fref(self.helpers.aget);
        let ctx = self.ctx();
        let t = self.b.ins().iconst(types::I32, table);
        let hv = self.b.ins().iconst(types::I32, h as i64);
        let ek = self.b.ins().iconst(types::I32, i64::from(elem.code()));
        let call = self.b.ins().call(fref, &[ctx, t, hv, index, ek, addr]);
        let status = self.b.inst_results(call)[0];
        self.cond_exit(status, at, kinds, pre, Why::Guard);
        if elem == K::Null {
            return (self.dummy(), K::Null);
        }
        let raw = self.b.ins().load(types::I64, MemFlagsData::trusted(), addr, 0);
        (self.bits_to_value(raw, elem), elem)
    }

    fn aset(&mut self, arr: (i64, usize, K), index: V, value: Entry, at: usize, kinds: &[K], pre: &[Entry]) {
        let (table, h, _) = arr;
        let bits = self.value_to_bits(value.0, value.1);
        let fref = self.fref(self.helpers.aset);
        let ctx = self.ctx();
        let t = self.b.ins().iconst(types::I32, table);
        let hv = self.b.ins().iconst(types::I32, h as i64);
        let kc = self.b.ins().iconst(types::I32, i64::from(value.1.code()));
        let call = self.b.ins().call(fref, &[ctx, t, hv, index, kc, bits]);
        let status = self.b.inst_results(call)[0];
        self.cond_exit(status, at, kinds, pre, Why::Guard);
    }

    /// A call of function `fi` with the top `argc` entries: a machine call
    /// of its callee form. `Some(why)`: left to the interpreter (nothing
    /// emitted: the caller exits here).
    #[allow(clippy::too_many_arguments)]
    fn call(&mut self, at: usize, next: usize, fi: u32, argc: usize, wants_value: bool, kinds: &[K], stack: &mut Vec<Entry>) -> Option<Why> {
        // (a BYREF write-back after it reads the callee's parameters as the
        // interpreter left them: the interpreter makes the call)
        if self.f.code.get(next) == Some(&(Op::LoadArgOut as u8)) || stack.len() < argc {
            return Some(Why::Planned);
        }
        let args = stack[stack.len() - argc..].to_vec();
        if args.iter().any(|a| !a.1.scalar()) {
            return Some(Why::Planned);
        }
        let callee = &self.m.functions[fi as usize];
        let n_params = callee.params.len().min(callee.n_locals as usize);
        let params: Vec<K> = (0..n_params).map(|i| args.get(i).map_or(K::Null, |a| a.1)).collect();
        let spec = (fi, params.clone());
        if !self.env.callees.contains_key(&spec) {
            let name = self.env.next_name();
            let Ok(func) = self.jm.declare_function(&name, Linkage::Local, &callee_signature(&params)) else { return Some(Why::Planned) };
            self.env.callees.insert(spec.clone(), CalleeInfo { func, ret: None, callable: Callable::Unknown, globals: BTreeSet::new(), garrays: BTreeSet::new(), defined: false, calls: BTreeSet::new() });
        }
        self.out.calls.insert(spec.clone());
        let info = self.env.callees[&spec].clone();
        if info.callable == Callable::No {
            return Some(Why::Planned);
        }
        let Some(ret) = info.ret else {
            self.out.pending = true;
            return Some(Why::Pending);
        };
        self.out.globals.extend(info.globals.iter().copied());
        self.out.garrays.extend(info.garrays.iter().copied());
        // (the machine stack's room, in bytes — SEC-50 —: too little left
        // for the callee's frame, or as many frames as the VM may have, and
        // the interpreter makes the call, and the stack overflow error at
        // the depth it always has. One compare: `Ctx::stack_limit` folds
        // both in)
        let ctx = self.ctx();
        let sp = self.b.ins().get_stack_pointer(types::I64);
        let floor = self.b.ins().load(types::I64, MemFlagsData::trusted(), ctx, CTX_STACK_LIMIT);
        let low = self.b.ins().icmp(IntCC::UnsignedLessThan, sp, floor);
        self.cond_exit(low, at, kinds, stack, Why::Depth);
        let mut argv = vec![ctx];
        for (i, a) in args.iter().enumerate().take(n_params) {
            if params[i] != K::Null {
                argv.push(a.0);
            }
        }
        let fref = self.fref(info.func);
        let inst = self.b.ins().call(fref, &argv);
        let res = self.b.inst_results(inst).to_vec();
        let below: Vec<Entry> = stack[..stack.len() - argc].to_vec();
        self.cond_exit(res[0], next, kinds, &below, Why::AfterCall { wants_value });
        stack.truncate(stack.len() - argc);
        if wants_value {
            let v = if ret == K::Null { self.dummy() } else { self.bits_to_value(res[1], ret) };
            stack.push((v, ret));
        }
        None
    }

    /// Runs the instructions from `start` (a block's first) to the end of
    /// its block, emitting their code.
    fn walk_block(&mut self, start: usize) -> Result<(), Unbuilt> {
        let (block, mut kinds, skinds) = self.blocks.get(&start).cloned().expect("a block");
        self.b.switch_to_block(block);
        let mut stack: Vec<Entry> = Vec::with_capacity(skinds.len());
        for (d, &k) in skinds.iter().enumerate() {
            let v = if k == K::Null {
                self.dummy()
            } else {
                let var = self.var(STACK_VARS + d as u32, k);
                self.b.use_var(var)
            };
            stack.push((v, k));
        }
        let (f, m) = (self.f, self.m);
        let code = &f.code;
        let mut ip = start;
        loop {
            // (falling into another block's first instruction)
            if ip != start && self.leaders.get(ip).copied().unwrap_or(false) {
                self.edge(ip, ip - 1, &kinds, &stack);
                return Ok(());
            }
            if ip >= code.len() {
                self.ret(ip, &kinds, &stack, None);
                return Ok(());
            }
            let insn = decode(code, ip).ok_or_else(|| Unbuilt(format!("undecodable at {ip}")))?;
            let at = ip;
            let mut next = insn.next;
            let o = &insn.operands;
            let pre = stack.clone();
            // (an instruction compiled code doesn't run: the interpreter's,
            // from here)
            // (an instruction compiled code doesn't run, or one that reads
            // a local whose kind differs on the ways that meet before it —
            // `K::Dyn`: the interpreter's, from here)
            macro_rules! bail {
                () => {{
                    let why = if reads_dyn(&insn, &kinds) { Why::Conflict } else { Why::Planned };
                    self.exit_now(at, &kinds, &pre, why);
                    return Ok(());
                }};
            }
            macro_rules! pop {
                () => {
                    match stack.pop() {
                        Some(e) => e,
                        None => bail!(),
                    }
                };
            }
            match insn.op {
                Op::Nop => {}
                Op::LoadConst => match self.constant(o[0] as u32) {
                    Some(e) => stack.push(e),
                    None => bail!(),
                },
                Op::LoadNull => {
                    let d = self.dummy();
                    stack.push((d, K::Null));
                }
                Op::LoadTrue | Op::LoadFalse => {
                    let v = self.b.ins().iconst(types::I8, i64::from(insn.op == Op::LoadTrue));
                    stack.push((v, K::Bool));
                }
                Op::Pop => {
                    pop!();
                }
                Op::Dup => {
                    let top = pop!();
                    stack.push(top);
                    stack.push(top);
                }
                Op::LoadLocal => match self.leaf(Mode::L, o[0], &kinds) {
                    Some(e) => stack.push(e),
                    None => bail!(),
                },
                Op::StoreLocal => {
                    let v = pop!();
                    if !v.1.scalar() {
                        bail!();
                    }
                    self.store_local(&mut kinds, o[0] as u16, v);
                }
                Op::LoadGlobal => match self.load_global(o[0] as u32) {
                    Some(e) => stack.push(e),
                    None => {
                        // (a global array, read by the `AGet 1` after its
                        // index — `memory(a + IIF(…))`: an index with a call
                        // in it is worked out after the array is read)
                        let Some(arr) = self.array(Mode::G, o[0], &kinds) else { bail!() };
                        let v = self.b.ins().iconst(types::I64, arr.1 as i64);
                        self.arr_vals.insert(v, (o[0] as u32, arr));
                        stack.push((v, K::Arr));
                    }
                },
                Op::AGet | Op::ASet if o[0] == 1 => {
                    let value = if insn.op == Op::ASet { Some(pop!()) } else { None };
                    let idx = pop!();
                    let arr = pop!();
                    let Some(&(_, handle)) = self.arr_vals.get(&arr.0).filter(|_| arr.1 == K::Arr) else { bail!() };
                    let Some(i) = self.as_i64(idx) else { bail!() };
                    match value {
                        // (the value as it is: ASet converts nothing)
                        Some(v) if v.1.scalar() => self.aset(handle, i, v, at, &kinds, &pre),
                        Some(_) => bail!(),
                        None => {
                            let v = self.aget(handle, i, at, &kinds, &pre);
                            stack.push(v);
                        }
                    }
                }
                Op::StoreGlobal => {
                    let v = pop!();
                    if !self.store_global(o[0] as u32, v) {
                        bail!();
                    }
                }
                Op::Add | Op::Sub | Op::Mul | Op::Div => {
                    let b = pop!();
                    let a = pop!();
                    let bin = match insn.op {
                        Op::Add => Bin::Add,
                        Op::Sub => Bin::Sub,
                        Op::Mul => Bin::Mul,
                        _ => Bin::Div,
                    };
                    match self.arith(bin, a, b) {
                        Some(r) => stack.push(r),
                        None => bail!(),
                    }
                }
                Op::Mod | Op::IDiv => {
                    let b = pop!();
                    let a = pop!();
                    match self.int_div(insn.op == Op::Mod, a, b, at, &kinds, &pre) {
                        Some(r) => stack.push(r),
                        None => bail!(),
                    }
                }
                Op::Pow => {
                    let b = pop!();
                    let a = pop!();
                    if !matches!(a.1, K::Int | K::Dbl) || !matches!(b.1, K::Int | K::Dbl) {
                        bail!();
                    }
                    let (x, y) = (self.as_f64(a), self.as_f64(b));
                    let fref = self.fref(self.helpers.powf);
                    let call = self.b.ins().call(fref, &[x, y]);
                    let r = self.b.inst_results(call)[0];
                    stack.push((r, K::Dbl));
                }
                Op::Neg => {
                    let (v, k) = pop!();
                    match k {
                        K::Int | K::I64 | K::U64 => {
                            let r = self.b.ins().ineg(v);
                            stack.push((r, k));
                        }
                        K::Dbl => {
                            let r = self.b.ins().fneg(v);
                            stack.push((r, K::Dbl));
                        }
                        // (`-&Value` of a boolean or Null: a Double, -0 for
                        // a false or Null)
                        K::Bool | K::Null => {
                            let x = self.as_f64((v, k));
                            let r = self.b.ins().fneg(x);
                            stack.push((r, K::Dbl));
                        }
                        _ => bail!(),
                    }
                }
                Op::Eq | Op::Ne | Op::Lt | Op::Le | Op::Gt | Op::Ge => {
                    let b = pop!();
                    let a = pop!();
                    let cmp = match insn.op {
                        Op::Eq => Cmp::Eq,
                        Op::Ne => Cmp::Ne,
                        Op::Lt => Cmp::Lt,
                        Op::Le => Cmp::Le,
                        Op::Gt => Cmp::Gt,
                        _ => Cmp::Ge,
                    };
                    match self.compare(cmp, a, b) {
                        Some(r) => stack.push((r, K::Bool)),
                        None => bail!(),
                    }
                }
                Op::And | Op::Or | Op::Xor => {
                    let b = pop!();
                    let a = pop!();
                    let r = match (a.1, b.1) {
                        (K::Bool, K::Bool) => (
                            match insn.op {
                                Op::And => self.b.ins().band(a.0, b.0),
                                Op::Or => self.b.ins().bor(a.0, b.0),
                                _ => self.b.ins().bxor(a.0, b.0),
                            },
                            K::Bool,
                        ),
                        // (an INT64 / UINT64 side: all 64 bits, wide::bitwise)
                        (x, y) if (x.wide() || y.wide()) && x.whole() && y.whole() => (
                            match insn.op {
                                Op::And => self.b.ins().band(a.0, b.0),
                                Op::Or => self.b.ins().bor(a.0, b.0),
                                _ => self.b.ins().bxor(a.0, b.0),
                            },
                            if x == K::U64 || y == K::U64 { K::U64 } else { K::I64 },
                        ),
                        // (two Integers — or a boolean, a Double, Null with
                        // one: `(pc - 8 * (c = 1)) AND 255` — as the 32-bit
                        // integers `Value::bits` makes of them)
                        (x, y) if basic(x) && basic(y) => {
                            let (x, y) = (self.basic_bits(a), self.basic_bits(b));
                            (
                                match insn.op {
                                    Op::And => self.b.ins().band(x, y),
                                    Op::Or => self.b.ins().bor(x, y),
                                    _ => self.b.ins().bxor(x, y),
                                },
                                K::Int,
                            )
                        }
                        _ => bail!(),
                    };
                    stack.push(r);
                }
                Op::Not => {
                    let (v, k) = pop!();
                    match k {
                        K::Bool => {
                            let r = self.b.ins().bxor_imm_u(v, 1);
                            stack.push((r, K::Bool));
                        }
                        K::Int => {
                            let n = self.int32_of(v);
                            let r = self.b.ins().bnot(n);
                            stack.push((r, K::Int));
                        }
                        K::I64 | K::U64 => {
                            let r = self.b.ins().bnot(v);
                            stack.push((r, k));
                        }
                        K::Dbl | K::Null => {
                            let n = self.basic_bits((v, k));
                            let r = self.b.ins().bnot(n);
                            stack.push((r, K::Int));
                        }
                        _ => bail!(),
                    }
                }
                Op::BAnd | Op::BOr | Op::BXor | Op::Shl | Op::Shr => {
                    let b = pop!();
                    let a = pop!();
                    let (Some(x), Some(y)) = (self.as_i64(a), self.as_i64(b)) else { bail!() };
                    let r = match insn.op {
                        Op::BAnd => self.b.ins().band(x, y),
                        Op::BOr => self.b.ins().bor(x, y),
                        Op::BXor => self.b.ins().bxor(x, y),
                        Op::Shl => self.b.ins().ishl(x, y),
                        _ => self.b.ins().sshr(x, y),
                    };
                    stack.push((r, K::Int));
                }
                Op::BNot => {
                    let a = pop!();
                    let Some(x) = self.as_i64(a) else { bail!() };
                    let r = self.b.ins().bnot(x);
                    stack.push((r, K::Int));
                }
                Op::Jump => {
                    self.edge(o[0] as usize, at, &kinds, &stack);
                    return Ok(());
                }
                Op::JumpIf | Op::JumpIfNot => {
                    let v = pop!();
                    let Some(c) = self.truth(v) else { bail!() };
                    let (t, fall) = (o[0] as usize, next);
                    if insn.op == Op::JumpIf {
                        self.branch(c, t, fall, at, &kinds, &stack);
                    } else {
                        self.branch(c, fall, t, at, &kinds, &stack);
                    }
                    return Ok(());
                }
                Op::CallSub | Op::CallFunc => {
                    if let Some(why) = self.call(at, next, o[0] as u32, o[1] as usize, insn.op == Op::CallFunc, &kinds, &mut stack) {
                        self.exit_now(at, &kinds, &pre, why);
                        return Ok(());
                    }
                }
                Op::Ret => {
                    self.ret(at, &kinds, &stack, None);
                    return Ok(());
                }
                Op::RetVal => {
                    let v = pop!();
                    if !v.1.scalar() {
                        bail!();
                    }
                    self.ret(at, &kinds, &stack, Some(v));
                    return Ok(());
                }
                Op::CallBuiltin => {
                    let name = m.strings.get(o[0] as usize).map(String::as_str).unwrap_or("");
                    // (IIF(c, a, b): both arms already worked out, as the
                    // interpreter has them — the one the condition picks,
                    // as it is: `rapidr_value::builtins::rp_iif`, which the
                    // hosts that let compiled code call the maths call too)
                    if self.env.direct_maths && o[1] == 3 && rapidr_bytecode::builtins::is_builtin(name) && rapidr_bytecode::builtins::builtin_key(name) == "iif" {
                        let no = pop!();
                        let yes = pop!();
                        let cond = pop!();
                        let Some(c) = self.truth(cond) else { bail!() };
                        if yes.1 == no.1 && yes.1.scalar() {
                            stack.push(self.pick(c, yes, no));
                        } else if yes.1.scalar() && no.1.scalar() && !self.leaders.get(next).copied().unwrap_or(true) {
                            // (arms of two kinds stored into a typed variable
                            // or converted, as `flagC = IIF(v > 255, 1, 0.5)`:
                            // each arm converted as the store converts, then
                            // the one picked — the very value the store makes
                            // of the arm the interpreter picks)
                            let Some(after) = decode(code, next) else { bail!() };
                            let conv = match after.op {
                                Op::StoreLocalNum | Op::StoreGlobalNum => after.operands[1] as u8,
                                Op::ToNum => after.operands[0] as u8,
                                _ => bail!(),
                            };
                            let (Some(y), Some(n)) = (self.convert(yes, conv), self.convert(no, conv)) else { bail!() };
                            if y.1 != n.1 {
                                bail!();
                            }
                            let r = self.pick(c, y, n);
                            match after.op {
                                Op::StoreLocalNum => self.store_local(&mut kinds, after.operands[0] as u16, r),
                                Op::StoreGlobalNum => {
                                    if !self.store_global(after.operands[0] as u32, r) {
                                        bail!();
                                    }
                                }
                                _ => stack.push(r),
                            }
                            next = after.next;
                        } else {
                            bail!();
                        }
                        ip = next;
                        continue;
                    }
                    let code = if self.env.direct_maths { rapidr_vm::fast::direct_code(name) } else { 0 };
                    // (one argument; SHL and SHR two; IIF's three: above)
                    let argc = rapidr_vm::fast::direct_arity(code);
                    if code == 0 || argc > 2 || o[1] as usize != argc {
                        bail!();
                    }
                    let b = pop!();
                    let args = if argc == 2 { vec![pop!(), b] } else { vec![b] };
                    match self.math(code, &args, at, &kinds, &pre) {
                        Some(r) => stack.push(r),
                        None => bail!(),
                    }
                }
                Op::ToNum => {
                    let v = pop!();
                    match self.convert(v, o[0] as u8) {
                        Some(r) => stack.push(r),
                        None => bail!(),
                    }
                }
                Op::StoreLocalNum => {
                    let v = pop!();
                    match self.convert(v, o[1] as u8) {
                        Some(r) => self.store_local(&mut kinds, o[0] as u16, r),
                        None => bail!(),
                    }
                }
                Op::StoreGlobalNum => {
                    let v = pop!();
                    match self.convert(v, o[1] as u8) {
                        Some(r) if self.store_global(o[0] as u32, r) => {}
                        _ => bail!(),
                    }
                }
                Op::ConvLocal | Op::ConvArgLocal => {
                    let s = o[0] as u16;
                    let Some(mut v) = self.leaf(Mode::L, o[0], &kinds) else { bail!() };
                    if insn.op == Op::ConvArgLocal {
                        match self.convert(v, ARG_ROUND) {
                            Some(r) => v = r,
                            None => bail!(),
                        }
                    }
                    match self.convert(v, o[1] as u8) {
                        Some(r) => self.store_local(&mut kinds, s, r),
                        None => bail!(),
                    }
                }
                Op::MoveLocal => match self.leaf(Mode::L, o[1], &kinds) {
                    Some(v) => self.store_local(&mut kinds, o[0] as u16, v),
                    None => bail!(),
                },
                Op::ForTestL | Op::ForTestG => {
                    let var = if insn.op == Op::ForTestL { self.leaf(Mode::L, o[0], &kinds) } else { self.load_global(o[0] as u32) };
                    let end = self.leaf(Mode::L, o[1], &kinds);
                    let step = self.leaf(Mode::L, o[2], &kinds);
                    let (Some(var), Some(end), Some(step)) = (var, end, step) else { bail!() };
                    let zero = (self.b.ins().iconst(types::I64, 0), K::Int);
                    let (Some(down), Some(ge), Some(le)) = (self.compare(Cmp::Lt, step, zero), self.compare(Cmp::Ge, var, end), self.compare(Cmp::Le, var, end)) else { bail!() };
                    let go = self.b.ins().select(down, ge, le);
                    self.branch(go, next, o[3] as usize, at, &kinds, &stack);
                    return Ok(());
                }
                Op::ForStepL => {
                    let (Some(var), Some(step)) = (self.leaf(Mode::L, o[0], &kinds), self.leaf(Mode::L, o[1], &kinds)) else { bail!() };
                    // (a 64-bit counter: past its type's end the loop ends,
                    // the counter as it was — wide::for_step)
                    if var.1.wide() {
                        let Some((r, ok)) = self.for_step_wide(var, step) else { bail!() };
                        self.store_local(&mut kinds, o[0] as u16, r);
                        self.branch(ok, o[2] as usize, next, at, &kinds, &stack);
                        return Ok(());
                    }
                    let Some(r) = self.arith(Bin::Add, var, step) else { bail!() };
                    self.store_local(&mut kinds, o[0] as u16, r);
                    self.edge(o[2] as usize, at, &kinds, &stack);
                    return Ok(());
                }
                Op::ForStepG => {
                    let (Some(var), Some(step)) = (self.load_global(o[0] as u32), self.leaf(Mode::L, o[1], &kinds)) else { bail!() };
                    if var.1.wide() {
                        let Some((r, ok)) = self.for_step_wide(var, step) else { bail!() };
                        if !self.store_global(o[0] as u32, r) {
                            bail!();
                        }
                        self.branch(ok, o[2] as usize, next, at, &kinds, &stack);
                        return Ok(());
                    }
                    let Some(r) = self.arith(Bin::Add, var, step) else { bail!() };
                    if !self.store_global(o[0] as u32, r) {
                        bail!();
                    }
                    self.edge(o[2] as usize, at, &kinds, &stack);
                    return Ok(());
                }
                Op::AGetLS | Op::AGetGS | Op::AGetLL | Op::AGetLG | Op::AGetGL | Op::AGetGG => {
                    let (am, im) = match insn.op {
                        Op::AGetLS => (Mode::L, Mode::S),
                        Op::AGetGS => (Mode::G, Mode::S),
                        Op::AGetLL => (Mode::L, Mode::L),
                        Op::AGetLG => (Mode::L, Mode::G),
                        Op::AGetGL => (Mode::G, Mode::L),
                        _ => (Mode::G, Mode::G),
                    };
                    let idx = if im == Mode::S { pop!() } else {
                        match self.leaf(im, o[1], &kinds) {
                            Some(e) => e,
                            None => bail!(),
                        }
                    };
                    let (Some(arr), Some(i)) = (self.array(am, o[0], &kinds), self.as_i64(idx)) else { bail!() };
                    let v = self.aget(arr, i, at, &kinds, &pre);
                    stack.push(v);
                }
                Op::ASetLS | Op::ASetGS | Op::ASetLL | Op::ASetLG | Op::ASetGL | Op::ASetGG => {
                    let (am, im, kind_at) = match insn.op {
                        Op::ASetLS => (Mode::L, Mode::S, 1),
                        Op::ASetGS => (Mode::G, Mode::S, 1),
                        Op::ASetLL => (Mode::L, Mode::L, 2),
                        Op::ASetLG => (Mode::L, Mode::G, 2),
                        Op::ASetGL => (Mode::G, Mode::L, 2),
                        _ => (Mode::G, Mode::G, 2),
                    };
                    // (S: the index on top, the value below it)
                    let idx = if im == Mode::S { pop!() } else {
                        match self.leaf(im, o[1], &kinds) {
                            Some(e) => e,
                            None => bail!(),
                        }
                    };
                    let mut value = pop!();
                    let kind = o[kind_at] as u8;
                    if kind != NO_KIND {
                        match self.convert(value, kind) {
                            Some(r) => value = r,
                            None => bail!(),
                        }
                    }
                    let (Some(arr), Some(i)) = (self.array(am, o[0], &kinds), self.as_i64(idx)) else { bail!() };
                    if !value.1.scalar() {
                        bail!();
                    }
                    self.aset(arr, i, value, at, &kinds, &pre);
                }
                op => {
                    if let Some((bin, ma, mb)) = op.as_fused_arith() {
                        // (S on one side: the value on the stack)
                        let (a, b) = match (ma, mb) {
                            (Mode::S, mb) => {
                                let a = pop!();
                                let Some(b) = self.leaf(mb, o[0], &kinds) else { bail!() };
                                (a, b)
                            }
                            (ma, Mode::S) => {
                                let b = pop!();
                                let Some(a) = self.leaf(ma, o[0], &kinds) else { bail!() };
                                (a, b)
                            }
                            (ma, mb) => {
                                let (Some(a), Some(b)) = (self.leaf(ma, o[0], &kinds), self.leaf(mb, o[1], &kinds)) else { bail!() };
                                (a, b)
                            }
                        };
                        let r = if bin == Bin::Mod { self.int_div(true, a, b, at, &kinds, &pre) } else { self.arith(bin, a, b) };
                        match r {
                            Some(r) => stack.push(r),
                            None => bail!(),
                        }
                    } else if let Some((cmp, ma, mb)) = op.as_cmp_jump() {
                        let (a, b, t) = match (ma, mb) {
                            (Mode::S, Mode::S) => {
                                let b = pop!();
                                let a = pop!();
                                (a, b, o[0])
                            }
                            (Mode::S, mb) => {
                                let a = pop!();
                                let Some(b) = self.leaf(mb, o[0], &kinds) else { bail!() };
                                (a, b, o[1])
                            }
                            (ma, mb) => {
                                let (Some(a), Some(b)) = (self.leaf(ma, o[0], &kinds), self.leaf(mb, o[1], &kinds)) else { bail!() };
                                (a, b, o[2])
                            }
                        };
                        let Some(go) = self.compare(cmp, a, b) else { bail!() };
                        self.branch(go, next, t as usize, at, &kinds, &stack);
                        return Ok(());
                    } else {
                        // (a host operation, text, objects, GOSUB, END …)
                        bail!();
                    }
                }
            }
            ip = next;
        }
    }

    /// A 64-bit FOR counter's step (`rapidr_value::wide::for_step`): the
    /// counter after it — the same one when the step would pass the type's
    /// end — and whether the loop goes on. A step of another kind (a
    /// DOUBLE, a UINT64 on an INT64): the interpreter's.
    fn for_step_wide(&mut self, var: Entry, step: Entry) -> Option<(Entry, V)> {
        let (v, s) = (var.0, step.0);
        let r = self.b.ins().iadd(v, s);
        let ok = match (var.1, step.1) {
            // (signed + signed: no overflow when the sign of r is either's)
            (K::I64, K::Int | K::I64) => {
                let x = self.b.ins().bxor(v, r);
                let y = self.b.ins().bxor(s, r);
                let both = self.b.ins().band(x, y);
                self.b.ins().icmp_imm_s(IntCC::SignedGreaterThanOrEqual, both, 0)
            }
            // (unsigned + signed: up without a carry, down without a borrow)
            (K::U64, K::Int | K::I64) => {
                let up = self.b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, r, v);
                let down = self.b.ins().icmp(IntCC::UnsignedLessThanOrEqual, r, v);
                let negative = self.b.ins().icmp_imm_s(IntCC::SignedLessThan, s, 0);
                self.b.ins().select(negative, down, up)
            }
            (K::U64, K::U64) => self.b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, r, v),
            _ => return None,
        };
        Some(((self.b.ins().select(ok, r, v), var.1), ok))
    }

    /// `a MOD b` / `a \ b` on two Integers (`fast::modulo`,
    /// `Value::checked_int_div`): both as 32-bit integers
    /// (`int32_of`), by zero the interpreter's "Division by zero".
    fn int_div(&mut self, modulo: bool, a: Entry, b: Entry, at: usize, kinds: &[K], pre: &[Entry]) -> Option<Entry> {
        if (a.1.wide() || b.1.wide()) && a.1.whole() && b.1.whole() {
            return self.int_div_wide(modulo, a, b, at, kinds, pre);
        }
        if a.1 != K::Int || b.1 != K::Int {
            return None;
        }
        let y = self.int32_of(b.0);
        let zero = self.b.ins().icmp_imm_s(IntCC::Equal, y, 0);
        self.cond_exit(zero, at, kinds, pre, Why::Error);
        let x = self.int32_of(a.0);
        let r = if modulo { self.b.ins().srem(x, y) } else { self.b.ins().sdiv(x, y) };
        Some((r, K::Int))
    }

    /// `a \ b` / `a MOD b` with an INT64 / UINT64 side (`wide::div_mod`):
    /// exact, by zero the interpreter's error; both signed, or unsigned
    /// with no negative side (a negative one: the interpreter's, in 128
    /// bits).
    fn int_div_wide(&mut self, modulo: bool, a: Entry, b: Entry, at: usize, kinds: &[K], pre: &[Entry]) -> Option<Entry> {
        let (ua, ub) = (a.1 == K::U64, b.1 == K::U64);
        let zero = self.b.ins().icmp_imm_s(IntCC::Equal, b.0, 0);
        self.cond_exit(zero, at, kinds, pre, Why::Error);
        if ua != ub {
            // (one of each: unsigned while the signed side isn't negative —
            // `u MOD 7`; a negative one is the interpreter's, in 128 bits)
            let signed = if ua { b.0 } else { a.0 };
            let negative = self.b.ins().icmp_imm_s(IntCC::SignedLessThan, signed, 0);
            self.cond_exit(negative, at, kinds, pre, Why::Guard);
        }
        if ua || ub {
            let r = if modulo { self.b.ins().urem(a.0, b.0) } else { self.b.ins().udiv(a.0, b.0) };
            return Some((r, K::U64));
        }
        // (the smallest INT64 \ -1 is itself, MOD -1 is 0: no trap)
        let minus_one = self.b.ins().icmp_imm_s(IntCC::Equal, b.0, -1);
        let one = self.b.ins().iconst(types::I64, 1);
        let safe = self.b.ins().select(minus_one, one, b.0);
        let r = if modulo {
            let m = self.b.ins().srem(a.0, safe);
            let z = self.b.ins().iconst(types::I64, 0);
            self.b.ins().select(minus_one, z, m)
        } else {
            let q = self.b.ins().sdiv(a.0, safe);
            let n = self.b.ins().ineg(a.0);
            self.b.ins().select(minus_one, n, q)
        };
        Some((r, K::I64))
    }
}

/// A callee form that goes straight back to the interpreter: it writes out
/// its frame at the routine's start (the parameters as passed, the other
/// locals Null) as exit `record` and returns 1 — the record it made is
/// returned for the engine to keep at that index.
#[allow(clippy::too_many_arguments)]
pub fn build_stub(m: &Module, fi: u32, params: &[K], func: &mut ir::Function, fctx: &mut FunctionBuilderContext, jm: &mut JITModule, helpers: Helpers, record: u32) -> Result<ExitRecord, Unbuilt> {
    let f = m.functions.get(fi as usize).ok_or_else(|| Unbuilt("no such routine".into()))?;
    let frontend = jm.isa().frontend_config();
    let exit = jm.declare_func_in_func(helpers.exit, func);
    let mut b = FunctionBuilder::new(func, fctx);
    let block = b.create_block();
    b.append_block_params_for_function_params(block);
    b.switch_to_block(block);
    let args = b.block_params(block).to_vec();
    let mut locals = Vec::new();
    let mut vals = Vec::new();
    let mut next = 1;
    for s in 0..f.n_locals as usize {
        let k = params.get(s).copied().unwrap_or(K::Null);
        locals.push((s as u16, k));
        if ty(k).is_some() {
            let v = args[next];
            next += 1;
            vals.push(match k {
                K::Dbl => b.ins().bitcast(types::I64, MemFlagsData::new(), v),
                K::Bool => b.ins().uextend(types::I64, v),
                _ => v,
            });
        }
    }
    let n = vals.len();
    let slot = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, (8 * n.max(1)) as u32, 3));
    let addr = b.ins().stack_addr(types::I64, slot, 0);
    for (i, v) in vals.into_iter().enumerate() {
        b.ins().store(MemFlagsData::trusted(), v, addr, (i * 8) as i32);
    }
    let idv = b.ins().iconst(types::I32, i64::from(record));
    let nv = b.ins().iconst(types::I32, n as i64);
    b.ins().call(exit, &[args[0], idv, addr, nv]);
    let one = b.ins().iconst(types::I32, 1);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().return_(&[one, zero]);
    b.seal_all_blocks();
    b.finalize(frontend);
    Ok(ExitRecord { fn_index: fi, ip: 0, locals, stack: Vec::new(), arrays: Vec::new(), why: Why::Planned, form: usize::MAX, line: f.get_line_for_ip(0).unwrap_or(0), what: String::new() })
}

/// A new function of the code generator for a form.
pub fn new_function(sig: Signature, n: u32) -> ir::Function {
    ir::Function::with_name_signature(UserFuncName::user(0, n), sig)
}
