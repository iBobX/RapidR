//! The JIT the VM asks (`rapidr_vm::Jit`): when a loop or a routine is
//! hot it compiles it (with every routine it calls), and where compiled
//! code exists it runs it — reading the frame's values into the buffers
//! compiled code works on, and afterwards writing back what it left, as
//! the interpreter would have it at the instruction compiled code stopped
//! before (docs/jit-design.md §3, §4).

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::rc::{Rc, Weak};
use std::time::{Duration, Instant};

use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{default_libcall_names, FuncId, Linkage, Module as _};

use rapidr_bytecode::disasm::decode;
use rapidr_bytecode::{Module, Op, Operand};
use rapidr_value::Value;
use rapidr_vm::{Frame, JitAnswer, JitView};

use crate::compile::{self, Callable, Env, ExitRecord, FormKind, FormOut, Helpers, Observed, Why};
use crate::kinds::{bits_of, value_of, K};
use crate::native::{self, Ctx};
use crate::Mode;

/// A compiled frame whose size the code generator didn't say: taken as
/// this big (it always says; a guess only errs on the safe side).
const UNKNOWN_FRAME: usize = 256 << 10;

/// Turns of a loop before it is compiled (`always`: at once).
const LOOP_HOT: u32 = 10_000;
/// Calls of a routine before it is compiled.
const CALL_HOT: u32 = 1_000;
// (compiled calls inside compiled calls: as deep as the machine stack has
// room for, in bytes, and never more frames than the VM may have —
// `Ctx::stack_limit`, SEC-50; it replaced a cap of 256 frames by count)
/// Times compiled code may find values of other kinds than it was
/// compiled for before it is used no more.
const MAX_DEOPTS: u32 = 10;

/// Where compiled code may start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Start {
    Counting(u32),
    Compiled(usize),
    Rejected,
}

/// A compiled entry form.
struct Form {
    fi: u32,
    code: *const u8,
    /// The frame's locals it uses and the kinds it was compiled for.
    guard: Vec<(u16, K)>,
    larrays: Vec<u16>,
    globals: Vec<usize>,
    garrays: Vec<usize>,
    n_locals: usize,
    /// Its region (a loop's: head to the end of its last backward jump).
    region: (usize, usize),
    deopts: u32,
    runs: u64,
    disabled: bool,
    /// The loop around it was made hot (see `PROMOTE`).
    promoted: bool,
    /// Its runs that went back to the interpreter at an instruction it
    /// doesn't run, and the loop turns its runs made (see `RARE`).
    unclean: u64,
    turns: u64,
}

/// A form compiled with a way back to the interpreter that a run can go
/// round (`unavoidable`): once it has run this often, it is used no more
/// if most of its runs went back that way having made few loop turns —
/// the way wasn't rare (docs/jit-design.md §12.8).
const RARE: u64 = 64;
/// Loop turns a run makes on average for its way back not to matter.
const RARE_TURNS: u64 = 16;
/// A compile this long is named by `--verbose` (the program waits for it,
/// once).
const SLOW_COMPILE: Duration = Duration::from_millis(20);

/// Runs of an inner loop's form from the interpreter before the loop
/// around it is compiled too (whose turns are few — its inner loop's
/// were counted —: a synthesizer's voice loop around its samples' loop).
const PROMOTE: u64 = 16;

/// A local written back at an exit, of kind `k`, from the exit's values:
/// none for a Null, its bits for a scalar, its bits and its kind's code for
/// a `K::Dyn` (the kind it held there).
fn local_value<'a>(k: K, vals: &mut impl Iterator<Item = &'a u64>) -> Option<Value> {
    Some(match k {
        K::Null => Value::Null,
        K::Dyn => {
            let bits = *vals.next()?;
            value_of(K::from_code(*vals.next()? as u32), bits)
        }
        k => value_of(k, *vals.next()?),
    })
}

/// Stack entry `d` (of kind `k`) of exit `rec`, from the exit's values: a
/// global array is the one its global holds (the very value the
/// interpreter's `LoadGlobal` pushed: compiled code never makes a global
/// hold another array).
fn stack_value<'a>(rec: &ExitRecord, d: usize, k: K, globals: &[Option<Value>], vals: &mut impl Iterator<Item = &'a u64>) -> Option<Value> {
    if k == K::Arr {
        let &(_, g) = rec.arrays.iter().find(|a| a.0 as usize == d)?;
        return globals.get(g as usize)?.clone();
    }
    local_value(k, vals)
}

/// The innermost loop of `code` around `region` (its head), if any.
fn enclosing_loop(code: &[u8], region: (usize, usize)) -> Option<usize> {
    let mut best: Option<(usize, usize)> = None;
    let mut ip = 0;
    while let Some(insn) = decode(code, ip) {
        for (o, v) in insn.op.layout().iter().zip(&insn.operands) {
            let t = *v as usize;
            if *o == Operand::Target && t <= ip && t < region.0 && insn.next >= region.1 && best.is_none_or(|b| insn.next - t < b.1 - b.0) {
                best = Some((t, insn.next));
            }
        }
        ip = insn.next;
    }
    best.map(|b| b.0)
}

/// What the engine did (`rapidr run --verbose`).
#[derive(Default)]
struct Stats {
    loops: u32,
    routines: u32,
    callees: u32,
    rejected: u32,
    code_bytes: usize,
    /// The largest compiled frame, in bytes (SEC-50).
    max_frame: usize,
    /// The machine stack free where compiled code first ran.
    stack_free: Option<usize>,
    /// Entries refused: too little machine stack left there.
    low_stack: u64,
    compile_time: Duration,
    /// Of it, the code generator's making the machine code (the rest is
    /// RapidR's: the IR from the bytecode, its rounds).
    codegen_time: Duration,
    runs: u64,
    exits: HashMap<&'static str, u64>,
    notes: Vec<String>,
    /// The places left interpreted (`rapidr run --verbose`), at most
    /// [`MAX_NOTES`].
    refused: Vec<Refused>,
}

/// The most places left interpreted `--verbose` names.
const MAX_NOTES: usize = 48;
/// The places compiled code went back to the interpreter from most often
/// that `--verbose` names.
const TOP_EXITS: usize = 6;

/// A loop or a routine left interpreted, and where and why
/// (`rapidr run --verbose`).
struct Refused {
    /// "P2_Iif", "the main program's loop", and its place ("cpu.inc:812").
    who: String,
    place: String,
    /// Where it can't be compiled (its place and compiled line), why, and
    /// the instruction there; `None`: why only.
    at: Option<(String, u32, Why, String)>,
    why: String,
}

