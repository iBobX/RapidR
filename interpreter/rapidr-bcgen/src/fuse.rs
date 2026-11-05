//! Fused instructions (docs/vm-fast-design.md): the compiler emits an
//! expression or a statement as it always has, then folds a sequence it
//! knows into the one instruction that means the same (`rapidr_bytecode::
//! Op`'s fused instructions, each defined by its expansion). Every rewrite
//! here works on whole instructions the caller just emitted, within one
//! statement: no line entry, label or jump points inside what is rewritten.
//!
//! An operand read later than the code would have read it (`a + (b * c)` as
//! `MulLL b, c; AddLS a`, an array read after its index) is used only when
//! the code in between [`is_pure`]: it reads values and calls nothing that
//! could change one.

use rapidr_bytecode::{builtins, Bin, Cmp, Mode, Module, Op, NO_KIND};

/// `code` as one load of a leaf — `LoadLocal`, `LoadGlobal` or
/// `LoadConst` — and nothing else: its mode and its operand's bytes.
pub(crate) fn leaf(code: &[u8]) -> Option<(Mode, Vec<u8>)> {
    let op = Op::from_u8(*code.first()?)?;
    let mode = match op {
        Op::LoadLocal => Mode::L,
        Op::LoadGlobal => Mode::G,
        Op::LoadConst => Mode::K,
        _ => return None,
    };
    (code.len() == 1 + mode.width()).then(|| (mode, code[1..].to_vec()))
}

/// Whether `code` (whole instructions) only reads values: loads,
/// arithmetic, comparisons, conversions, array and field reads and the pure
/// maths builtins — no call of the program's, no host operation, no store,
/// no jump.
pub(crate) fn is_pure(code: &[u8], module: &Module) -> bool {
    let mut ip = 0;
    while ip < code.len() {
        let Some(op) = Op::from_u8(code[ip]) else { return false };
        let pure = match op {
            Op::LoadConst | Op::LoadNull | Op::LoadTrue | Op::LoadFalse | Op::LoadLocal | Op::LoadGlobal => true,
            Op::Add | Op::Sub | Op::Mul | Op::Div | Op::IDiv | Op::Mod | Op::Pow | Op::Neg | Op::Concat => true,
            Op::Eq | Op::Ne | Op::Lt | Op::Le | Op::Gt | Op::Ge => true,
            Op::And | Op::Or | Op::Not | Op::Xor | Op::BAnd | Op::BOr | Op::BNot | Op::BXor | Op::Shl | Op::Shr => true,
            Op::ToNum | Op::AGet | Op::GetField => true,
            Op::AGetLS | Op::AGetLL | Op::AGetLG | Op::AGetGS | Op::AGetGL | Op::AGetGG => true,
            Op::CallBuiltin => {
                let at = ip + 1;
                code.get(at..at + 4).and_then(|b| module.strings.get(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)).is_some_and(|name| builtins::is_pure_maths(name))
            }
            _ => op.as_fused_arith().is_some(),
        };
        if !pure {
            return false;
        }
        ip += 1 + op.operand_len();
    }
    ip == code.len()
}

fn emit(code: &mut Vec<u8>, op: Op, operands: &[&[u8]]) {
    code.push(op as u8);
    for o in operands {
        code.extend_from_slice(o);
    }
}

/// `a op b`, its operands' code at `start..mid` and `mid..` (the end of
/// `code`): the fused instruction when one fits; false (nothing changed)
/// for the plain one.
pub(crate) fn arith(code: &mut Vec<u8>, start: usize, mid: usize, bin: Bin, module: &Module) -> bool {
    let a = leaf(&code[start..mid]);
    let b = leaf(&code[mid..]);
    match (a, b) {
        (Some((am, ao)), Some((bm, bo))) => {
            if let Some(op) = Op::fused_arith(bin, am, bm) {
                code.truncate(start);
                emit(code, op, &[&ao, &bo]);
                return true;
            }
            // (no such form, `2 MOD n` …: a's load stays)
            match Op::fused_arith(bin, Mode::S, bm) {
                Some(op) => {
                    code.truncate(mid);
                    emit(code, op, &[&bo]);
                    true
                }
                None => false,
            }
        }
        (None, Some((bm, bo))) => match Op::fused_arith(bin, Mode::S, bm) {
            Some(op) => {
                code.truncate(mid);
                emit(code, op, &[&bo]);
                true
            }
            None => false,
        },
        (Some((am, ao)), None) => {
            // (a read after b's code: only when that code can't change it)
            let Some(op) = Op::fused_arith(bin, am, Mode::S) else { return false };
            if !is_pure(&code[mid..], module) {
                return false;
            }
            code.drain(start..mid);
            emit(code, op, &[&ao]);
            true
        }
        (None, None) => false,
    }
}

/// `IF NOT (a cmp b) THEN GOTO …`, the operands' code at `start..mid` and
/// `mid..`: emits the compare-and-branch (its target to fill in, after
/// it).
pub(crate) fn cmp_jump(code: &mut Vec<u8>, start: usize, mid: usize, cmp: Cmp) {
    let a = leaf(&code[start..mid]);
    let b = leaf(&code[mid..]);
    if let (Some((am, ao)), Some((bm, bo))) = (&a, &b) {
        if let Some(op) = Op::cmp_jump(cmp, *am, *bm) {
            code.truncate(start);
            emit(code, op, &[ao, bo]);
            return;
        }
    }
    if let Some((bm, bo)) = &b {
        if let Some(op) = Op::cmp_jump(cmp, Mode::S, *bm) {
            code.truncate(mid);
            emit(code, op, &[bo]);
            return;
        }
    }
    emit(code, Op::cmp_jump(cmp, Mode::S, Mode::S).expect("the stack form"), &[]);
}

