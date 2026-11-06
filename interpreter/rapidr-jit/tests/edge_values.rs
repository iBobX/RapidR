//! Phase 2's edge-value guard (docs/vm-fast-design.md), through compiled
//! code: every operation the JIT compiles, on the edge values (0, ±1, the
//! 32- and 64-bit bounds, ±2^53 ± 1, -0, NaN, ±∞, 1E300, halves,
//! booleans, Null, text), gives with the JIT (`always`) exactly what the
//! interpreter gives — the same kind, the same bits, the same branch, the
//! same error. Each case is a two-turn loop: the first turn interpreted,
//! the second compiled (the loop's head is compiled at its first turn).
//!
//! A debug build tries a smaller set of values; `cargo test --release -p
//! rapidr-jit` (tools/regress.sh) all of them.

use rapidr_bytecode::{Bin, Cmp, Const, Function, Mode, Module, Op, NO_KIND};
use rapidr_jit::kinds::{bits_of, K};
use rapidr_jit::Mode as Jit;
use rapidr_value::Value;
use rapidr_vm::{Host, StubHost, Vm};

#[derive(Default)]
struct MathHost(StubHost);

impl Host for MathHost {
    const DIRECT_MATHS: bool = true;
    fn call_builtin(&mut self, name: &str, args: &[Value]) -> Result<Value, String> {
        match rapidr_vm::direct_builtin(name) {
            Some(f) if args.len() == 1 => Ok(f(&args[0])),
            _ => self.0.call_builtin(name, args),
        }
    }
    fn create_comp(&mut self, k: &str, id: &str) -> Result<Value, String> {
        self.0.create_comp(k, id)
    }
    fn set_prop(&mut self, id: &str, n: &str, v: Value) -> Result<(), String> {
        self.0.set_prop(id, n, v)
    }
    fn get_prop(&mut self, id: &str, n: &str) -> Result<Value, String> {
        self.0.get_prop(id, n)
    }
    fn call_method(&mut self, id: &str, m: &str, a: &[Value]) -> Result<Value, String> {
        self.0.call_method(id, m, a)
    }
    fn register_event(&mut self, id: &str, e: &str, f: u32) -> Result<(), String> {
        self.0.register_event(id, e, f)
    }
    fn print(&mut self, s: &str) -> Result<(), String> {
        self.0.print(s)
    }
    fn input(&mut self) -> Result<String, String> {
        self.0.input()
    }
}

fn values() -> Vec<Value> {
    let full = vec![
        Value::Integer(0),
        Value::Integer(1),
        Value::Integer(-1),
        Value::Integer(7),
        Value::Integer(i32::MAX as i64),
        Value::Integer(i32::MIN as i64),
        Value::Integer(i64::MAX),
        Value::Integer(i64::MIN),
        Value::Integer((1 << 53) + 1),
        Value::Integer(-(1 << 53) - 1),
        Value::Double(0.0),
        Value::Double(-0.0),
        Value::Double(1.0),
        Value::Double(2.5),
        Value::Double(-2.5),
        Value::Double(3e9),
        Value::Double(f64::NAN),
        Value::Double(f64::INFINITY),
        Value::Double(f64::NEG_INFINITY),
        Value::Double(1e300),
        Value::Boolean(true),
        Value::Boolean(false),
        Value::Null,
        Value::String("abc".into()),
        // (RapidR's 64-bit kinds, docs/big-numbers-design.md)
        Value::Int64(0),
        Value::Int64(-1),
        Value::Int64(i64::MAX),
        Value::Int64(i64::MIN),
        Value::Int64((1 << 53) + 1),
        Value::UInt64(0),
        Value::UInt64(1),
        Value::UInt64(u64::MAX),
        Value::UInt64(1 << 63),
    ];
    if cfg!(debug_assertions) {
        // (the debug build's set: one of each kind of edge)
        vec![Value::Integer(0), Value::Integer(-1), Value::Integer(i64::MIN), Value::Integer((1 << 53) + 1), Value::Double(-0.0), Value::Double(2.5), Value::Double(f64::NAN), Value::Double(1e300), Value::Boolean(true), Value::Null, Value::Int64(i64::MIN), Value::Int64(-1), Value::UInt64(u64::MAX), Value::UInt64(1 << 63)]
    } else {
        full
    }
}

fn konst(v: &Value) -> Const {
    match v {
        Value::Integer(n) => Const::Int(*n),
        Value::Double(f) => Const::Double(*f),
        Value::Boolean(b) => Const::Bool(*b),
        Value::String(s) => Const::Str(s.to_string()),
        Value::Int64(n) => Const::Int64(*n),
        Value::UInt64(n) => Const::UInt64(*n),
        _ => Const::Null,
    }
}