/// What a way back to the interpreter is, in words.
fn words(why: Why) -> &'static str {
    match why {
        Why::Planned => "an instruction it leaves to the interpreter",
        Why::Conflict => "kinds that disagree",
        Why::LoopExit => "the end of a loop",
        Why::Return => "a return",
        Why::Guard => "a value of another kind",
        Why::Error => "a run-time error",
        Why::Interrupt => "the debugger",
        Why::Depth => "deep calls",
        Why::AfterCall { .. } => "a call",
        Why::Pending => "a call not worked out",
    }
}

/// A routine's name as `--verbose` says it.
fn shown_name(name: &str) -> &str {
    if name == "__main" {
        "the main program"
    } else {
        name
    }
}

/// Compiled line `line`'s place: its file and line there ("cpu.inc:812"),
/// else "line N".
fn place(map: &rapidr_bytecode::SourceMap, line: u32) -> String {
    match map.locate(line) {
        Some((file, l)) => format!("{file}:{l}"),
        None if line > 0 => format!("line {line}"),
        None => "?".into(),
    }
}

/// A statement's text for `--verbose`: one line, spaces folded, at most
/// 90 characters.
fn statement(text: &str) -> String {
    let folded: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if folded.chars().count() > 90 {
        let cut: String = folded.chars().take(87).collect();
        format!("{cut}...")
    } else {
        folded
    }
}

/// Everything compiled for one module.
struct Compiled {
    module: usize,
    jm: Option<JITModule>,
    fctx: FunctionBuilderContext,
    helpers: Helpers,
    env: Env,
    records: Vec<ExitRecord>,
    forms: Vec<Form>,
    loops: HashMap<(u32, usize), Start>,
    starts: HashMap<u32, Start>,
    gbuf: Vec<u64>,
    ctx: Ctx,
    locals: Vec<u64>,
    /// The largest frame of any routine compiled for the module, its frame
    /// pointer and return address included: the room each compiled call
    /// must leave above `native::STACK_MARGIN` (SEC-50).
    max_frame: usize,
    /// The smallest: the least stack a compiled call takes (frames the VM
    /// may have, counted in bytes).
    min_frame: usize,
    /// The code generator failed beyond repair: nothing more compiled or
    /// run for this module.
    broken: Option<String>,
    /// Where the last loop or routine refused can't be compiled, and why.
    refused_at: Option<(usize, Why)>,
    /// How often compiled code went back to the interpreter at each record
    /// (the innermost frame's), and the module's routine names and source
    /// map, for `--verbose`.
    exit_counts: Vec<u64>,
    names: Vec<String>,
    map: rapidr_bytecode::SourceMap,
}

/// The JIT of one VM. Its state is shared with [`report_now`] (a program
/// that ENDs: the process ends inside the VM's run, so `--verbose` asks
/// for the report there), borrowed only while the VM asks it something.
pub struct Engine {
    inner: Rc<RefCell<Inner>>,
}

thread_local! {
    /// This thread's last JIT ([`report_now`]).
    static LAST: RefCell<Weak<RefCell<Inner>>> = const { RefCell::new(Weak::new()) };
}

impl Engine {
    pub fn new(mode: Mode) -> Engine {
        let inner = Rc::new(RefCell::new(Inner::new(mode)));
        LAST.with(|l| *l.borrow_mut() = Rc::downgrade(&inner));
        Engine { inner }
    }
}

/// What this thread's JIT did so far ([`rapidr_vm::Jit::report_with`]), for
/// a program about to end the process (`END`): `None` without one, or
/// while it is busy.
pub fn report_now(text: &dyn Fn(u32) -> Option<String>) -> Option<String> {
    let inner = LAST.with(|l| l.borrow().upgrade())?;
    let inner = inner.try_borrow().ok()?;
    Some(rapidr_vm::Jit::report_with(&*inner, text))
}

impl rapidr_vm::Jit for Engine {
    fn loop_head(&mut self, view: JitView<'_>, ip: usize, turns: u32) -> JitAnswer {
        self.inner.borrow_mut().loop_head(view, ip, turns)
    }
    fn called(&mut self, view: JitView<'_>, calls: u32) -> JitAnswer {
        self.inner.borrow_mut().called(view, calls)
    }
    fn first_ask(&self) -> (u32, u32) {
        self.inner.borrow().first_ask()
    }
    fn report(&self) -> String {
        self.inner.borrow().report()
    }
    fn report_with(&self, text: &dyn Fn(u32) -> Option<String>) -> String {
        self.inner.borrow().report_with(text)
    }
}

struct Inner {
    mode: Mode,
    state: Option<Compiled>,
    /// The JIT can't run (no memory for code, a module it couldn't set
    /// up): the interpreter runs everything.
    off: Option<String>,
    stats: Stats,
}

impl Inner {
    fn new(mode: Mode) -> Inner {
        Inner { mode, state: None, off: None, stats: Stats::default() }
    }

    fn hot(&self) -> (u32, u32) {
        match self.mode {
            Mode::Always => (1, 1),
            _ => (LOOP_HOT, CALL_HOT),
        }
    }

    /// The module's compiled code, set up the first time.
    fn state(&mut self, m: &Module) -> Option<&mut Compiled> {
        if self.off.is_some() {
            return None;
        }
        let id = m as *const Module as usize;
        if self.state.is_none() {
            match new_state(m, self.mode) {
                Ok(s) => self.state = Some(s),
                Err(e) => {
                    self.off = Some(e);
                    return None;
                }
            }
        }
        // (one module per VM: a debugger's snippet runs in an evaluation,
        // where nothing compiled runs)
        if let Some(why) = self.state.as_ref().and_then(|s| s.broken.clone()) {
            self.off = Some(why);
            return None;
        }
        self.state.as_mut().filter(|s| s.module == id)
    }

    fn count_exit(&mut self, why: Why) {
        *self.stats.exits.entry(words(why)).or_default() += 1;
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        if let Some(jm) = self.state.as_mut().and_then(|s| s.jm.take()) {
            native::free(jm);
        }
    }
}

/// `RAPIDR_JIT_VERIFY=1` (or `on`, `yes`): the IR verifier on (SEC-52).
fn verify_asked() -> bool {
    std::env::var("RAPIDR_JIT_VERIFY").is_ok_and(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "on" | "yes"))
}

