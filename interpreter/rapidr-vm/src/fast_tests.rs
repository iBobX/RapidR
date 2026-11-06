//! The guard of docs/vm-fast-design.md: every fused instruction gives what
//! its expansion gives — the same kind of value, the same bits, the same
//! branch, the same error — on edge values (0, ±1, the 32- and 64-bit
//! bounds, ±2^53 ± 1, -0, NaN, ±∞, 1E300, halves, booleans, text, Null),
//! and the operations the instructions share give what `Value`'s own
//! operators give.

use rapidr_bytecode::{Bin, Cmp, Const, Function, Mode, Module, Op, NO_KIND};
use rapidr_value::numeric::{self, NumKind};
use rapidr_value::Value;

use crate::fast;
use crate::{StubHost, Vm};

fn edge_values() -> Vec<Value> {
    let mut v = vec![
        Value::Integer(0),
        Value::Integer(1),
        Value::Integer(-1),
        Value::Integer(2),
        Value::Integer(7),
        Value::Integer(-7),
        Value::Integer(300),
        Value::Integer(i32::MAX as i64),
        Value::Integer(i32::MIN as i64),
        Value::Integer(i32::MAX as i64 + 1),
        Value::Integer(i64::MAX),
        Value::Integer(i64::MIN),
        Value::Integer(1 << 53),
        Value::Integer((1 << 53) + 1),
        Value::Integer(-(1 << 53) - 1),
        // (RapidR's 64-bit kinds: the generic path, the same answers)
        Value::Int64(0),
        Value::Int64(-1),
        Value::Int64(i64::MAX),
        Value::Int64(i64::MIN),
        Value::Int64((1 << 53) + 1),
        Value::UInt64(0),
        Value::UInt64(u64::MAX),
        Value::UInt64(1 << 63),
        Value::UInt64(7),
    ];
    for f in [0.0, -0.0, 0.5, 2.5, -2.5, 2.7, -2.7, 3.0, 1e10, 1e300, -1e300, f64::INFINITY, f64::NEG_INFINITY, f64::NAN, 9007199254740992.0, 4294967295.0] {
        v.push(Value::Double(f));
    }
    v.extend([Value::Boolean(true), Value::Boolean(false), Value::String("12".into()), Value::String("abc".into()), Value::String(String::new().into()), Value::Null]);
    v
}

fn konst(v: &Value) -> Const {
    match v {
        Value::Integer(n) => Const::Int(*n),
        Value::Double(f) => Const::Double(*f),
        Value::Boolean(b) => Const::Bool(*b),
        Value::String(s) => Const::Str(s.as_str().to_string()),
        Value::Null => Const::Null,
        Value::Int64(n) => Const::Int64(*n),
        Value::UInt64(n) => Const::UInt64(*n),
        _ => unreachable!("no constant"),
    }
}

/// A value's kind and bits, to compare two runs exactly.
fn exact(v: &Value) -> String {
    match v {
        Value::Integer(n) => format!("I{n}"),
        Value::Int64(n) => format!("W{n}"),
        Value::UInt64(n) => format!("U{n}"),
        Value::Double(f) => format!("D{:016x}", f.to_bits()),
        Value::Boolean(b) => format!("B{b}"),
        Value::String(s) => format!("S{:?}", s.as_str()),
        Value::Null => "N".into(),
        Value::Array(a) => {
            let a = a.borrow();
            format!("A{:?}[{}]{{{}}}", a.bounds, a.data.iter().map(exact).collect::<Vec<_>>().join(","), a.overflow.iter().map(|(k, v)| format!("{k}:{}", exact(v))).collect::<Vec<_>>().join(","))
        }
        Value::Object(o) => format!("O{}", o.id),
    }
}

/// A program being put together: code, its constants and globals.
struct Asm {
    m: Module,
    code: Vec<u8>,
}