/// A two-turn loop around `body` (code pushing nothing, storing its
/// answer into global `r`), after locals 0, 1 (and globals `ga`, `gb`)
/// are set to `a` and `b`; an array `arr` (0 TO 3, Doubles) in global
/// `arr` and local 5.
struct Case {
    m: Module,
    f: Function,
    pub ka: u32,
    pub kb: u32,
    pub ga: u32,
    pub gb: u32,
    pub r: u32,
    pub arr: u32,
}

impl Case {
    fn new(a: &Value, b: &Value) -> Case {
        let mut m = Module::new();
        let ka = m.add_const(konst(a));
        let kb = m.add_const(konst(b));
        let ga = m.add_string("ga");
        let gb = m.add_string("gb");
        let r = m.add_string("r");
        let arr = m.add_string("arr");
        let f = Function { name: "__main".into(), n_locals: 8, ..Default::default() };
        let mut c = Case { m, f, ka, kb, ga, gb, r, arr };
        c.load_k(ka).op(Op::StoreLocal).u16(0);
        c.load_k(kb).op(Op::StoreLocal).u16(1);
        c.load_k(ka).op(Op::StoreGlobal).u32(ga);
        c.load_k(kb).op(Op::StoreGlobal).u32(gb);
        // (the array: Null fill, 0 TO 3, then Doubles)
        c.op(Op::LoadNull);
        let zero = c.m.add_const(Const::Int(0));
        let three = c.m.add_const(Const::Int(3));
        c.load_k(zero).load_k(three).op(Op::NewArray).u8(1).op(Op::StoreGlobal).u32(arr);
        c.op(Op::LoadGlobal).u32(arr).op(Op::StoreLocal).u16(5);
        for i in 0..4 {
            let ki = c.m.add_const(Const::Int(i));
            let kv = c.m.add_const(Const::Double(i as f64 + 0.5));
            c.op(Op::LoadGlobal).u32(arr).load_k(ki).load_k(kv).op(Op::ASet).u8(1);
        }
        c.load_k(zero).op(Op::StoreLocal).u16(2);
        c
    }