fn new_state(m: &Module, mode: Mode) -> Result<Compiled, String> {
    let mut flags = settings::builder();
    let set = |f: &mut settings::Builder, k: &str, v: &str| f.set(k, v).map_err(|e| format!("{k}: {e}"));
    set(&mut flags, "opt_level", "speed")?;
    set(&mut flags, "use_colocated_libcalls", "false")?;
    set(&mut flags, "is_pic", "false")?;
    set(&mut flags, "preserve_frame_pointers", "true")?;
    set(&mut flags, "enable_multi_ret_implicit_sret", "true")?;
    // (SEC-52: the code generator's IR verifier — a function it finds
    // malformed is refused, the interpreter runs it — in debug builds, in
    // `always` mode, the tests' way (conformance's vm-jit, the JIT's own
    // tests), and with RAPIDR_JIT_VERIFY=1; `rapidr run`'s default leaves
    // it off: a quarter more compile time, docs/jit-design.md §12.7)
    let verify = cfg!(debug_assertions) || mode == Mode::Always || verify_asked();
    set(&mut flags, "enable_verifier", if verify { "true" } else { "false" })?;
    // (SEC-50: a frame larger than a page touches every page on the way
    // down — none steps over the guard below the stack, and Windows commits
    // its stack page by page; inline: there is no probe function to call)
    set(&mut flags, "enable_probestack", "true")?;
    set(&mut flags, "probestack_strategy", "inline")?;
    set(&mut flags, "probestack_size_log2", "12")?;
    let isa = cranelift_native::builder()
        .map_err(|e| format!("this processor isn't one the JIT knows ({e})"))?
        .finish(settings::Flags::new(flags))
        .map_err(|e| e.to_string())?;
    let mut jb = JITBuilder::with_isa(isa, default_libcall_names());
    for (name, p) in native::helper_symbols() {
        jb.symbol(name, p);
    }
    if let Some(p) = native::memory_provider()? {
        jb.memory_provider(p);
    }
    let mut jm = JITModule::new(jb);
    let helpers = Helpers::declare(&mut jm)?;
    Ok(Compiled {
        module: m as *const Module as usize,
        jm: Some(jm),
        fctx: FunctionBuilderContext::new(),
        helpers,
        env: Env::new(m),
        records: Vec::new(),
        forms: Vec::new(),
        loops: HashMap::new(),
        starts: HashMap::new(),
        gbuf: Vec::new(),
        ctx: Ctx { globals: std::ptr::null_mut(), interrupt: &native::NOT_INTERRUPTED, stack_limit: u64::MAX, turns: 0, exits: Vec::new(), garrays: Vec::new(), larrays: Vec::new() },
        locals: Vec::new(),
        max_frame: 0,
        min_frame: usize::MAX,
        broken: None,
        refused_at: None,
        exit_counts: Vec::new(),
        names: m.functions.iter().map(|f| f.name.clone()).collect(),
        map: m.source_map.clone(),
    })
}

/// Of the places a form leaves for the interpreter at an instruction it
/// doesn't run (or kinds that disagree), the first that every run of it
/// passes — a routine's from its start to any return, a loop's turn from
/// its head back to it (or out of it) —; `None` when each one can be gone
/// round (docs/jit-design.md §12.8). A routine with a rare way out — an
/// emulator's CPU step that prints a character once in a thousand
/// instructions, a `CASE` that calls a routine of the host's — is worth
/// compiling: it leaves on that way only; one that leaves on every run is
/// not.
fn unavoidable(code: &[u8], region: (usize, usize), start: usize, is_loop: bool, unclean: &[(usize, Why)]) -> Option<(usize, Why)> {
    // (every instruction's successors in the region; a jump out of it, a
    // return, the code's end: a run that got through)
    let mut index = HashMap::new();
    let mut succ: Vec<Vec<usize>> = Vec::new();
    let mut done: Vec<bool> = Vec::new();
    let mut ip = 0;
    while let Some(insn) = decode(code, ip) {
        index.insert(ip, succ.len());
        let mut next: Vec<usize> = insn.op.layout().iter().zip(&insn.operands).filter(|(o, _)| **o == Operand::Target).map(|(_, v)| *v as usize).collect();
        let ends = matches!(insn.op, Op::Ret | Op::RetVal | Op::Halt | Op::GosubRet);
        if !ends && insn.op != Op::Jump {
            next.push(insn.next);
        }
        done.push(ends);
        succ.push(next);
        ip = insn.next;
    }
    let mut seen_at: BTreeSet<usize> = BTreeSet::new();
    for &(at, why) in unclean {
        if !seen_at.insert(at) || seen_at.len() > 512 {
            continue;
        }
        // (a walk from the start that never steps on `at`)
        let mut visited = vec![false; succ.len()];
        let mut todo = vec![start];
        let mut through = false;
        while let Some(ip) = todo.pop() {
            if ip == at {
                continue;
            }
            let Some(&i) = index.get(&ip) else {
                // (the code's end: the routine returns)
                through = true;
                break;
            };
            if visited[i] {
                continue;
            }
            visited[i] = true;
            if done[i] {
                through = true;
                break;
            }
            for &t in &succ[i] {
                // (round to the loop's head again, or out of the loop)
                if (is_loop && t == start) || t < region.0 || t >= region.1 {
                    through = true;
                    break;
                }
                todo.push(t);
            }
            if through {
                break;
            }
        }
        if !through {
            return Some((at, why));
        }
    }
    None
}

/// The loop whose head is `head`: from it to the end of the last backward
/// jump to it.
fn loop_region(code: &[u8], head: usize) -> Option<(usize, usize)> {
    let mut end = None;
    let mut ip = 0;
    while let Some(insn) = decode(code, ip) {
        if ip >= head {
            for (o, v) in insn.op.layout().iter().zip(&insn.operands) {
                if *o == Operand::Target && *v as usize == head {
                    end = Some(insn.next);
                }
            }
        }
        ip = insn.next;
    }
    end.map(|e| (head, e))
}

impl Compiled {
    fn jm(&mut self) -> &mut JITModule {
        self.jm.as_mut().expect("the module's compiled code")
    }

    /// One build of a form (a round's, or the final one).
    #[allow(clippy::too_many_arguments)]
    fn build(&mut self, m: &Module, fi: u32, kind: &FormKind, entry: &[K], obs: &Observed, hint: Option<&BTreeSet<u16>>, sig: cranelift_codegen::ir::Signature) -> Result<(FormOut, cranelift_codegen::ir::Function), String> {
        let mut func = compile::new_function(sig, fi);
        let base = self.records.len();
        let jm = self.jm.as_mut().expect("the module's compiled code");
        let out = compile::build(m, fi, kind, entry, &mut func, &mut self.fctx, jm, self.helpers, &mut self.env, obs, hint, base).map_err(|e| e.0)?;
        Ok((out, func))
    }