impl Asm {
    fn new() -> Self {
        let mut m = Module::new();
        // (the globals the programs use, in a known order)
        for g in ["ga", "gb", "result", "path", "arr"] {
            m.add_string(g);
        }
        Asm { m, code: Vec::new() }
    }
    fn g(&self, name: &str) -> u32 {
        self.m.strings.iter().position(|s| s == name).unwrap() as u32
    }
    fn op(&mut self, op: Op) -> &mut Self {
        self.code.push(op as u8);
        self
    }
    fn u8(&mut self, v: u8) -> &mut Self {
        self.code.push(v);
        self
    }
    fn u16(&mut self, v: u16) -> &mut Self {
        self.code.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn u32(&mut self, v: u32) -> &mut Self {
        self.code.extend_from_slice(&v.to_le_bytes());
        self
    }
    /// A jump target to fill in later: its place.
    fn hole(&mut self) -> usize {
        self.code.extend_from_slice(&[0; 4]);
        self.code.len() - 4
    }
    fn fill(&mut self, hole: usize, target: usize) {
        self.code[hole..hole + 4].copy_from_slice(&(target as u32).to_le_bytes());
    }
    fn push(&mut self, v: &Value) -> &mut Self {
        let k = self.m.add_const(konst(v));
        self.op(Op::LoadConst).u32(k)
    }
    fn k(&mut self, v: &Value) -> u32 {
        self.m.add_const(konst(v))
    }
    fn store_global(&mut self, name: &str) -> &mut Self {
        let g = self.g(name);
        self.op(Op::StoreGlobal).u32(g)
    }
    /// Locals 0 and 1, globals ga and gb hold `a` and `b`.
    fn setup(&mut self, a: &Value, b: &Value) -> &mut Self {
        self.push(a).op(Op::StoreLocal).u16(0);
        self.push(b).op(Op::StoreLocal).u16(1);
        self.push(a).store_global("ga");
        self.push(b).store_global("gb")
    }
    /// Runs the program (4 locals); what it left in its globals, or its
    /// error.
    fn run(mut self) -> String {
        self.op(Op::Halt);
        let f = Function { name: "__main".into(), n_locals: 4, code: std::mem::take(&mut self.code), ..Default::default() };
        self.m.entry = self.m.add_function(f);
        let mut host = StubHost::default();
        let mut vm = Vm::new(&mut host);
        match vm.run(&self.m) {
            Ok(()) => vm.global_values(&self.m).map(|(n, v)| format!("{n}={}", exact(v))).collect::<Vec<_>>().join(" "),
            Err(e) => format!("error: {e}"),
        }
    }
}

/// Pushes operand `mode` (for a: local 0 / ga / `v`; b: local 1 / gb).
fn load(asm: &mut Asm, mode: Mode, v: &Value, first: bool) {
    match mode {
        Mode::S | Mode::K => {
            asm.push(v);
        }
        Mode::L => {
            asm.op(Op::LoadLocal).u16(if first { 0 } else { 1 });
        }
        Mode::G => {
            let g = asm.g(if first { "ga" } else { "gb" });
            asm.op(Op::LoadGlobal).u32(g);
        }
    }
}

/// Operand `mode`'s bytes in a fused instruction.
fn operand(asm: &mut Asm, mode: Mode, v: &Value, first: bool) {
    match mode {
        Mode::S => {}
        Mode::L => {
            asm.u16(if first { 0 } else { 1 });
        }
        Mode::G => {
            let g = asm.g(if first { "ga" } else { "gb" });
            asm.u32(g);
        }
        Mode::K => {
            let k = asm.k(v);
            asm.u32(k);
        }
    }
}

const MODES: [Mode; 4] = [Mode::S, Mode::L, Mode::G, Mode::K];

#[test]
fn fused_arithmetic_is_its_expansion() {
    let values = edge_values();
    let mut checked = 0;
    for bin in [Bin::Add, Bin::Sub, Bin::Mul, Bin::Div, Bin::Mod] {
        for a in MODES {
            for b in MODES {
                let Some(fused) = Op::fused_arith(bin, a, b) else { continue };
                for x in &values {
                    for y in &values {
                        let mut want = Asm::new();
                        want.setup(x, y);
                        load(&mut want, a, x, true);
                        load(&mut want, b, y, false);
                        want.op(bin.op()).store_global("result");
                        let mut got = Asm::new();
                        got.setup(x, y);
                        // (the stack operand first — b's when a is a leaf)
                        if a == Mode::S {
                            got.push(x);
                        }
                        if b == Mode::S {
                            got.push(y);
                        }
                        got.op(fused);
                        operand(&mut got, a, x, true);
                        operand(&mut got, b, y, false);
                        got.store_global("result");
                        assert_eq!(got.run(), want.run(), "{fused:?} on {x:?}, {y:?}");
                        checked += 1;
                    }
                }
            }
        }
    }
    assert!(checked > 60_000, "{checked}");
}

#[test]
fn compare_and_branch_is_its_expansion() {
    let values = edge_values();
    for cmp in [Cmp::Eq, Cmp::Ne, Cmp::Lt, Cmp::Le, Cmp::Gt, Cmp::Ge] {
        for a in MODES {
            for b in MODES {
                let Some(fused) = Op::cmp_jump(cmp, a, b) else { continue };
                for x in &values {
                    for y in &values {
                        let tail = |asm: &mut Asm, hole: usize| {
                            asm.push(&Value::String("then".into())).store_global("path").op(Op::Halt);
                            let at = asm.code.len();
                            asm.fill(hole, at);
                            asm.push(&Value::String("else".into())).store_global("path");
                        };
                        let mut want = Asm::new();
                        want.setup(x, y);
                        load(&mut want, a, x, true);
                        load(&mut want, b, y, false);
                        want.op(cmp.op()).op(Op::JumpIfNot);
                        let hole = want.hole();
                        tail(&mut want, hole);
                        let mut got = Asm::new();
                        got.setup(x, y);
                        if a == Mode::S {
                            got.push(x);
                        }
                        if b == Mode::S {
                            got.push(y);
                        }
                        got.op(fused);
                        operand(&mut got, a, x, true);
                        operand(&mut got, b, y, false);
                        let hole = got.hole();
                        tail(&mut got, hole);
                        assert_eq!(got.run(), want.run(), "{fused:?} on {x:?}, {y:?}");
                    }
                }
            }
        }
    }
}

/// Every conversion operand a store takes: the kinds and ARG_ROUND.
fn kinds() -> Vec<u8> {
    let mut k: Vec<u8> = (0..9).map(|c| NumKind::from_code(c).unwrap().code()).collect();
    k.push(numeric::ARG_ROUND);
    k
}

#[test]
fn typed_stores_and_conversions_are_their_expansions() {
    for x in &edge_values() {
        for kind in kinds() {
            // StoreLocalNum / StoreGlobalNum: ToNum; Store
            let mut want = Asm::new();
            want.push(x).op(Op::ToNum).u8(kind).op(Op::StoreLocal).u16(2).op(Op::LoadLocal).u16(2).store_global("result");
            let mut got = Asm::new();
            got.push(x).op(Op::StoreLocalNum).u16(2).u8(kind).op(Op::LoadLocal).u16(2).store_global("result");
            assert_eq!(got.run(), want.run(), "StoreLocalNum {kind} of {x:?}");
            let mut want = Asm::new();
            want.push(x).op(Op::ToNum).u8(kind).store_global("result");
            let mut got = Asm::new();
            let g = got.g("result");
            got.push(x).op(Op::StoreGlobalNum).u32(g).u8(kind);
            assert_eq!(got.run(), want.run(), "StoreGlobalNum {kind} of {x:?}");
            if kind == numeric::ARG_ROUND {
                continue;
            }
            // ConvLocal / ConvArgLocal: in place
            for arg in [false, true] {
                let mut want = Asm::new();
                want.push(x).op(Op::StoreLocal).u16(0).op(Op::LoadLocal).u16(0);
                if arg {
                    want.op(Op::ToNum).u8(numeric::ARG_ROUND);
                }
                want.op(Op::ToNum).u8(kind).op(Op::StoreLocal).u16(0).op(Op::LoadLocal).u16(0).store_global("result");
                let mut got = Asm::new();
                got.push(x).op(Op::StoreLocal).u16(0).op(if arg { Op::ConvArgLocal } else { Op::ConvLocal }).u16(0).u8(kind).op(Op::LoadLocal).u16(0).store_global("result");
                assert_eq!(got.run(), want.run(), "Conv{} {kind} of {x:?}", if arg { "Arg" } else { "" });
            }
        }
        // MoveLocal
        let mut want = Asm::new();
        want.push(x).op(Op::StoreLocal).u16(1).op(Op::LoadLocal).u16(1).op(Op::StoreLocal).u16(3).op(Op::LoadLocal).u16(3).store_global("result");
        let mut got = Asm::new();
        got.push(x).op(Op::StoreLocal).u16(1).op(Op::MoveLocal).u16(3).u16(1).op(Op::LoadLocal).u16(3).store_global("result");
        assert_eq!(got.run(), want.run(), "MoveLocal of {x:?}");
    }
    // (a kind no runtime knows: both refused before they run, by the
    // verifier — rapidr_bytecode::verify)
    let mut want = Asm::new();
    want.push(&Value::Integer(1)).op(Op::ToNum).u8(77).op(Op::StoreLocal).u16(0);
    let mut got = Asm::new();
    got.push(&Value::Integer(1)).op(Op::StoreLocalNum).u16(0).u8(77);
    for r in [got.run(), want.run()] {
        assert!(r.contains("is damaged") && r.contains("kind 77"), "{r}");
    }
}

/// The numbers a FOR loop's counter, end and step are tried with.
fn for_values() -> Vec<Value> {
    edge_values().into_iter().filter(|v| !matches!(v, Value::String(s) if s.as_str() == "abc")).collect()
}

#[test]
fn the_for_test_and_step_are_their_expansions() {
    let values = for_values();
    for global in [false, true] {
        for var in &values {
            for end in &values {
                for step in &values {
                    // ForTest: (var: local 0 or ga; end local 1, step local 2)
                    let setup = |asm: &mut Asm| {
                        asm.push(var);
                        if global {
                            asm.store_global("ga");
                        } else {
                            asm.op(Op::StoreLocal).u16(0);
                        }
                        asm.push(end).op(Op::StoreLocal).u16(1);
                        asm.push(step).op(Op::StoreLocal).u16(2);
                    };
                    let load_var = |asm: &mut Asm| {
                        if global {
                            let g = asm.g("ga");
                            asm.op(Op::LoadGlobal).u32(g);
                        } else {
                            asm.op(Op::LoadLocal).u16(0);
                        }
                    };
                    let tail = |asm: &mut Asm, exit: usize| {
                        asm.push(&Value::String("body".into())).store_global("path").op(Op::Halt);
                        let at = asm.code.len();
                        asm.fill(exit, at);
                        asm.push(&Value::String("exit".into())).store_global("path");
                    };
                    let mut want = Asm::new();
                    setup(&mut want);
                    let zero = want.k(&Value::Integer(0));
                    want.op(Op::LoadLocal).u16(2).op(Op::LoadConst).u32(zero).op(Op::Lt).op(Op::JumpIfNot);
                    let up = want.hole();
                    load_var(&mut want);
                    want.op(Op::LoadLocal).u16(1).op(Op::Ge).op(Op::Jump);
                    let test = want.hole();
                    let at = want.code.len();
                    want.fill(up, at);
                    load_var(&mut want);
                    want.op(Op::LoadLocal).u16(1).op(Op::Le);
                    let at = want.code.len();
                    want.fill(test, at);
                    want.op(Op::JumpIfNot);
                    let exit = want.hole();
                    tail(&mut want, exit);
                    let mut got = Asm::new();
                    setup(&mut got);
                    if global {
                        let g = got.g("ga");
                        got.op(Op::ForTestG).u32(g);
                    } else {
                        got.op(Op::ForTestL).u16(0);
                    }
                    got.u16(1).u16(2);
                    let exit = got.hole();
                    tail(&mut got, exit);
                    assert_eq!(got.run(), want.run(), "FOR test of {var:?} to {end:?} step {step:?} (global {global})");
                }
                // ForStep: var = var + step, then to the top (which records it)
                let step = end;
                let program = |fused: bool| {
                    let mut asm = Asm::new();
                    asm.push(var);
                    if global {
                        asm.store_global("ga");
                    } else {
                        asm.op(Op::StoreLocal).u16(0);
                    }
                    asm.push(step).op(Op::StoreLocal).u16(2).op(Op::Jump);
                    let to_step = asm.hole();
                    let top = asm.code.len();
                    if global {
                        let g = asm.g("ga");
                        asm.op(Op::LoadGlobal).u32(g);
                    } else {
                        asm.op(Op::LoadLocal).u16(0);
                    }
                    asm.store_global("result").op(Op::Halt);
                    let at = asm.code.len();
                    asm.fill(to_step, at);
                    let g = asm.g("ga");
                    match (fused, global) {
                        (true, false) => {
                            asm.op(Op::ForStepL).u16(0).u16(2).u32(top as u32);
                        }
                        (true, true) => {
                            asm.op(Op::ForStepG).u32(g).u16(2).u32(top as u32);
                        }
                        (false, false) => {
                            asm.op(Op::LoadLocal).u16(0).op(Op::LoadLocal).u16(2).op(Op::Add).op(Op::StoreLocal).u16(0).op(Op::Jump).u32(top as u32);
                        }
                        (false, true) => {
                            asm.op(Op::LoadGlobal).u32(g).op(Op::LoadLocal).u16(2).op(Op::Add).op(Op::StoreGlobal).u32(g).op(Op::Jump).u32(top as u32);
                        }
                    }
                    asm.run()
                };
                assert_eq!(program(true), program(false), "FOR step of {var:?} by {step:?} (global {global})");
            }
        }
    }
}

/// The arrays an access is tried on: one dimension 1 TO 5 (Doubles), one
/// 0 TO 3 of text, two dimensions, a string read as a list, Null, a number.
fn arrays(asm: &mut Asm, which: usize) {
    let fill = |asm: &mut Asm, v: Value, bounds: &[(i64, i64)]| {
        asm.push(&v);
        for &(lo, hi) in bounds {
            asm.push(&Value::Integer(lo)).push(&Value::Integer(hi));
        }
        asm.op(Op::NewArray).u8(bounds.len() as u8);
    };
    match which {
        0 => fill(asm, Value::Double(0.5), &[(1, 5)]),
        1 => fill(asm, Value::String("".into()), &[(0, 3)]),
        2 => fill(asm, Value::Integer(9), &[(0, 2), (1, 3)]),
        3 => {
            asm.push(&Value::String("a, b ,c".into()));
        }
        4 => {
            asm.push(&Value::Null);
        }
        _ => {
            asm.push(&Value::Integer(42));
        }
    }
}

#[test]
fn one_index_array_access_is_its_expansion() {
    let indices: Vec<Value> = [0, 1, 3, 5, 6, -1, i64::MAX, i64::MIN].map(Value::Integer).into_iter().chain([Value::Double(2.7), Value::Double(f64::NAN), Value::String("3".into()), Value::Null, Value::Boolean(true)]).collect();
    let stored = [Value::Double(-2.5), Value::Integer(i64::MAX), Value::String("x".into()), Value::Null];
    for which in 0..6 {
        for i in &indices {
            for arr_mode in [Mode::L, Mode::G] {
                for idx_mode in [Mode::S, Mode::L, Mode::G] {
                    // (the array in local 3 or global arr, the index in local
                    // 1, global gb or on the stack)
                    let setup = |asm: &mut Asm| {
                        arrays(asm, which);
                        if arr_mode == Mode::L {
                            asm.op(Op::StoreLocal).u16(3);
                        } else {
                            asm.store_global("arr");
                        }
                        asm.push(i).op(Op::StoreLocal).u16(1);
                        asm.push(i).store_global("gb");
                    };
                    let load_arr = |asm: &mut Asm| {
                        if arr_mode == Mode::L {
                            asm.op(Op::LoadLocal).u16(3);
                        } else {
                            let g = asm.g("arr");
                            asm.op(Op::LoadGlobal).u32(g);
                        }
                    };
                    let arr_operand = |asm: &mut Asm| {
                        if arr_mode == Mode::L {
                            asm.u16(3);
                        } else {
                            let g = asm.g("arr");
                            asm.u32(g);
                        }
                    };
                    let get = match (arr_mode, idx_mode) {
                        (Mode::L, Mode::S) => Op::AGetLS,
                        (Mode::L, Mode::L) => Op::AGetLL,
                        (Mode::L, _) => Op::AGetLG,
                        (_, Mode::S) => Op::AGetGS,
                        (_, Mode::L) => Op::AGetGL,
                        _ => Op::AGetGG,
                    };
                    let set = match (arr_mode, idx_mode) {
                        (Mode::L, Mode::S) => Op::ASetLS,
                        (Mode::L, Mode::L) => Op::ASetLL,
                        (Mode::L, _) => Op::ASetLG,
                        (_, Mode::S) => Op::ASetGS,
                        (_, Mode::L) => Op::ASetGL,
                        _ => Op::ASetGG,
                    };
                    // AGet
                    let mut want = Asm::new();
                    setup(&mut want);
                    load_arr(&mut want);
                    load(&mut want, idx_mode, i, false);
                    want.op(Op::AGet).u8(1).store_global("result");
                    let mut got = Asm::new();
                    setup(&mut got);
                    if idx_mode == Mode::S {
                        got.push(i);
                    }
                    got.op(get);
                    arr_operand(&mut got);
                    operand(&mut got, idx_mode, i, false);
                    got.store_global("result");
                    assert_eq!(got.run(), want.run(), "{get:?} of array {which} at {i:?}");
                    // ASet, with and without a conversion
                    for v in &stored {
                        for kind in [NO_KIND, NumKind::Double.code(), NumKind::Long.code(), NumKind::Byte.code()] {
                            let snapshot = |asm: &mut Asm| {
                                load_arr(asm);
                                asm.store_global("result");
                            };
                            let mut want = Asm::new();
                            setup(&mut want);
                            load_arr(&mut want);
                            load(&mut want, idx_mode, i, false);
                            want.push(v);
                            if kind != NO_KIND {
                                want.op(Op::ToNum).u8(kind);
                            }
                            want.op(Op::ASet).u8(1);
                            snapshot(&mut want);
                            let mut got = Asm::new();
                            setup(&mut got);
                            // (the value first, then a stack index on it)
                            got.push(v);
                            if idx_mode == Mode::S {
                                got.push(i);
                            }
                            got.op(set);
                            arr_operand(&mut got);
                            operand(&mut got, idx_mode, i, false);
                            got.u8(kind);
                            snapshot(&mut got);
                            assert_eq!(got.run(), want.run(), "{set:?} of array {which} at {i:?} to {v:?} (kind {kind})");
                        }
                    }
                }
            }
        }
    }
}

/// The operations the instructions share give what `Value`'s operators
/// give (the old instructions called those directly).
#[test]
fn shared_operations_are_values_own() {
    let values = edge_values();
    for a in &values {
        for b in &values {
            let same = |x: Value, y: Value, what: &str| assert_eq!(exact(&x), exact(&y), "{what} {a:?}, {b:?}");
            same(fast::add(a, b), a + b, "+");
            same(fast::sub(a, b), a - b, "-");
            same(fast::mul(a, b), a * b, "*");
            same(fast::div(a, b), a / b, "/");
            assert_eq!(fast::modulo(a, b).map(|v| exact(&v)).map_err(|e| e.to_string()), a.checked_mod(b).map(|v| exact(&v)).map_err(|e| format!("run-time error: {e}")), "MOD {a:?}, {b:?}");
            assert_eq!(fast::eq(a, b), a.rp_eq(b).to_bool(), "= {a:?}, {b:?}");
            assert_eq!(fast::ne(a, b), a.rp_ne(b).to_bool(), "<> {a:?}, {b:?}");
            assert_eq!(fast::lt(a, b), a.rp_lt(b).to_bool(), "< {a:?}, {b:?}");
            assert_eq!(fast::le(a, b), a.rp_le(b).to_bool(), "<= {a:?}, {b:?}");
            assert_eq!(fast::gt(a, b), a.rp_gt(b).to_bool(), "> {a:?}, {b:?}");
            assert_eq!(fast::ge(a, b), a.rp_ge(b).to_bool(), ">= {a:?}, {b:?}");
        }
        for kind in kinds() {
            let want = if kind == numeric::ARG_ROUND { numeric::arg_round(a) } else { numeric::convert(a, NumKind::from_code(kind).unwrap()) };
            assert_eq!(exact(&fast::to_num(a.clone(), kind).unwrap()), exact(&want), "ToNum {kind} of {a:?}");
        }
        assert_eq!(fast::index(a), a.to_i64());
    }
}

/// The builtins the VM calls itself are the ones the hosts call: each
/// name's function, on every edge value, gives what `rapidr_value`'s
/// builtin of that name gives (rapidr-vm-host-native's tests/
/// direct_maths.rs checks the desktop host answers each name the same).
#[test]
fn direct_builtins_are_named_as_the_hosts_name_them() {
    use rapidr_value::builtins as b;
    type Builtin = fn(&Value) -> Value;
    let table: [(&str, Builtin); 22] = [
        ("ABS", b::rp_abs),
        ("Int", b::rp_int),
        ("FIX", b::rp_fix),
        ("frac", b::rp_frac),
        ("SGN", b::rp_sgn),
        ("SQR", b::rp_sqr),
        ("SIN", b::rp_sin),
        ("COS", b::rp_cos),
        ("TAN", b::rp_tan),
        ("ATN", b::rp_atn),
        ("ATAN", b::rp_atn),
        ("ASIN", b::rp_asin),
        ("ACOS", b::rp_acos),
        ("LOG", b::rp_log),
        ("EXP", b::rp_exp),
        ("CEIL", b::rp_ceil),
        ("FLOOR", b::rp_floor),
        ("ROUND", b::rp_round),
        ("CINT", b::rp_cint),
        ("CLNG", b::rp_clng),
        ("CDBL", b::rp_cdbl),
        ("CSNG", b::rp_csng),
    ];
    assert_eq!(fast::direct_names().count(), table.len() + 2);
    assert_eq!(fast::direct_names().collect::<Vec<_>>(), rapidr_bytecode::builtins::PURE_MATHS, "the compiler's list is the VM's");
    // (SHL and SHR take two)
    for (name, f) in [("SHL", rapidr_value::rp_shl as fn(&Value, &Value) -> Value), ("shr", rapidr_value::rp_shr)] {
        let direct = fast::direct_builtin2(name).unwrap_or_else(|| panic!("{name}"));
        assert!(fast::direct_builtin(name).is_none());
        for a in edge_values() {
            for b in edge_values() {
                assert_eq!(exact(&direct(&a, &b)), exact(&f(&a, &b)), "{name}({a:?}, {b:?})");
            }
        }
        let code = fast::direct_code(name);
        assert_eq!(exact(&fast::call_direct(code, &[Value::Integer(-1)])), exact(&f(&Value::Integer(-1), &Value::Null)), "a missing argument is Null");
    }
    for (name, f) in table {
        let direct = fast::direct_builtin(name).unwrap_or_else(|| panic!("{name}"));
        for v in edge_values() {
            assert_eq!(exact(&direct(&v)), exact(&f(&v)), "{name}({v:?})");
        }
    }
    for other in ["MID$", "TIMER", "RND", "LEN", "nosuch", "SQRT"] {
        assert!(fast::direct_builtin(other).is_none(), "{other}");
    }
    // (a missing argument is Null, as the hosts read it)
    assert_eq!(exact(&fast::call_direct(fast::direct_code("ABS"), &[])), exact(&b::rp_abs(&Value::Null)));
}

/// A host that doesn't opt in hears every builtin itself; one that does
/// doesn't hear the pure maths ones, and the program gets the same answer.
#[test]
fn direct_maths_skip_only_a_host_that_lets_them() {
    struct Hears {
        heard: Vec<String>,
    }
    impl crate::Host for Hears {
        const DIRECT_MATHS: bool = true;
        fn call_builtin(&mut self, name: &str, _args: &[Value]) -> Result<Value, String> {
            self.heard.push(name.to_string());
            Ok(Value::Integer(5))
        }
        fn create_comp(&mut self, _k: &str, id: &str) -> Result<Value, String> {
            Ok(rapidr_value::v_str(id))
        }
        fn set_prop(&mut self, _id: &str, _n: &str, _v: Value) -> Result<(), String> {
            Ok(())
        }
        fn get_prop(&mut self, _id: &str, _n: &str) -> Result<Value, String> {
            Ok(Value::Null)
        }
        fn call_method(&mut self, _id: &str, _m: &str, _a: &[Value]) -> Result<Value, String> {
            Ok(Value::Null)
        }
        fn register_event(&mut self, _id: &str, _e: &str, _f: u32) -> Result<(), String> {
            Ok(())
        }
        fn print(&mut self, _s: &str) -> Result<(), String> {
            Ok(())
        }
        fn input(&mut self) -> Result<String, String> {
            Ok(String::new())
        }
    }
    let mut asm = Asm::new();
    let sqr = asm.m.add_string("SQR");
    let len = asm.m.add_string("LEN");
    asm.push(&Value::Double(2.25)).op(Op::CallBuiltin).u32(sqr).u8(1).store_global("ga");
    asm.push(&Value::String("abc".into())).op(Op::CallBuiltin).u32(len).u8(1).store_global("gb");
    asm.op(Op::Halt);
    let f = Function { name: "__main".into(), n_locals: 4, code: std::mem::take(&mut asm.code), ..Default::default() };
    asm.m.entry = asm.m.add_function(f);
    let mut host = Hears { heard: Vec::new() };
    let mut vm = Vm::new(&mut host);
    vm.run(&asm.m).unwrap();
    let ga = vm.global_index(&asm.m, "ga").unwrap();
    assert_eq!(exact(vm.globals[ga].as_ref().unwrap()), exact(&Value::Double(1.5)));
    assert_eq!(host.heard, ["LEN"], "SQR ran in the VM, LEN in the host");
    // (the stub host doesn't opt in: it hears SQR)
    let mut stub = StubHost::default();
    let mut vm = Vm::new(&mut stub);
    vm.run(&asm.m).unwrap();
    assert_eq!(exact(vm.globals[ga].as_ref().unwrap()), "N", "the stub host's answer");
}