    fn op(&mut self, op: Op) -> &mut Self {
        self.f.code.push(op as u8);
        self
    }
    fn u8(&mut self, v: u8) -> &mut Self {
        self.f.code.push(v);
        self
    }
    fn u16(&mut self, v: u16) -> &mut Self {
        self.f.code.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn u32(&mut self, v: u32) -> &mut Self {
        self.f.code.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn load_k(&mut self, k: u32) -> &mut Self {
        self.op(Op::LoadConst).u32(k)
    }
    fn here(&self) -> u32 {
        self.f.code.len() as u32
    }
    /// An operand of mode `mode` for `a` (`first`) or `b`.
    fn operand(&mut self, mode: Mode, first: bool) -> &mut Self {
        match mode {
            Mode::L => self.u16(if first { 0 } else { 1 }),
            Mode::G => {
                let g = if first { self.ga } else { self.gb };
                self.u32(g)
            }
            Mode::K => {
                let k = if first { self.ka } else { self.kb };
                self.u32(k)
            }
            Mode::S => self,
        }
    }
    fn store_r(&mut self) -> &mut Self {
        let r = self.r;
        self.op(Op::StoreGlobal).u32(r)
    }

    /// The loop around `body`; returns the module.
    fn finish(mut self, body: impl FnOnce(&mut Case)) -> Module {
        let head = self.here();
        body(&mut self);
        let one = self.m.add_const(Const::Int(1));
        let two = self.m.add_const(Const::Int(2));
        self.op(Op::AddLK).u16(2).u32(one).op(Op::StoreLocal).u16(2);
        let jump_at = self.here();
        self.op(Op::JumpIfNotLtLK).u16(2).u32(two).u32(0);
        self.op(Op::Jump).u32(head);
        let end = self.here();
        self.f.code[jump_at as usize + 7..jump_at as usize + 11].copy_from_slice(&end.to_le_bytes());
        self.op(Op::Halt);
        self.f.line_info = vec![(0, 1), (head, 2)];
        let Case { mut m, f, .. } = self;
        m.entry = m.add_function(f);
        m
    }
}

/// The global `r` (kind and bits) and the error, run one way.
fn outcome(m: &Module, jit: Jit) -> (Option<(K, u64)>, Option<String>, String) {
    let mut h = MathHost::default();
    let mut vm = Vm::new(&mut h);
    let _ = rapidr_jit::install_mode(&mut vm, jit);
    let err = vm.run(m).err().map(|e| e.to_string());
    let r = m.strings.iter().position(|s| s == "r").and_then(|i| vm.globals.get(i).cloned().flatten());
    let report = rapidr_jit::report(&vm).unwrap_or_default();
    (r.map(|v| (K::of(&v), bits_of(&v))), err, report)
}

/// Runs every value pair through `body` both ways; the number of runs in
/// which compiled code ran.
fn check(name: &str, body: impl Fn(&mut Case)) -> usize {
    let mut compiled = 0;
    for a in values() {
        for b in values() {
            let m = Case::new(&a, &b).finish(&body);
            let (want, want_err, _) = outcome(&m, Jit::Off);
            let (got, got_err, report) = outcome(&m, Jit::Always);
            assert_eq!(got, want, "{name}({a:?}, {b:?}): {report}");
            assert_eq!(got_err, want_err, "{name}({a:?}, {b:?}): {report}");
            if report.contains("ran ") && !report.contains("ran 0 time") {
                compiled += 1;
            }
        }
    }
    compiled
}

#[test]
fn fused_arithmetic() {
    let modes = [Mode::S, Mode::L, Mode::G, Mode::K];
    for bin in [Bin::Add, Bin::Sub, Bin::Mul, Bin::Div, Bin::Mod] {
        for ma in modes {
            for mb in modes {
                let Some(op) = Op::fused_arith(bin, ma, mb) else { continue };
                let ran = check(&format!("{op:?}"), |c| {
                    // (the S operand pushed first: the local's value)
                    if ma == Mode::S {
                        c.op(Op::LoadLocal).u16(0);
                    }
                    if mb == Mode::S {
                        c.op(Op::LoadLocal).u16(1);
                    }
                    c.op(op);
                    if ma != Mode::S {
                        c.operand(ma, true);
                    }
                    if mb != Mode::S {
                        c.operand(mb, false);
                    }
                    c.store_r();
                });
                assert!(ran > 0, "{op:?} never ran compiled");
            }
        }
    }
}

#[test]
fn plain_operations() {
    for op in [Op::Add, Op::Sub, Op::Mul, Op::Div, Op::IDiv, Op::Mod, Op::Pow, Op::Eq, Op::Ne, Op::Lt, Op::Le, Op::Gt, Op::Ge, Op::And, Op::Or, Op::Xor, Op::BAnd, Op::BOr, Op::BXor, Op::Shl, Op::Shr] {
        check(&format!("{op:?}"), |c| {
            c.op(Op::LoadLocal).u16(0).op(Op::LoadLocal).u16(1).op(op).store_r();
        });
    }
    for op in [Op::Neg, Op::Not, Op::BNot] {
        check(&format!("{op:?}"), |c| {
            c.op(Op::LoadLocal).u16(0).op(op).store_r();
        });
    }
}

#[test]
fn compare_and_branch() {
    let modes = [Mode::S, Mode::L, Mode::G, Mode::K];
    for cmp in [Cmp::Eq, Cmp::Ne, Cmp::Lt, Cmp::Le, Cmp::Gt, Cmp::Ge] {
        for ma in modes {
            for mb in modes {
                let Some(op) = Op::cmp_jump(cmp, ma, mb) else { continue };
                check(&format!("{op:?}"), |c| {
                    if ma == Mode::S {
                        c.op(Op::LoadLocal).u16(0);
                    }
                    if mb == Mode::S {
                        c.op(Op::LoadLocal).u16(1);
                    }
                    c.op(op);
                    if ma != Mode::S {
                        c.operand(ma, true);
                    }
                    if mb != Mode::S {
                        c.operand(mb, false);
                    }
                    let target_at = c.here();
                    c.u32(0);
                    // (not jumped: r = 1; jumped: r = 0)
                    let one = c.m.add_const(Const::Int(1));
                    let zero = c.m.add_const(Const::Int(0));
                    c.load_k(one).store_r();
                    let skip_at = c.here() + 1;
                    c.op(Op::Jump).u32(0);
                    let t = c.here();
                    c.f.code[target_at as usize..target_at as usize + 4].copy_from_slice(&t.to_le_bytes());
                    c.load_k(zero).store_r();
                    let end = c.here();
                    c.f.code[skip_at as usize..skip_at as usize + 4].copy_from_slice(&end.to_le_bytes());
                });
            }
        }
    }
    // (JumpIf / JumpIfNot on every value's truth)
    for op in [Op::JumpIf, Op::JumpIfNot] {
        check(&format!("{op:?}"), |c| {
            c.op(Op::LoadLocal).u16(0).op(op);
            let target_at = c.here();
            c.u32(0);
            let one = c.m.add_const(Const::Int(1));
            let zero = c.m.add_const(Const::Int(0));
            c.load_k(one).store_r();
            let skip_at = c.here() + 1;
            c.op(Op::Jump).u32(0);
            let t = c.here();
            c.f.code[target_at as usize..target_at as usize + 4].copy_from_slice(&t.to_le_bytes());
            c.load_k(zero).store_r();
            let end = c.here();
            c.f.code[skip_at as usize..skip_at as usize + 4].copy_from_slice(&end.to_le_bytes());
        });
    }
}

#[test]
fn conversions() {
    let kinds: Vec<u8> = (0..9).chain([rapidr_value::numeric::ARG_ROUND]).collect();
    for kind in kinds {
        check(&format!("ToNum {kind}"), |c| {
            c.op(Op::LoadLocal).u16(0).op(Op::ToNum).u8(kind).store_r();
        });
        check(&format!("StoreLocalNum {kind}"), |c| {
            c.op(Op::LoadLocal).u16(0).op(Op::StoreLocalNum).u16(3).u8(kind).op(Op::LoadLocal).u16(3).store_r();
        });
        let r = Case::new(&Value::Null, &Value::Null).r;
        check(&format!("StoreGlobalNum {kind}"), |c| {
            c.op(Op::LoadLocal).u16(0).op(Op::StoreGlobalNum).u32(r).u8(kind);
        });
        if kind < 9 {
            for op in [Op::ConvLocal, Op::ConvArgLocal] {
                check(&format!("{op:?} {kind}"), |c| {
                    c.op(Op::MoveLocal).u16(3).u16(0).op(op).u16(3).u8(kind).op(Op::LoadLocal).u16(3).store_r();
                });
            }
        }
    }
}

#[test]
fn maths_builtins() {
    for name in rapidr_vm::direct_names() {
        // (SHL and SHR take two: a and b)
        let argc = if rapidr_vm::direct_builtin2(name).is_some() { 2 } else { 1 };
        let ran = check(name, |c| {
            let s = c.m.add_string(&name.to_ascii_uppercase());
            c.op(Op::LoadLocal).u16(0);
            if argc == 2 {
                c.op(Op::LoadLocal).u16(1);
            }
            c.op(Op::CallBuiltin).u32(s).u8(argc).store_r();
        });
        assert!(ran > 0, "{name} never ran compiled");
    }
}

/// `IIF(c, a, b)` (docs/jit-design.md §12.8): every condition's truth,
/// arms of every pair of kinds — as they are, and stored or converted into
/// each numeric kind (where arms of two kinds compile: each converted, the
/// one picked) —: the arm the interpreter picks, of its kind and bits.
#[test]
fn iif() {
    let conds: Vec<Value> = if cfg!(debug_assertions) {
        vec![Value::Integer(0), Value::Double(f64::NAN), Value::Boolean(true), Value::Null, Value::String("x".into())]
    } else {
        vec![Value::Integer(0), Value::Integer(-1), Value::Double(-0.0), Value::Double(0.5), Value::Double(f64::NAN), Value::Boolean(true), Value::Boolean(false), Value::Null, Value::String("x".into()), Value::String(String::new().into()), Value::Int64(1), Value::UInt64(0)]
    };
    for cond in &conds {
        let iif = |c: &mut Case| {
            let kc = c.m.add_const(konst(cond));
            let s = c.m.add_string("IIF");
            c.load_k(kc).op(Op::StoreLocal).u16(3);
            c.op(Op::LoadLocal).u16(3).op(Op::LoadLocal).u16(0).op(Op::LoadLocal).u16(1).op(Op::CallBuiltin).u32(s).u8(3);
        };
        let ran = check(&format!("IIF({cond:?}, a, b)"), |c| {
            iif(c);
            c.store_r();
        });
        assert!(ran > 0, "IIF({cond:?}, a, b) never ran compiled");
        let kinds: Vec<u8> = if cfg!(debug_assertions) { vec![3, 5, 8] } else { (0..10).chain([rapidr_value::numeric::ARG_ROUND]).collect() };
        for kind in kinds {
            let r = Case::new(&Value::Null, &Value::Null).r;
            check(&format!("IIF({cond:?}, a, b) StoreGlobalNum {kind}"), |c| {
                iif(c);
                c.op(Op::StoreGlobalNum).u32(r).u8(kind);
            });
            check(&format!("IIF({cond:?}, a, b) StoreLocalNum {kind}"), |c| {
                iif(c);
                c.op(Op::StoreLocalNum).u16(4).u8(kind).op(Op::LoadLocal).u16(4).store_r();
            });
            check(&format!("IIF({cond:?}, a, b) ToNum {kind}"), |c| {
                iif(c);
                c.op(Op::ToNum).u8(kind).store_r();
            });
        }
    }
}

#[test]
fn for_loops() {
    // (counter a, end b, steps 1, -1, 0.5, 0, NaN: the test's branch, the
    // step's sum)
    for step in [Value::Integer(1), Value::Integer(-1), Value::Double(0.5), Value::Integer(0), Value::Double(f64::NAN)] {
        check(&format!("ForTestL step {step:?}"), |c| {
            let ks = c.m.add_const(konst(&step));
            c.load_k(ks).op(Op::StoreLocal).u16(4);
            c.op(Op::ForTestL).u16(0).u16(1).u16(4);
            let target_at = c.here();
            c.u32(0);
            let one = c.m.add_const(Const::Int(1));
            let zero = c.m.add_const(Const::Int(0));
            c.load_k(one).store_r();
            let skip_at = c.here() + 1;
            c.op(Op::Jump).u32(0);
            let t = c.here();
            c.f.code[target_at as usize..target_at as usize + 4].copy_from_slice(&t.to_le_bytes());
            c.load_k(zero).store_r();
            let end = c.here();
            c.f.code[skip_at as usize..skip_at as usize + 4].copy_from_slice(&end.to_le_bytes());
        });
    }
}

#[test]
fn one_index_arrays() {
    // (index a, value b: in range, out of it, of every kind)
    check("AGetGL", |c| {
        let arr = c.arr;
        c.op(Op::AGetGL).u32(arr).u16(0).store_r();
    });
    check("AGetLS", |c| {
        c.op(Op::LoadLocal).u16(0).op(Op::AGetLS).u16(5).store_r();
    });
    // (the global array read first, its index after — `arr(i + IIF(…))` —
    // and written so: the plain AGet / ASet of one index)
    check("LoadGlobal arr; AGet 1", |c| {
        let arr = c.arr;
        c.op(Op::LoadGlobal).u32(arr).op(Op::LoadLocal).u16(0).op(Op::AGet).u8(1).store_r();
    });
    check("LoadGlobal arr; ASet 1", |c| {
        let arr = c.arr;
        c.op(Op::LoadGlobal).u32(arr).op(Op::LoadLocal).u16(0).op(Op::LoadLocal).u16(1).op(Op::ASet).u8(1);
        c.op(Op::LoadGlobal).u32(arr).op(Op::LoadLocal).u16(0).op(Op::AGet).u8(1).store_r();
    });
    for kind in [NO_KIND, 5, 3] {
        check(&format!("ASetGL {kind}"), |c| {
            let arr = c.arr;
            let k0 = c.m.add_const(Const::Int(0));
            c.op(Op::LoadLocal).u16(1).op(Op::ASetGL).u32(arr).u16(0).u8(kind);
            c.op(Op::AGetGL).u32(arr).u16(0).store_r();
            let _ = k0;
        });
        check(&format!("ASetLS {kind}"), |c| {
            c.op(Op::LoadLocal).u16(1).op(Op::LoadLocal).u16(0).op(Op::ASetLS).u16(5).u8(kind);
            c.op(Op::LoadLocal).u16(0).op(Op::AGetLS).u16(5).store_r();
        });
    }
}

/// How often each kind of case ran compiled code (`--nocapture`).
#[test]
#[ignore]
fn show_coverage() {
    let n = values().len() * values().len();
    let add = check("AddLL", |c| {
        c.op(Op::AddLL).u16(0).u16(1).store_r();
    });
    let cmp = check("Lt", |c| {
        c.op(Op::LoadLocal).u16(0).op(Op::LoadLocal).u16(1).op(Op::Lt).store_r();
    });
    let m = Case::new(&Value::Integer(1), &Value::Double(2.5)).finish(|c| {
        c.op(Op::AddLL).u16(0).u16(1).store_r();
    });
    println!("{}", rapidr_bytecode::disasm::module(&m));
    println!("{}", outcome(&m, Jit::Always).2);
    println!("AddLL {add}/{n}, Lt {cmp}/{n}");
}
