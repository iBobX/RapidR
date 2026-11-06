//! The JIT the VM asks (`rapidr_vm::Jit`): when a loop or a routine is
//! hot it compiles it (with every routine it calls), and where compiled
//! code exists it runs it — reading the frame's values into the buffers
//! compiled code works on, and afterwards writing back what it left, as
//! the interpreter would have it at the instruction compiled code stopped
//! before (docs/jit-design.md §3, §4).

use std::collections::{BTreeSet, HashMap};
use std::time::{Duration, Instant};

use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{default_libcall_names, FuncId, Linkage, Module as _};

use rapidr_bytecode::disasm::decode;
use rapidr_bytecode::{Module, Operand};
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
}

/// Runs of an inner loop's form from the interpreter before the loop
/// around it is compiled too (whose turns are few — its inner loop's
/// were counted —: a synthesizer's voice loop around its samples' loop).
const PROMOTE: u64 = 16;

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
    runs: u64,
    exits: HashMap<&'static str, u64>,
    notes: Vec<String>,
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
}

pub struct Engine {
    mode: Mode,
    state: Option<Compiled>,
    /// The JIT can't run (no memory for code, a module it couldn't set
    /// up): the interpreter runs everything.
    off: Option<String>,
    stats: Stats,
}

impl Engine {
    pub fn new(mode: Mode) -> Engine {
        Engine { mode, state: None, off: None, stats: Stats::default() }
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
        let name = match why {
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
        };
        *self.stats.exits.entry(name).or_default() += 1;
    }
}

impl Drop for Engine {
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
        ctx: Ctx { globals: std::ptr::null_mut(), interrupt: &native::NOT_INTERRUPTED, stack_limit: u64::MAX, exits: Vec::new(), garrays: Vec::new(), larrays: Vec::new() },
        locals: Vec::new(),
        max_frame: 0,
        min_frame: usize::MAX,
        broken: None,
    })
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
                if (strict && !cout.unclean.is_empty()) || cout.rets.len() > 1 {
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
        if let Some((at, why)) = out.unclean.first().filter(|_| strict) {
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
            if !fout.used_locals.is_subset(&hint) || (strict && !fout.unclean.is_empty()) {
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
        });
        stats.compile_time += t0.elapsed();
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
        jm.define_function(id, &mut ctx)?;
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
            let named = rec.locals.iter().filter(|l| l.1 != K::Null).count() + rec.stack.iter().filter(|&&k| k != K::Null).count();
            if named != vs.len() {
                return Err(format!("exit record {r}: {} values for {named}", vs.len()));
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
            let v = if k == K::Null { Value::Null } else { value_of(k, *vals.next()?) };
            if let Some(slot) = top.locals.get_mut(s as usize) {
                *slot = v;
            }
        }
        top.ip = rec0.ip as usize;
        for &k in &rec0.stack {
            view.stack.push(if k == K::Null { Value::Null } else { value_of(k, *vals.next()?) });
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
                let v = if k == K::Null { Value::Null } else { value_of(k, *vals.next()?) };
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
            for &k in &rec.stack {
                view.stack.push(if k == K::Null { Value::Null } else { value_of(k, *vals.next()?) });
            }
            caller = rec;
        }
        // (why the innermost frame left)
        Some(exits.first().map_or(rec0.why, |(r, _)| self.records[*r as usize].why))
    }
}

impl Engine {
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
                if stats.notes.len() < 32 {
                    let name = view.module.functions.get(fi as usize).map_or("?", |f| f.name.as_str());
                    stats.notes.push(format!("{name} at byte {ip}: stays interpreted ({e})"));
                }
                None
            }
        };
        self.stats = stats;
        r
    }
}

impl rapidr_vm::Jit for Engine {
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
        if let Some(off) = &self.off {
            return format!("JIT: off for this program ({off})");
        }
        let s = &self.stats;
        let mut text = format!(
            "JIT: {} loop(s) and {} routine(s) compiled ({} called from compiled code), {} KB of code in {:.1} ms; ran {} time(s)",
            s.loops,
            s.routines,
            s.callees,
            s.code_bytes.div_ceil(1024),
            s.compile_time.as_secs_f64() * 1000.0,
            s.runs
        );
        if s.rejected > 0 {
            text.push_str(&format!("; {} place(s) left interpreted", s.rejected));
        }
        if let Some(free) = s.stack_free {
            text.push_str(&format!("; machine stack {} KB free, frames up to {} KB", free / 1024, s.max_frame.div_ceil(1024)));
        }
        if s.low_stack > 0 {
            text.push_str(&format!("; {} entry(ies) interpreted: too little machine stack left", s.low_stack));
        }
        let mut exits: Vec<_> = s.exits.iter().collect();
        exits.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        if !exits.is_empty() {
            let parts: Vec<String> = exits.iter().map(|(w, n)| format!("{n} at {w}")).collect();
            text.push_str(&format!("; back to the interpreter: {}", parts.join(", ")));
        }
        for n in &s.notes {
            text.push_str(&format!("\n  {n}"));
        }
        text
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