    /// Works out which routines the form calls can be called compiled,
    /// building each one's callee form until nothing changes; then the
    /// entry form's last round.
    /// `strict`: a routine or loop with an instruction compiled code leaves
    /// to the interpreter isn't worth compiling (`always` compiles it all
    /// the same — the tests' way: every exit taken, often).
    fn settle(&mut self, m: &Module, fi: u32, kind: &FormKind, entry: &[K], obs: &Observed, strict: bool) -> Result<FormOut, String> {
        let entry_sig = compile::entry_signature(self.jm());
        for _round in 0..16 {
            let (out, _) = self.build(m, fi, kind, entry, obs, None, entry_sig.clone())?;
            let mut changed = false;
            let open: Vec<(u32, Vec<K>)> = self.env.callees.iter().filter(|(_, c)| !c.defined && c.callable != Callable::No).map(|(k, _)| k.clone()).collect();
            for spec in open {
                let ckind = FormKind::Callee { params: spec.1.clone() };
                let (cout, _) = self.build(m, spec.0, &ckind, &[], obs, None, compile::callee_signature(&spec.1))?;
                let c = self.env.callees.get_mut(&spec).expect("a callee");
                let before = (c.callable, c.ret);
                let f = &m.functions[spec.0 as usize];
                if (strict && unavoidable(&f.code, (0, f.code.len() + 1), 0, false, &cout.unclean).is_some()) || cout.rets.len() > 1 {
                    c.callable = Callable::No;
                } else {
                    if c.ret.is_none() {
                        c.ret = cout.rets.iter().next().copied();
                    }
                    c.callable = if c.ret.is_some() && !cout.pending { Callable::Yes } else { Callable::Unknown };
                }
                c.globals = cout.globals.clone();
                c.garrays = cout.garrays.clone();
                c.calls = cout.calls.clone();
                changed |= before != (c.callable, c.ret);
            }
            if !changed {
                // (settled — unless a routine is still not worked out: none
                // of those can be, and their callers are built again
                // without them; a call is only ever compiled to a routine
                // that is worked out and will be defined)
                let mut open = false;
                for c in self.env.callees.values_mut() {
                    if !c.defined && c.callable == Callable::Unknown {
                        c.callable = Callable::No;
                        open = true;
                    }
                }
                if !open && !out.pending {
                    return Ok(out);
                }
            }
        }
        Err("its routines' calls didn't settle".into())
    }

