//! The bytecode verifier: a module is checked before it runs
//! (docs/jit-design.md §7, §10). Every instruction decodes; every operand
//! names something the module has (a local slot of its function, a string,
//! a constant, a function, a conversion kind); every jump lands on the
//! start of an instruction (or the code's end). A module that fails is
//! *damaged* — a file cut short, edited by hand, or made by something that
//! isn't RapidR's compiler — and is refused before its first instruction
//! with what is wrong and where.
//!
//! The interpreter checks the same things as it goes (a bad slot is an
//! error there too), so for it the verifier is an early, clear refusal. For
//! the JIT it is the precondition: compiled code reads locals, globals and
//! constants at offsets taken from the operands, so they must be in range
//! before anything is compiled.
//!
//! What it doesn't judge: the operand stack's height (the interpreter
//! checks each pop; the JIT works it out itself and stays away from a
//! function whose heights disagree), and anything about meaning.

use crate::disasm::decode;
use crate::{Module, Op, Operand, NO_KIND};

/// What is wrong with a damaged module, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyError {
    /// The function's index and name (`None`: the module itself).
    pub function: Option<(u32, String)>,
    /// The instruction's offset in its code.
    pub at: Option<usize>,
    pub problem: String,
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (&self.function, self.at) {
            (Some((i, name)), Some(at)) => write!(f, "{} (in {}, routine {i}, at byte {at})", self.problem, shown(name)),
            (Some((i, name)), None) => write!(f, "{} (in {}, routine {i})", self.problem, shown(name)),
            _ => write!(f, "{}", self.problem),
        }
    }
}

impl std::error::Error for VerifyError {}

/// A routine's name for a message: the main program by what it is, not by
/// its inner name; a damaged name cut short.
fn shown(name: &str) -> String {
    if name == "__main" {
        return "the main program".into();
    }
    let clean: String = name.chars().filter(|c| !c.is_control()).take(64).collect();
    format!("{clean:?}")
}

/// The most local slots a function may have: their operands are `u16`.
pub const MAX_LOCALS: u32 = u16::MAX as u32 + 1;

/// Checks the whole module ([`function`] for each).
pub fn module(m: &Module) -> Result<(), VerifyError> {
    let at_module = |problem: String| VerifyError { function: None, at: None, problem };
    if m.functions.is_empty() {
        return Err(at_module("it has no code".into()));
    }
    if m.entry as usize >= m.functions.len() {
        return Err(at_module(format!("its start (routine {}) is not one of its {} routines", m.entry, m.functions.len())));
    }
    if m.functions.len() > u32::MAX as usize || m.strings.len() > u32::MAX as usize || m.consts.len() > u32::MAX as usize {
        return Err(at_module("it is too big".into()));
    }
    for i in 0..m.functions.len() {
        function(m, i as u32)?;
    }
    Ok(())
}

