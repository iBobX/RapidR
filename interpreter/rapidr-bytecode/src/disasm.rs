//! A function's code as text, one instruction a line — for the verifier's
//! messages, the JIT's trace (`RAPIDR_JIT_TRACE`) and tests. Never parsed
//! back.

use crate::{Function, Module, Op, Operand};

/// One decoded instruction: its offset, opcode and operand values (each
/// widened to `i64`), and the offset of the next one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Insn {
    pub at: usize,
    pub op: Op,
    pub operands: Vec<i64>,
    pub next: usize,
}

/// Decodes the instruction at `at`: `None` for an unknown opcode or code
/// that ends inside its operands.
pub fn decode(code: &[u8], at: usize) -> Option<Insn> {
    let op = Op::from_u8(*code.get(at)?)?;
    let mut ip = at + 1;
    let mut operands = Vec::with_capacity(4);
    for o in op.layout() {
        let w = o.width();
        let b = code.get(ip..ip + w)?;
        let v = match o {
            Operand::U8 => i64::from(b[0]),
            Operand::Local | Operand::Field => i64::from(u16::from_le_bytes([b[0], b[1]])),
            Operand::I32 => i64::from(i32::from_le_bytes([b[0], b[1], b[2], b[3]])),
            _ => i64::from(u32::from_le_bytes([b[0], b[1], b[2], b[3]])),
        };
        operands.push(v);
        ip += w;
    }
    Some(Insn { at, op, operands, next: ip })
}

/// `f`'s code, one instruction a line (`  12 L3  AddLK s1 k4`): offset,
/// the line where a statement starts, the opcode and its operands (`s`
/// local, `g` global by name, `k` constant, `@` target, `f` function).
pub fn function(m: &Module, f: &Function) -> String {
    let mut out = String::new();
    let mut ip = 0;
    while ip < f.code.len() {
        let line = f.line_at(ip).filter(|&(_, start)| start).map(|(l, _)| format!("L{l}")).unwrap_or_default();
        let Some(insn) = decode(&f.code, ip) else {
            out.push_str(&format!("{ip:5} {line:6} ?? 0x{:02X}\n", f.code[ip]));
            break;
        };
        let mut text = format!("{ip:5} {line:6} {:?}", insn.op);
        for (o, v) in insn.op.layout().iter().zip(&insn.operands) {
            let s = match o {
                Operand::Local => format!(" s{v}"),
                Operand::Global => format!(" g:{}", m.strings.get(*v as usize).map_or("?", String::as_str)),
                Operand::Str => format!(" {:?}", m.strings.get(*v as usize).map_or("?", String::as_str)),
                Operand::Const => format!(" k{v}={}", m.consts.get(*v as usize).map_or("?".into(), |c| format!("{c:?}"))),
                Operand::Func => format!(" f{v}:{}", m.functions.get(*v as usize).map_or("?", |f| f.name.as_str())),
                Operand::Target => format!(" @{v}"),
                Operand::Field => format!(" .{v}"),
                Operand::U8 | Operand::I32 => format!(" {v}"),
            };
            text.push_str(&s);
        }
        out.push_str(&text);
        out.push('\n');
        ip = insn.next;
    }
    out
}

/// Every function of `m` ([`function`]), each under its name.
pub fn module(m: &Module) -> String {
    let mut out = String::new();
    for (i, f) in m.functions.iter().enumerate() {
        out.push_str(&format!("== f{i} {} (params {}, locals {})\n", f.name, f.params.len(), f.n_locals));
        out.push_str(&function(m, f));
    }
    out
}