    /// Compiles the entry form of `fi` at `ip` (`start`: the routine's
    /// start, else a loop's head) for the frame on top of `view`.
    #[allow(clippy::result_large_err)]
    fn compile(&mut self, view: &JitView<'_>, fi: u32, ip: usize, start: bool, strict: bool, stats: &mut Stats) -> Result<usize, String> {
        let m = view.module;
        let t0 = Instant::now();
        let frame = view.frames.last().ok_or("no frame")?;
        if view.stack.len() != frame.stack_base {
            return Err("values on the stack where it would start".into());
        }
        let f = &m.functions[fi as usize];
        let region = if start { (0, f.code.len() + 1) } else { loop_region(&f.code, ip).ok_or("not a loop's head")? };
        let kind = FormKind::Entry { ip, region };
        let entry: Vec<K> = frame.locals.iter().map(K::of).collect();
        self.env.direct_maths = view.direct_maths;
        let obs = Observed { globals: view.globals, locals: Some(&frame.locals) };
        let out = self.settle(m, fi, &kind, &entry, &obs, strict)?;
        if let Some((at, why)) = unavoidable(&f.code, region, ip, !start, &out.unclean).filter(|_| strict) {
            self.refused_at = Some((at, why));
            return Err(format!("{why:?} at byte {at}"));
        }
        // (the final builds: every callee form found callable, then the
        // entry form loading and writing back only the locals it uses)
        let jm_err = |e: cranelift_module::ModuleError| format!("{e:?}");
        let mut defined: Vec<(u32, Vec<K>)> = Vec::new();
        let todo: Vec<(u32, Vec<K>)> = self.env.callees.iter().filter(|(_, c)| !c.defined && c.callable == Callable::Yes).map(|(k, _)| k.clone()).collect();
        let mut result = Ok(());
        for spec in &todo {
            let ckind = FormKind::Callee { params: spec.1.clone() };
            let built = self.build(m, spec.0, &ckind, &[], &obs, None, compile::callee_signature(&spec.1));
            let (cout, func) = match built {
                Ok(b) => b,
                Err(e) => {
                    result = Err(e);
                    break;
                }
            };
            let id = self.env.callees[spec].func;
            if let Err(e) = self.define(id, func, stats) {
                // (callers already defined may call it: in its place a form
                // that goes straight back to the interpreter at the
                // routine's start — its frame, its parameters)
                stats.notes.push(format!("a routine compiled as a stub ({})", jm_err(e)));
                let stub = self.stub(m, spec);
                if let Err(e) = stub.and_then(|func| self.define(id, func, stats).map_err(jm_err)) {
                    result = Err(e);
                    break;
                }
            } else {
                self.commit_records(cout.records, usize::MAX);
            }
            defined.push(spec.clone());
            stats.callees += 1;
        }
        for spec in &defined {
            if let Some(c) = self.env.callees.get_mut(spec) {
                c.defined = true;
            }
        }
        let form = result.and_then(|()| {
            let hint = out.used_locals.clone();
            let sig = compile::entry_signature(self.jm());
            let (fout, func) = self.build(m, fi, &kind, &entry, &obs, Some(&hint), sig.clone())?;
            if !fout.used_locals.is_subset(&hint) || (strict && unavoidable(&f.code, region, ip, !start, &fout.unclean).is_some()) {
                return Err("its last build differed".into());
            }
            let name = self.env.next_name();
            let id = self.jm().declare_function(&name, Linkage::Local, &sig).map_err(jm_err)?;
            self.define(id, func, stats).map_err(jm_err)?;
            Ok((id, fout, hint))
        });
        // (defined code is made executable, whatever happened after; a
        // failure there — the code generator's own — and nothing of this
        // module is compiled or run any more)
        let finalized = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.jm().finalize_definitions()));
        match finalized {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                self.broken = Some(jm_err(e));
                return Err("the compiled code couldn't be made executable".into());
            }
            Err(_) => {
                self.broken = Some("the code generator failed".into());
                return Err("the code generator failed".into());
            }
        }
        let (id, fout, hint) = form?;
        let code = self.jm().get_finalized_function(id);
        let form_index = self.forms.len();
        self.commit_records(fout.records, form_index);
        // (the globals and arrays of everything it calls too)
        let mut globals = fout.globals.clone();
        let mut garrays = fout.garrays.clone();
        let mut seen = BTreeSet::new();
        let mut stack: Vec<(u32, Vec<K>)> = fout.calls.iter().cloned().collect();
        while let Some(spec) = stack.pop() {
            if !seen.insert(spec.clone()) {
                continue;
            }
            if let Some(c) = self.env.callees.get(&spec) {
                globals.extend(c.globals.iter().copied());
                garrays.extend(c.garrays.iter().copied());
                stack.extend(c.calls.iter().cloned());
            }
        }
        let guard = hint.iter().map(|&s| (s, entry[s as usize])).collect();
        self.forms.push(Form {
            fi,
            code,
            guard,
            larrays: fout.larrays.iter().map(|&(s, _)| s).collect(),
            globals: globals.into_iter().collect(),
            garrays: garrays.into_iter().collect(),
            n_locals: f.n_locals as usize,
            region,
            deopts: 0,
            runs: 0,
            disabled: false,
            promoted: false,
            unclean: 0,
            turns: 0,
        });
        let took = t0.elapsed();
        stats.compile_time += took;
        // (a long one said: the program paused that long, once)
        if took >= SLOW_COMPILE && stats.notes.len() < 32 {
            let name = shown_name(&f.name);
            let who = if start { name.to_string() } else { format!("{name}'s loop") };
            stats.notes.push(format!("{who} ({}): compiled in {:.0} ms", place(&m.source_map, f.get_line_for_ip(ip).unwrap_or(0)), took.as_secs_f64() * 1000.0));
        }
        if start {
            stats.routines += 1;
        } else {
            stats.loops += 1;
        }
        Ok(form_index)
    }

    /// A callee form that goes straight back to the interpreter: the frame
    /// at the routine's start, its parameters as passed. (In place of one
    /// the code generator refused, so its callers stay sound.)
    fn stub(&mut self, m: &Module, spec: &(u32, Vec<K>)) -> Result<cranelift_codegen::ir::Function, String> {
        let rec = self.records.len() as u32;
        let mut func = compile::new_function(compile::callee_signature(&spec.1), spec.0);
        let jm = self.jm.as_mut().expect("the module's compiled code");
        let record = compile::build_stub(m, spec.0, &spec.1, &mut func, &mut self.fctx, jm, self.helpers, rec).map_err(|e| e.0)?;
        self.records.push(record);
        Ok(func)
    }

    // (the generator's own error, as it comes: rare, and only reported)
    #[allow(clippy::result_large_err)]
    fn define(&mut self, id: FuncId, func: cranelift_codegen::ir::Function, stats: &mut Stats) -> Result<(), cranelift_module::ModuleError> {
        if stats.code_bytes > native::CODE_CAP {
            return Err(cranelift_module::ModuleError::Allocation { err: std::io::Error::new(std::io::ErrorKind::OutOfMemory, "the compiled code's cap is reached") });
        }
        let jm = self.jm.as_mut().expect("the module's compiled code");
        let mut ctx = jm.make_context();
        ctx.func = func;
        let t0 = Instant::now();
        let defined = jm.define_function(id, &mut ctx);
        stats.codegen_time += t0.elapsed();
        defined?;
        if let Some(c) = ctx.compiled_code() {
            stats.code_bytes += c.code_buffer().len();
            // (its frame: below the frame pointer, then the frame pointer
            // and the return address — the code generator says it; were it
            // not to, the largest and the least a frame could be)
            let layout = c.buffer.frame_layout().map(|l| l.frame_to_fp_offset as usize + native::MIN_FRAME);
            self.max_frame = self.max_frame.max(layout.unwrap_or(UNKNOWN_FRAME));
            self.min_frame = self.min_frame.min(layout.unwrap_or(native::MIN_FRAME));
            stats.max_frame = stats.max_frame.max(self.max_frame);
        }
        jm.clear_context(&mut ctx);
        Ok(())
    }

    fn commit_records(&mut self, records: Vec<ExitRecord>, form: usize) {
        for mut r in records {
            r.form = form;
            self.records.push(r);
        }
    }

    /// The lowest stack pointer compiled code may make a call at, from here
    /// (SEC-50, `Ctx::stack_limit`), and whether the VM's room for frames
    /// (`depth_room`, before `MAX_CALL_DEPTH`) set it rather than the
    /// machine stack's: the thread's stack end, the margin the helpers run
    /// in and the largest compiled frame above it — or, higher, here less
    /// the smallest compiled frame for each frame the VM has room for (a
    /// compiled call takes at least that much: never more frames than the
    /// VM may have). `None`: not room enough here for the entry form's own
    /// frame (or the stack can't be measured) — the interpreter runs it.
    fn stack_limit(&self, stats: &mut Stats, depth_room: usize) -> Option<(u64, bool)> {
        let here = native::stack_here();
        let end = native::stack_end()?;
        let floor = end.checked_add(native::STACK_MARGIN)?.checked_add(self.max_frame)?;
        if stats.stack_free.is_none() {
            stats.stack_free = Some(here.saturating_sub(end));
        }
        if here <= floor {
            return None;
        }
        let frames = here.saturating_sub(depth_room.saturating_mul(self.min_frame.max(native::MIN_FRAME)));
        Some(if frames > floor { (frames as u64, true) } else { (floor as u64, false) })
    }

    /// Runs entry form `fi` on the top frame: `false` when the frame or the
    /// globals aren't what it was compiled for (nothing changed then).
    fn run(&mut self, view: &mut JitView<'_>, form: usize) -> Result<Vec<(u32, Vec<u64>)>, ()> {
        let f = &self.forms[form];
        let frame = view.frames.last().ok_or(())?;
        if view.stack.len() != frame.stack_base || frame.locals.len() < f.n_locals {
            return Err(());
        }
        self.locals.clear();
        self.locals.resize(f.n_locals.max(1), 0);
        for &(s, k) in &f.guard {
            let v = &frame.locals[s as usize];
            if K::of(v) != k {
                return Err(());
            }
            self.locals[s as usize] = bits_of(v);
        }
        self.gbuf.resize(self.env.globals.len().max(1), 0);
        for &g in &f.globals {
            let (name, k) = self.env.globals[g];
            let v = view.globals.get(name as usize).and_then(Option::as_ref);
            if v.map_or(K::Null, K::of) != k {
                return Err(());
            }
            self.gbuf[g] = v.map_or(0, bits_of);
        }
        self.ctx.garrays.clear();
        self.ctx.garrays.resize(self.env.garrays.len(), None);
        for &h in &f.garrays {
            match view.globals.get(self.env.garrays[h].0 as usize) {
                Some(Some(Value::Array(a))) => self.ctx.garrays[h] = Some(a.clone()),
                _ => return Err(()),
            }
        }
        self.ctx.larrays.clear();
        for &s in &f.larrays {
            match &frame.locals[s as usize] {
                Value::Array(a) => self.ctx.larrays.push(a.clone()),
                _ => return Err(()),
            }
        }
        self.ctx.globals = self.gbuf.as_mut_ptr();
        self.ctx.interrupt = view.interrupt.map_or(&native::NOT_INTERRUPTED as *const u8, native::interrupt_byte);
        self.ctx.exits.clear();
        self.ctx.turns = 0;
        let code = f.code;
        native::call_entry(code, &mut self.ctx, &mut self.locals);
        // (no reference to an array outlives the run)
        self.ctx.garrays.clear();
        self.ctx.larrays.clear();
        self.ctx.globals = std::ptr::null_mut();
        let exits = std::mem::take(&mut self.ctx.exits);
        // (the globals it used, back as values)
        for &g in &self.forms[form].globals {
            let (name, k) = self.env.globals[g];
            if !k.scalar() || k == K::Null {
                continue;
            }
            if view.globals.len() <= name as usize {
                view.globals.resize(view.module.strings.len().max(name as usize + 1), None);
            }
            view.globals[name as usize] = Some(value_of(k, self.gbuf[g]));
        }
        Ok(exits)
    }

    /// Whether the exits compiled code left describe frames the VM can
    /// take: each record known, as many values as it names, every frame
    /// above the entry frame called from the one below it, slots inside
    /// their function. Checked before anything is written back.
    fn exits_sound(&self, view: &JitView<'_>, exits: &[(u32, Vec<u64>)]) -> Result<(), String> {
        let (r0, _) = exits.last().ok_or("no exit")?;
        let top = view.frames.last().ok_or("no frame")?;
        let mut caller: Option<&ExitRecord> = None;
        for (i, (r, vs)) in exits.iter().enumerate().rev() {
            let rec = self.records.get(*r as usize).ok_or_else(|| format!("exit record {r} unknown"))?;
            let func = view.module.functions.get(rec.fn_index as usize).ok_or("a routine unknown")?;
            let named = rec.locals.iter().map(|l| l.1.words()).sum::<usize>() + rec.stack.iter().map(|k| k.words()).sum::<usize>();
            if named != vs.len() {
                return Err(format!("exit record {r}: {} values for {named}", vs.len()));
            }
            // (a local whose kind travels with it: one compiled code holds)
            let mut at = 0;
            for &(_, k) in &rec.locals {
                if k == K::Dyn && !K::from_code(vs[at + 1] as u32).scalar() {
                    return Err(format!("exit record {r}: a local of kind {}", vs[at + 1]));
                }
                at += k.words();
            }
            // (each global array on its stack named, and holding one)
            let arrays = rec.stack.iter().filter(|&&k| k == K::Arr).count();
            if arrays != rec.arrays.len() || rec.arrays.iter().any(|&(d, g)| rec.stack.get(d as usize) != Some(&K::Arr) || !matches!(view.globals.get(g as usize), Some(Some(Value::Array(_))))) {
                return Err(format!("exit record {r}: a global array on the stack it doesn't name"));
            }
            let room = if i == exits.len() - 1 { top.locals.len() } else { func.n_locals as usize };
            if rec.locals.iter().any(|l| l.0 as usize >= room) || rec.ip as usize > func.code.len() {
                return Err(format!("exit record {r}: outside its routine"));
            }
            if i == exits.len() - 1 && (rec.fn_index != top.fn_index || *r0 != *r) {
                return Err("the entry frame's exit is another routine's".into());
            }
            if let Some(c) = caller {
                if !matches!(c.why, Why::AfterCall { .. }) {
                    return Err("a frame above one that made no call".into());
                }
            }
            caller = Some(rec);
        }
        Ok(())
    }

    /// Puts back the frames compiled code left (innermost first in
    /// `exits`; the last one is the entry frame's).
    fn apply(&self, view: &mut JitView<'_>, exits: &[(u32, Vec<u64>)]) -> Option<Why> {
        let (r0, v0) = exits.last()?;
        let rec0 = &self.records[*r0 as usize];
        let top = view.frames.last_mut()?;
        let mut vals = v0.iter();
        for &(s, k) in &rec0.locals {
            let v = local_value(k, &mut vals)?;
            if let Some(slot) = top.locals.get_mut(s as usize) {
                *slot = v;
            }
        }
        top.ip = rec0.ip as usize;
        for (d, &k) in rec0.stack.iter().enumerate() {
            let v = stack_value(rec0, d, k, view.globals, &mut vals)?;
            view.stack.push(v);
        }
        // (the callees that went back to the interpreter: real frames now,
        // each above its caller)
        let mut caller = rec0;
        for (r, vs) in exits[..exits.len() - 1].iter().rev() {
            let rec = &self.records[*r as usize];
            let Why::AfterCall { wants_value } = caller.why else { return None };
            let func = view.module.functions.get(rec.fn_index as usize)?;
            let mut locals = vec![Value::Null; func.n_locals as usize];
            let mut vals = vs.iter();
            for &(s, k) in &rec.locals {
                let v = local_value(k, &mut vals)?;
                *locals.get_mut(s as usize)? = v;
            }
            let stack_base = view.stack.len();
            view.frames.push(Frame {
                fn_index: rec.fn_index,
                locals,
                ret_ip: caller.ip as usize,
                wants_value,
                ip: rec.ip as usize,
                gosub: Vec::new(),
                stop: false,
                waiting: false,
                then: Vec::new(),
                nested: false,
                stack_base,
            });
            for (d, &k) in rec.stack.iter().enumerate() {
                let v = stack_value(rec, d, k, view.globals, &mut vals)?;
                view.stack.push(v);
            }
            caller = rec;
        }
        // (why the innermost frame left)
        Some(exits.first().map_or(rec0.why, |(r, _)| self.records[*r as usize].why))
    }
}