/// Checks function `index` of `m`.
pub fn function(m: &Module, index: u32) -> Result<(), VerifyError> {
    let f = &m.functions[index as usize];
    let err = |at: Option<usize>, problem: String| VerifyError { function: Some((index, f.name.clone())), at, problem };
    if f.n_locals > MAX_LOCALS {
        return Err(err(None, format!("it has {} local variables (at most {MAX_LOCALS})", f.n_locals)));
    }
    if f.params.len() > f.n_locals as usize {
        return Err(err(None, format!("its {} parameters don't fit its {} local variables", f.params.len(), f.n_locals)));
    }
    if f.code.len() > u32::MAX as usize {
        return Err(err(None, "its code is too long".into()));
    }
    let code = &f.code;
    // (where instructions start: a jump must land on one)
    let mut starts = vec![false; code.len() + 1];
    starts[code.len()] = true;
    let mut targets: Vec<(usize, u32)> = Vec::new();
    let mut ip = 0;
    while ip < code.len() {
        let Some(insn) = decode(code, ip) else {
            return Err(match Op::from_u8(code[ip]) {
                None => err(Some(ip), format!("0x{:02X} is not an instruction", code[ip])),
                Some(op) => err(Some(ip), format!("its {op:?} instruction is cut short")),
            });
        };
        starts[ip] = true;
        for (o, &v) in insn.op.layout().iter().zip(&insn.operands) {
            let bad = |what: &str| err(Some(ip), format!("{:?} names {what} {v}, which it doesn't have", insn.op));
            match o {
                Operand::Local if v as u64 >= u64::from(f.n_locals) => return Err(bad("local variable")),
                Operand::Global | Operand::Str if v as usize >= m.strings.len() => return Err(bad("name")),
                Operand::Const if v as usize >= m.consts.len() => return Err(bad("constant")),
                Operand::Func if v as usize >= m.functions.len() => return Err(bad("routine")),
                Operand::Target => targets.push((ip, v as u32)),
                _ => {}
            }
        }
        // (a conversion's kind: one the runtime has)
        let kind = |i: usize| insn.operands[i] as u8;
        // (a typed store's conversion is `ToNum`'s, ARG_ROUND included —
        // the VM runs it so; an in-place conversion takes a kind only)
        let valid_kind = |k: u8| rapidr_value::numeric::NumKind::from_code(k).is_some();
        let to_num = |k: u8| valid_kind(k) || k == rapidr_value::numeric::ARG_ROUND;
        let kind_ok = match insn.op {
            Op::ToNum => to_num(kind(0)),
            Op::StoreLocalNum | Op::StoreGlobalNum => to_num(kind(1)),
            Op::ConvLocal | Op::ConvArgLocal => valid_kind(kind(1)),
            Op::ASetLS | Op::ASetGS => kind(1) == NO_KIND || to_num(kind(1)),
            Op::ASetLL | Op::ASetLG | Op::ASetGL | Op::ASetGG => kind(2) == NO_KIND || to_num(kind(2)),
            _ => true,
        };
        if !kind_ok {
            return Err(err(Some(ip), format!("{:?} converts to kind {}, which doesn't exist", insn.op, insn.operands.last().copied().unwrap_or(0))));
        }
        ip = insn.next;
    }
    for (at, t) in targets {
        if !starts.get(t as usize).copied().unwrap_or(false) {
            return Err(err(Some(at), format!("a jump to byte {t}, which is not the start of an instruction")));
        }
    }
    for &(off, _) in &f.line_info {
        if off as usize > code.len() {
            return Err(err(None, format!("its line table points past its code (byte {off})")));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Const, Function, Module, Param};

    fn op(code: &mut Vec<u8>, op: Op, operands: &[u8]) {
        code.push(op as u8);
        code.extend_from_slice(operands);
    }

    /// `x = 1 + 2 : PRINT x` and a FUNCTION: a sound module.
    fn sound() -> Module {
        let mut m = Module::new();
        let one = m.add_const(Const::Int(1));
        let two = m.add_const(Const::Int(2));
        let g = m.add_string("x");
        let mut f = Function { name: "F".into(), params: vec![Param { name: "a".into(), by_ref: false }], n_locals: 2, ..Default::default() };
        op(&mut f.code, Op::ConvLocal, &[0, 0, 3]);
        op(&mut f.code, Op::AddLK, &[0, 0, one as u8, 0, 0, 0]);
        op(&mut f.code, Op::StoreLocal, &[1, 0]);
        op(&mut f.code, Op::LoadLocal, &[1, 0]);
        op(&mut f.code, Op::RetVal, &[]);
        let fi = m.add_function(f);
        let mut main = Function { name: "__main".into(), n_locals: 0, ..Default::default() };
        op(&mut main.code, Op::LoadConst, &two.to_le_bytes());
        op(&mut main.code, Op::CallFunc, &[fi as u8, 0, 0, 0, 1]);
        op(&mut main.code, Op::StoreGlobalNum, &[g as u8, 0, 0, 0, 3]);
        // (a loop back to the start: a jump to an instruction's start)
        op(&mut main.code, Op::JumpIfNotLtGK, &[g as u8, 0, 0, 0, two as u8, 0, 0, 0, 0, 0, 0, 0]);
        op(&mut main.code, Op::LoadGlobal, &[g as u8, 0, 0, 0]);
        op(&mut main.code, Op::PrintLn, &[]);
        op(&mut main.code, Op::Halt, &[]);
        main.line_info = vec![(0, 1)];
        m.entry = m.add_function(main);
        m
    }

    fn damaged(m: &Module) -> String {
        module(m).expect_err("damaged").to_string()
    }

    #[test]
    fn a_sound_module_passes() {
        assert_eq!(module(&sound()), Ok(()));
    }

    #[test]
    fn each_kind_of_damage_is_named() {
        let mut m = sound();
        m.functions[1].code[0] = 0x00;
        assert!(damaged(&m).contains("0x00 is not an instruction"), "{}", damaged(&m));

        let mut m = sound();
        m.functions[1].code.truncate(3);
        assert!(damaged(&m).contains("cut short"), "{}", damaged(&m));

        let mut m = sound();
        m.functions[0].code[1] = 9; // ConvLocal s9 (2 locals)
        assert!(damaged(&m).contains("local variable 9"), "{}", damaged(&m));

        let mut m = sound();
        m.functions[0].code[7] = 77; // AddLK's constant
        assert!(damaged(&m).contains("constant 77"), "{}", damaged(&m));

        let mut m = sound();
        m.functions[1].code[6] = 5; // CallFunc f5
        assert!(damaged(&m).contains("routine 5"), "{}", damaged(&m));

        let mut m = sound();
        m.functions[1].code[12] = 200; // StoreGlobalNum's name
        assert!(damaged(&m).contains("name 200"), "{}", damaged(&m));

        let mut m = sound();
        m.functions[0].code[3] = 42; // ConvLocal's kind
        assert!(damaged(&m).contains("kind"), "{}", damaged(&m));

        let mut m = sound();
        m.functions[1].code[26] = 3; // the jump's target: inside the first instruction
        assert!(damaged(&m).contains("not the start of an instruction"), "{}", damaged(&m));

        let mut m = sound();
        m.entry = 7;
        assert!(damaged(&m).contains("start"), "{}", damaged(&m));

        let mut m = sound();
        m.functions[0].n_locals = 0;
        assert!(damaged(&m).contains("parameters"), "{}", damaged(&m));

        let mut m = sound();
        m.functions[1].line_info.push((10_000, 2));
        assert!(damaged(&m).contains("line table"), "{}", damaged(&m));

        assert!(module(&Module::new()).is_err());
    }

    #[test]
    fn a_jump_to_the_code_end_is_sound() {
        let mut m = sound();
        let end = m.functions[1].code.len() as u32;
        m.functions[1].code[26..30].copy_from_slice(&end.to_le_bytes());
        assert_eq!(module(&m), Ok(()));
    }

    /// Every byte of a sound module changed to every value: the verifier
    /// never panics, and what it passes decodes all the way.
    #[test]
    fn mutations_never_panic() {
        let base = sound();
        for fi in 0..base.functions.len() {
            for at in 0..base.functions[fi].code.len() {
                for v in [0u8, 1, 2, 3, 0x11, 0x50, 0x7F, 0x80, 0xFE, 0xFF] {
                    let mut m = base.clone();
                    m.functions[fi].code[at] = v;
                    if module(&m).is_ok() {
                        let f = &m.functions[fi];
                        let mut ip = 0;
                        while ip < f.code.len() {
                            ip = decode(&f.code, ip).expect("a verified module decodes").next;
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_message_says_where() {
        let mut m = sound();
        m.functions[0].code[1] = 9;
        assert_eq!(damaged(&m), "ConvLocal names local variable 9, which it doesn't have (in \"F\", routine 0, at byte 0)");
        let mut m = sound();
        m.functions[1].code[0] = 0x00;
        assert!(damaged(&m).ends_with("(in the main program, routine 1, at byte 0)"), "{}", damaged(&m));
    }
}