/// `arr(i)` with one index: the array's load at `start..idx`, the index's
/// code at `idx..`. False: the plain `AGet`.
pub(crate) fn aget(code: &mut Vec<u8>, start: usize, idx: usize, module: &Module) -> bool {
    let Some((am, ao)) = leaf(&code[start..idx]).filter(|(m, _)| *m != Mode::K) else { return false };
    let local = am == Mode::L;
    match leaf(&code[idx..]).filter(|(m, _)| *m != Mode::K) {
        Some((im, io)) => {
            let op = match (local, im == Mode::L) {
                (true, true) => Op::AGetLL,
                (true, false) => Op::AGetLG,
                (false, true) => Op::AGetGL,
                (false, false) => Op::AGetGG,
            };
            code.truncate(start);
            emit(code, op, &[&ao, &io]);
            true
        }
        None => {
            // (the array read after its index: only a pure one)
            if !is_pure(&code[idx..], module) {
                return false;
            }
            code.drain(start..idx);
            emit(code, if local { Op::AGetLS } else { Op::AGetGS }, &[&ao]);
            true
        }
    }
}

/// The end of an assignment `x = value` / `arr(i) = value`, its value's
/// code at `start..store` and the store's from `store` to the end:
/// the conversion folded into the store, a parameter's conversion done in
/// place, a copy of a local in one instruction, a one-index array store in
/// one. `converted`: the value is `__to_<kind>(…)` (its code ends with that
/// `ToNum`).
pub(crate) fn store(code: &mut Vec<u8>, start: usize, store: usize, converted: bool, module: &Module) {
    let kind = (converted && store >= start + 2 && code[store - 2] == Op::ToNum as u8).then(|| code[store - 1]);
    let tail = code[store..].to_vec();
    let Some(op) = tail.first().copied().and_then(Op::from_u8) else { return };
    match op {
        Op::StoreLocal if tail.len() == 3 => {
            let slot = &tail[1..3];
            if let Some(kind) = kind {
                let inner = &code[start..store - 2];
                // `x = __to_k(x)` (a parameter on entry): in place
                if inner.len() == 3 && inner[0] == Op::LoadLocal as u8 && &inner[1..3] == slot {
                    code.truncate(start);
                    emit(code, Op::ConvLocal, &[slot, &[kind]]);
                    return;
                }
                // `x = __to_k(__arg_round(x))` (a BYVAL integer one)
                if inner.len() == 5 && inner[0] == Op::LoadLocal as u8 && &inner[1..3] == slot && inner[3] == Op::ToNum as u8 && inner[4] == rapidr_value::numeric::ARG_ROUND {
                    code.truncate(start);
                    emit(code, Op::ConvArgLocal, &[slot, &[kind]]);
                    return;
                }
                code.truncate(store - 2);
                emit(code, Op::StoreLocalNum, &[slot, &[kind]]);
                return;
            }
            let value = &code[start..store];
            if value.len() == 3 && value[0] == Op::LoadLocal as u8 {
                let from = value[1..3].to_vec();
                code.truncate(start);
                emit(code, Op::MoveLocal, &[slot, &from]);
            }
        }
        Op::StoreGlobal if tail.len() == 5 => {
            if let Some(kind) = kind {
                code.truncate(store - 2);
                emit(code, Op::StoreGlobalNum, &[&tail[1..5], &[kind]]);
            }
        }
        // `StoreLocal t; <array>; <index>; LoadLocal t; ASet 1` (the value
        // parked while the array and its index are read)
        Op::StoreLocal if tail.len() >= 3 + 3 + 2 + 3 && tail[tail.len() - 2..] == [Op::ASet as u8, 1] && tail[tail.len() - 5] == Op::LoadLocal as u8 && tail[tail.len() - 4..tail.len() - 2] == tail[1..3] => {
            let middle = &tail[3..tail.len() - 5];
            let Some(arr_op) = middle.first().copied().and_then(Op::from_u8) else { return };
            let arr_len = match arr_op {
                Op::LoadLocal => 3,
                Op::LoadGlobal => 5,
                _ => return,
            };
            if middle.len() <= arr_len {
                return;
            }
            let (arr, index) = middle.split_at(arr_len);
            let local = arr_op == Op::LoadLocal;
            let index_leaf = leaf(index).filter(|(m, _)| *m != Mode::K);
            if index_leaf.is_none() && !is_pure(index, module) {
                return;
            }
            let (arr, index) = (arr[1..].to_vec(), index.to_vec());
            let kind = kind.unwrap_or(NO_KIND);
            code.truncate(if kind == NO_KIND { store } else { store - 2 });
            match index_leaf {
                Some((im, io)) => {
                    let op = match (local, im == Mode::L) {
                        (true, true) => Op::ASetLL,
                        (true, false) => Op::ASetLG,
                        (false, true) => Op::ASetGL,
                        (false, false) => Op::ASetGG,
                    };
                    emit(code, op, &[&arr, &io, &[kind]]);
                }
                None => {
                    // (the value, then its index on it)
                    code.extend_from_slice(&index);
                    emit(code, if local { Op::ASetLS } else { Op::ASetGS }, &[&arr, &[kind]]);
                }
            }
        }
        _ => {}
    }
}

/// The arithmetic a binary operator fuses as.
pub(crate) fn bin_of(op: Op) -> Option<Bin> {
    Some(match op {
        Op::Add => Bin::Add,
        Op::Sub => Bin::Sub,
        Op::Mul => Bin::Mul,
        Op::Div => Bin::Div,
        Op::Mod => Bin::Mod,
        _ => return None,
    })
}