impl Inner {
    /// Runs form `form` and puts the program back; `true` when it ran.
    fn enter(&mut self, view: &mut JitView<'_>, form: usize) -> bool {
        let Some(st) = self.state.as_mut() else { return false };
        if st.forms[form].disabled {
            return false;
        }
        // (too little machine stack left here for compiled frames: the
        // interpreter goes on — not the form's fault, no deopt counted)
        let Some((limit, by_frames)) = st.stack_limit(&mut self.stats, view.depth_room) else {
            self.stats.low_stack += 1;
            return false;
        };
        st.ctx.stack_limit = limit;
        let exits = match st.run(view, form) {
            Ok(e) => e,
            Err(()) => {
                let f = &mut st.forms[form];
                f.deopts += 1;
                f.disabled |= f.deopts > MAX_DEOPTS;
                return false;
            }
        };
        st.forms[form].runs += 1;
        // (compiled code ran: what it changed is changed. Exits that don't
        // describe the frames would be the JIT's own bug — the program
        // stops there with it, never goes on from frames half written)
        if let Err(bug) = st.exits_sound(view, &exits) {
            panic!("RapidR's JIT left the program in a state it can't describe ({bug}): please report it; RAPIDR_JIT=off runs it without the JIT");
        }
        let why = st.apply(view, &exits);
        // (where: the innermost frame's way back, for --verbose)
        if let Some(&(r, _)) = exits.first() {
            let r = r as usize;
            if st.exit_counts.len() <= r {
                st.exit_counts.resize(st.records.len().max(r + 1), 0);
            }
            st.exit_counts[r] += 1;
        }
        // (compiled code always leaves through an exit; one that names no
        // frame would be the JIT's own bug: the run is reported, and the
        // form never used again)
        let Some(why) = why else {
            st.forms[form].disabled = true;
            self.stats.notes.push(format!("an exit without a record in routine {}", st.forms[form].fi));
            return true;
        };
        if why == Why::Guard {
            let f = &mut st.forms[form];
            f.deopts += 1;
            f.disabled |= f.deopts > MAX_DEOPTS;
        }
        // (a way back that wasn't rare: most runs took it, having done
        // little — the interpreter alone is as fast)
        let turns = st.ctx.turns;
        let f = &mut st.forms[form];
        f.turns = f.turns.saturating_add(turns);
        if matches!(why, Why::Planned | Why::Conflict) {
            f.unclean += 1;
        }
        if f.runs >= RARE && !f.disabled && f.unclean * 2 > f.runs && f.turns < f.runs.saturating_mul(RARE_TURNS) && self.mode != Mode::Always {
            f.disabled = true;
            let (fi, region) = (f.fi, f.region);
            if self.stats.notes.len() < 32 {
                let name = st.names.get(fi as usize).map_or("?", String::as_str);
                let line = view.module.functions.get(fi as usize).and_then(|f| f.get_line_for_ip(region.0)).unwrap_or(0);
                let who = if region.0 == 0 { shown_name(name).to_string() } else { format!("{}'s loop", shown_name(name)) };
                self.stats.notes.push(format!("{who} ({}): compiled code used no more, most of its runs went back to the interpreter", place(&st.map, line)));
            }
        }
        // (a deep call stopped by the machine stack's room, or by the VM's
        // room for frames: "deep calls")
        self.stats.runs += 1;
        if why == Why::Depth && !by_frames {
            *self.stats.exits.entry("the machine stack's room").or_default() += 1;
        } else {
            self.count_exit(why);
        }
        true
    }

