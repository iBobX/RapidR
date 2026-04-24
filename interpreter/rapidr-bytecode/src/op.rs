//! Opcode definitions.
//!
//! All opcodes are a single `u8`. Operands follow inline as fixed-width
//! little-endian integers (no varint — favouring decode speed over size).
//! Operand widths per opcode are documented next to each variant.

/// Bytecode operations.
///
/// Operand encoding key:
/// * `u8`/`u16`/`u32`/`i32`/`i64`: little-endian, raw.
/// * No padding between operands.
/// * Branch targets are absolute byte offsets within the function's `code`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    // === stack / constants ===
    /// Push constant from pool. Operand: `u32` const index.
    LoadConst = 0x01,
    /// Push `Value::Null`.
    LoadNull = 0x02,
    /// Push `Value::Boolean(true)`.
    LoadTrue = 0x03,
    /// Push `Value::Boolean(false)`.
    LoadFalse = 0x04,
    /// Pop top.
    Pop = 0x05,
    /// Duplicate top.
    Dup = 0x06,

    // === locals & globals ===
    /// Push local slot. Operand: `u16` slot.
    LoadLocal = 0x10,
    /// Pop & store into local slot. Operand: `u16` slot.
    StoreLocal = 0x11,
    /// Push global by string-pool name. Operand: `u32` string-id.
    LoadGlobal = 0x12,
    /// Pop & store into global by name. Operand: `u32` string-id.
    StoreGlobal = 0x13,

    // === arithmetic ===
    Add = 0x20,
    Sub = 0x21,
    Mul = 0x22,
    Div = 0x23,
    /// BASIC integer division (`\`).
    IDiv = 0x24,
    Mod = 0x25,
    Pow = 0x26,
    Neg = 0x27,
    /// BASIC string concat (`&`).
    Concat = 0x28,

    // === comparison ===
    Eq = 0x30,
    Ne = 0x31,
    Lt = 0x32,
    Le = 0x33,
    Gt = 0x34,
    Ge = 0x35,

    // === logical ===
    And = 0x40,
    Or  = 0x41,
    Not = 0x42,
    Xor = 0x43,

    // === bitwise ===
    BAnd = 0x48,
    BOr  = 0x49,
    BNot = 0x4A,
    BXor = 0x4B,
    Shl  = 0x4C,
    Shr  = 0x4D,

    // === control flow ===
    /// Unconditional jump. Operand: `u32` target offset.
    Jump      = 0x50,
    /// Pop, jump if truthy. Operand: `u32`.
    JumpIf    = 0x51,
    /// Pop, jump if falsy. Operand: `u32`.
    JumpIfNot = 0x52,

    // === calls ===
    /// Call a SUB by function index, no return value pushed.
    /// Operand: `u32` fn-index, `u8` argc.
    CallSub = 0x60,
    /// Call a FUNCTION by function index, push return value.
    /// Operand: `u32` fn-index, `u8` argc.
    CallFunc = 0x61,
    /// Return from SUB (push Null on caller stack? No — caller didn't expect a value).
    Ret = 0x62,
    /// Return value from FUNCTION (top of stack is the return value).
    RetVal = 0x63,
    /// Call a builtin function by string-pool name. Pushes a `Value` result
    /// (PRINT-style sinks should still push `Null`).
    /// Operand: `u32` string-id (name), `u8` argc.
    CallBuiltin = 0x64,

    // === components / objects ===
    /// Create a component. Operand: `u32` kind (string-id),
    /// `u32` instance-id (string-id, the user's variable name).
    /// Pushes a Value::String(instance-id) reference.
    CreateComp = 0x70,
    /// Set property. Stack: [..., value]. Operand: `u32` instance-id (string-id),
    /// `u32` prop-name (string-id). Pops value.
    SetProp = 0x71,
    /// Get property. Operand: `u32` instance-id (string-id),
    /// `u32` prop-name (string-id). Pushes value.
    GetProp = 0x72,
    /// Call a method on a component. Stack: [..., arg1, ..., argN].
    /// Operand: `u32` instance-id (string-id), `u32` method-name (string-id),
    /// `u8` argc. Pushes return value.
    CallMethod = 0x73,
    /// Register a SUB as an event handler.
    /// Operand: `u32` instance-id (string-id), `u32` event-name (string-id),
    /// `u32` fn-index.
    RegisterEvent = 0x74,

    // === arrays ===
    /// Create a new dynamic array of N elements (default Null).
    /// Stack: [size]. Pops size, pushes the new array (as Value::String JSON for now).
    NewArray = 0x80,
    /// Get element at index. Stack: [array, index].
    AGet = 0x81,
    /// Set element at index. Stack: [array, index, value]. Pops 3.
    ASet = 0x82,
    /// Resize an array stored in a local. Operand: `u16` local, `i32` new-size.
    Redim = 0x83,

    // === I/O ===
    /// PRINT pop top. Calls `Host::print(value.to_string_val())`.
    Print = 0x90,
    /// PRINT a newline.
    PrintLn = 0x91,
    /// Push a line read from `Host::input()`.
    Input = 0x92,

    // === misc ===
    Halt = 0xFE,
    Nop  = 0xFF,
}

impl Op {
    /// Try to convert from raw byte.
    pub fn from_u8(b: u8) -> Option<Op> {
        // Safety net: explicit table since enum repr isn't exhaustive.
        Some(match b {
            0x01 => Op::LoadConst,
            0x02 => Op::LoadNull,
            0x03 => Op::LoadTrue,
            0x04 => Op::LoadFalse,
            0x05 => Op::Pop,
            0x06 => Op::Dup,
            0x10 => Op::LoadLocal,
            0x11 => Op::StoreLocal,
            0x12 => Op::LoadGlobal,
            0x13 => Op::StoreGlobal,
            0x20 => Op::Add,
            0x21 => Op::Sub,
            0x22 => Op::Mul,
            0x23 => Op::Div,
            0x24 => Op::IDiv,
            0x25 => Op::Mod,
            0x26 => Op::Pow,
            0x27 => Op::Neg,
            0x28 => Op::Concat,
            0x30 => Op::Eq,
            0x31 => Op::Ne,
            0x32 => Op::Lt,
            0x33 => Op::Le,
            0x34 => Op::Gt,
            0x35 => Op::Ge,
            0x40 => Op::And,
            0x41 => Op::Or,
            0x42 => Op::Not,
            0x43 => Op::Xor,
            0x48 => Op::BAnd,
            0x49 => Op::BOr,
            0x4A => Op::BNot,
            0x4B => Op::BXor,
            0x4C => Op::Shl,
            0x4D => Op::Shr,
            0x50 => Op::Jump,
            0x51 => Op::JumpIf,
            0x52 => Op::JumpIfNot,
            0x60 => Op::CallSub,
            0x61 => Op::CallFunc,
            0x62 => Op::Ret,
            0x63 => Op::RetVal,
            0x64 => Op::CallBuiltin,
            0x70 => Op::CreateComp,
            0x71 => Op::SetProp,
            0x72 => Op::GetProp,
            0x73 => Op::CallMethod,
            0x74 => Op::RegisterEvent,
            0x80 => Op::NewArray,
            0x81 => Op::AGet,
            0x82 => Op::ASet,
            0x83 => Op::Redim,
            0x90 => Op::Print,
            0x91 => Op::PrintLn,
            0x92 => Op::Input,
            0xFE => Op::Halt,
            0xFF => Op::Nop,
            _ => return None,
        })
    }
}