    fn try_compile(&mut self, view: &JitView<'_>, fi: u32, ip: usize, start: bool) -> Option<usize> {
        let strict = self.mode != Mode::Always;
        self.state(view.module)?;
        let mut stats = std::mem::take(&mut self.stats);
        let r = self.state.as_mut().expect("set up").compile(view, fi, ip, start, strict, &mut stats);
        let r = match r {
            Ok(form) => Some(form),
            Err(e) => {
                stats.rejected += 1;
                let st = self.state.as_mut().expect("set up");
                let refused_at = st.refused_at.take();
                if stats.refused.len() < MAX_NOTES {
                    let m = view.module;
                    let f = m.functions.get(fi as usize);
                    let name = shown_name(f.map_or("?", |f| f.name.as_str()));
                    let line = |at: usize| f.and_then(|f| f.get_line_for_ip(at)).unwrap_or(0);
                    let who = if start { name.to_string() } else { format!("{name}'s loop") };
                    let at = refused_at.map(|(at, why)| (place(&m.source_map, line(at)), line(at), why, f.map_or(String::new(), |f| compile::what(m, f, at))));
                    stats.refused.push(Refused { who, place: place(&m.source_map, line(ip)), at, why: e });
                }
                None
            }
        };
        self.stats = stats;
        r
    }
}

impl rapidr_vm::Jit for Inner {
    fn loop_head(&mut self, mut view: JitView<'_>, ip: usize, turns: u32) -> JitAnswer {
        let (loop_hot, _) = self.hot();
        let Some(fi) = view.frames.last().map(|f| f.fn_index) else { return JitAnswer::NOT_YET };
        let Some(st) = self.state(view.module) else { return JitAnswer::NOT_YET };
        let start = *st.loops.get(&(fi, ip)).unwrap_or(&Start::Counting(0));
        let form = match start {
            Start::Compiled(form) => form,
            Start::Rejected => return JitAnswer { ran: false, again_after: 4096 },
            Start::Counting(n) => {
                let n = n.saturating_add(turns);
                if n < loop_hot {
                    st.loops.insert((fi, ip), Start::Counting(n));
                    return JitAnswer { ran: false, again_after: (loop_hot - n).min(1000) };
                }
                match self.try_compile(&view, fi, ip, false) {
                    Some(form) => {
                        self.state.as_mut().unwrap().loops.insert((fi, ip), Start::Compiled(form));
                        form
                    }
                    None => {
                        if let Some(st) = self.state.as_mut() {
                            st.loops.insert((fi, ip), Start::Rejected);
                        }
                        return JitAnswer { ran: false, again_after: 4096 };
                    }
                }
            }
        };
        let ran = self.enter(&mut view, form);
        let Some(st) = self.state.as_mut() else { return JitAnswer::NOT_YET };
        let disabled = st.forms[form].disabled;
        if disabled {
            st.loops.insert((fi, ip), Start::Rejected);
        }
        // (entered again and again: the loop around it is hot)
        let f = &mut st.forms[form];
        if ran && !f.promoted && f.runs >= PROMOTE {
            f.promoted = true;
            let region = f.region;
            if let Some(head) = view.module.functions.get(fi as usize).and_then(|func| enclosing_loop(&func.code, region)) {
                let e = st.loops.entry((fi, head)).or_insert(Start::Counting(0));
                if let Start::Counting(n) = e {
                    *n = (*n).max(loop_hot);
                }
            }
        }
        JitAnswer { ran, again_after: if disabled { 4096 } else { 1 } }
    }

    fn called(&mut self, mut view: JitView<'_>, calls: u32) -> JitAnswer {
        let (_, call_hot) = self.hot();
        let Some(fi) = view.frames.last().map(|f| f.fn_index) else { return JitAnswer::NOT_YET };
        let Some(st) = self.state(view.module) else { return JitAnswer::NOT_YET };
        let start = *st.starts.get(&fi).unwrap_or(&Start::Counting(0));
        let form = match start {
            Start::Compiled(form) => form,
            Start::Rejected => return JitAnswer::NOT_YET,
            Start::Counting(n) => {
                let n = n.saturating_add(calls);
                if n < call_hot {
                    st.starts.insert(fi, Start::Counting(n));
                    return JitAnswer { ran: false, again_after: call_hot - n };
                }
                match self.try_compile(&view, fi, 0, true) {
                    Some(form) => {
                        self.state.as_mut().unwrap().starts.insert(fi, Start::Compiled(form));
                        form
                    }
                    None => {
                        if let Some(st) = self.state.as_mut() {
                            st.starts.insert(fi, Start::Rejected);
                        }
                        return JitAnswer::NOT_YET;
                    }
                }
            }
        };
        let ran = self.enter(&mut view, form);
        let disabled = self.state.as_ref().is_some_and(|s| s.forms[form].disabled);
        if disabled {
            if let Some(st) = self.state.as_mut() {
                st.starts.insert(fi, Start::Rejected);
            }
            return JitAnswer { ran, again_after: u32::MAX };
        }
        JitAnswer { ran, again_after: 1 }
    }

    fn first_ask(&self) -> (u32, u32) {
        match self.mode {
            Mode::Always => (1, 1),
            _ => (1000, 100),
        }
    }

    fn report(&self) -> String {
        self.report_with(&|_| None)
    }

    fn report_with(&self, text: &dyn Fn(u32) -> Option<String>) -> String {
        if let Some(off) = &self.off {
            return format!("JIT: off for this program ({off})");
        }
        let s = &self.stats;
        let mut out = format!(
            "JIT: {} loop(s) and {} routine(s) compiled ({} called from compiled code), {} KB of code in {:.1} ms ({:.1} making machine code); ran {} time(s)",
            s.loops,
            s.routines,
            s.callees,
            s.code_bytes.div_ceil(1024),
            s.compile_time.as_secs_f64() * 1000.0,
            s.codegen_time.as_secs_f64() * 1000.0,
            s.runs
        );
        if s.rejected > 0 {
            out.push_str(&format!("; {} place(s) left interpreted", s.rejected));
        }
        if let Some(free) = s.stack_free {
            out.push_str(&format!("; machine stack {} KB free, frames up to {} KB", free / 1024, s.max_frame.div_ceil(1024)));
        }
        if s.low_stack > 0 {
            out.push_str(&format!("; {} entry(ies) interpreted: too little machine stack left", s.low_stack));
        }
        let mut exits: Vec<_> = s.exits.iter().collect();
        exits.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        if !exits.is_empty() {
            let parts: Vec<String> = exits.iter().map(|(w, n)| format!("{n} at {w}")).collect();
            out.push_str(&format!("; back to the interpreter: {}", parts.join(", ")));
        }
        // (the statement at a compiled line, when the runtime has the
        // program's text)
        let said = |line: u32| text(line).map(|t| statement(&t)).filter(|t| !t.is_empty()).map(|t| format!(": {t}")).unwrap_or_default();
        for r in &s.refused {
            match &r.at {
                Some((place, line, why, what)) => {
                    let what = if what.is_empty() { String::new() } else { format!(" ({what})") };
                    out.push_str(&format!("\n  {} ({}) stays interpreted: {}{what} at {place}{}", r.who, r.place, words(*why), said(*line)));
                }
                None => out.push_str(&format!("\n  {} ({}) stays interpreted ({})", r.who, r.place, r.why)),
            }
        }
        // (where compiled code went back most — not its ordinary ends, a
        // return or a loop's end)
        if let Some(st) = &self.state {
            // (one place however many forms left there: a routine compiled
            // as an entry and as a callee has a record each)
            let mut places: HashMap<(u32, u32, &'static str, &str), (u64, usize)> = HashMap::new();
            for (r, &n) in st.exit_counts.iter().enumerate() {
                let Some(rec) = st.records.get(r).filter(|rec| n > 0 && !matches!(rec.why, Why::Return | Why::LoopExit)) else { continue };
                let e = places.entry((rec.fn_index, rec.line, words(rec.why), rec.what.as_str())).or_insert((0, r));
                e.0 += n;
            }
            let mut top: Vec<(u64, usize)> = places.into_values().collect();
            top.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
            if !top.is_empty() {
                out.push_str("\n  back to the interpreter most at:");
            }
            for &(n, r) in top.iter().take(TOP_EXITS) {
                let rec = &st.records[r];
                let who = shown_name(st.names.get(rec.fn_index as usize).map_or("?", String::as_str));
                let what = if rec.what.is_empty() { String::new() } else { format!(" ({})", rec.what) };
                out.push_str(&format!("\n    {} in {who}: {n} time(s), {}{what}{}", place(&st.map, rec.line), words(rec.why), said(rec.line)));
            }
        }
        for n in &s.notes {
            out.push_str(&format!("\n  {n}"));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The code generator's settings the security fixes rest on: stack
    /// probes, inline, in every build (SEC-50); the IR verifier in debug
    /// builds and `always` mode, the tests' way (SEC-52).
    #[test]
    fn the_code_generators_safety_settings() {
        let m = Module::new();
        for mode in [Mode::On, Mode::Always] {
            let mut st = new_state(&m, mode).expect("the JIT's state");
            let jm = st.jm.take().expect("its module");
            let flags = jm.isa().flags().clone();
            native::free(jm);
            assert!(flags.enable_probestack(), "probestack");
            assert_eq!(flags.probestack_strategy(), settings::ProbestackStrategy::Inline);
            assert_eq!(flags.probestack_size_log2(), 12);
            let verify_expected = cfg!(debug_assertions) || mode == Mode::Always || verify_asked();
            assert_eq!(flags.enable_verifier(), verify_expected, "{mode:?}");
        }
    }
}
