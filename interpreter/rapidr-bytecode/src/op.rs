//! Opcode definitions.
//!
//! All opcodes are a single `u8`. Operands follow inline as fixed-width
//! little-endian integers (no varint — favouring decode speed over size).
//! Operand widths per opcode are documented next to each variant, and
//! listed by [`Op::layout`].
//!
//! The instructions from `StoreLocalNum` on (2.123, docs/vm-fast-design.md)
//! are each one sequence of the older instructions fused into one — its
//! *expansion*, given in its documentation — and mean exactly what that
//! sequence means: the VM runs a fast path for numbers and the expansion's
//! own operations for anything else. A module that uses one needs the
//! 2.123 runtime ([`crate::FAST_OPS_SINCE`]).

/// An operand of a fused instruction ([`Op::fused_arith`],
/// [`Op::cmp_jump`]): where the value comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The value on the stack (computed before): no operand bytes.
    S = 0,
    /// A local slot: `u16`.
    L = 1,
    /// A global, by its name's string id: `u32`.
    G = 2,
    /// A constant of the pool: `u32`.
    K = 3,
}

impl Mode {
    /// Bytes of its operand.
    pub fn width(self) -> usize {
        match self {
            Mode::S => 0,
            Mode::L => 2,
            Mode::G | Mode::K => 4,
        }
    }

    fn of(bits: u8) -> Mode {
        match bits & 3 {
            0 => Mode::S,
            1 => Mode::L,
            2 => Mode::G,
            _ => Mode::K,
        }
    }

    /// Its operand's layout entry.
    fn operand(self) -> Option<Operand> {
        match self {
            Mode::S => None,
            Mode::L => Some(Operand::Local),
            Mode::G => Some(Operand::Global),
            Mode::K => Some(Operand::Const),
        }
    }
}

/// The arithmetic a fused instruction does ([`Op::fused_arith`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bin {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

impl Bin {
    /// The plain instruction.
    pub fn op(self) -> Op {
        match self {
            Bin::Add => Op::Add,
            Bin::Sub => Op::Sub,
            Bin::Mul => Op::Mul,
            Bin::Div => Op::Div,
            Bin::Mod => Op::Mod,
        }
    }
}

/// The comparison a compare-and-branch does ([`Op::cmp_jump`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmp {
    Eq = 0,
    Ne = 1,
    Lt = 2,
    Le = 3,
    Gt = 4,
    Ge = 5,
}

impl Cmp {
    const ALL: [Cmp; 6] = [Cmp::Eq, Cmp::Ne, Cmp::Lt, Cmp::Le, Cmp::Gt, Cmp::Ge];

    /// The plain instruction.
    pub fn op(self) -> Op {
        match self {
            Cmp::Eq => Op::Eq,
            Cmp::Ne => Op::Ne,
            Cmp::Lt => Op::Lt,
            Cmp::Le => Op::Le,
            Cmp::Gt => Op::Gt,
            Cmp::Ge => Op::Ge,
        }
    }
}

/// What an operand is, for a reader walking the code ([`Op::layout`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    /// `u8`: a count, an argument index, a conversion kind, dimensions.
    U8,
    /// `u16`: a local slot.
    Local,
    /// `u16`: an object's field slot.
    Field,
    /// `u32`: a global (its name's string id).
    Global,
    /// `u32`: a string of the pool (a name: a builtin, a component, a
    /// property, a method, an event).
    Str,
    /// `u32`: a constant of the pool.
    Const,
    /// `u32`: a function index.
    Func,
    /// `u32`: a jump target (an offset in the function's code).
    Target,
    /// `i32`: a size.
    I32,
}

impl Operand {
    pub fn width(self) -> usize {
        match self {
            Operand::U8 => 1,
            Operand::Local | Operand::Field => 2,
            Operand::Global | Operand::Str | Operand::Const | Operand::Func | Operand::Target | Operand::I32 => 4,
        }
    }
}

macro_rules! ops {
    ($( $(#[$meta:meta])* $name:ident = $value:expr ),* $(,)?) => {
        /// Bytecode operations.
        ///
        /// Operand encoding key:
        /// * `u8`/`u16`/`u32`/`i32`/`i64`: little-endian, raw.
        /// * No padding between operands.
        /// * Branch targets are absolute byte offsets within the function's `code`.
        #[repr(u8)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Op {
            $( $(#[$meta])* $name = $value ),*
        }

        /// Every opcode, once.
        const ALL: &[Op] = &[ $( Op::$name ),* ];

        /// Every opcode's byte, by its name (`raw::Add` is `Op::Add as
        /// u8`): what an interpreter matches the code's bytes against
        /// directly, without going through [`Op::from_u8`].
        #[allow(non_upper_case_globals)]
        pub mod raw {
            $( pub const $name: u8 = $value; )*
        }
    };
}

ops! {
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
    /// Push the final value of parameter `k` of the SUB/FUNCTION that just
    /// returned (for BYREF write-back into the caller's variable).
    /// Operand: `u8` parameter index.
    LoadArgOut = 0x65,
    /// GOSUB: remember the next instruction on this frame's GOSUB stack and
    /// jump. Operand: `u32` target.
    Gosub = 0x66,
    /// RETURN inside a routine that uses GOSUB: jump back to the most recent
    /// GOSUB if there is one, otherwise fall through (to the normal return).
    GosubRet = 0x67,

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
    /// Get a property of the object whose id is on the stack (a `Sender`
    /// parameter, `This`, …). Stack: [id] → [value]. Operand: `u32` prop.
    GetPropDyn = 0x75,
    /// Set a property of the object whose id is on the stack.
    /// Stack: [value, id] → []. Operand: `u32` prop.
    SetPropDyn = 0x76,
    /// Call a method of the object whose id is on the stack.
    /// Stack: [id, arg1, …, argN] → [result]. Operand: `u32` method, `u8` argc.
    CallMethodDyn = 0x77,
    /// Call through a function pointer (`CALLFUNC`): the routine's index + 1,
    /// as made by `BIND`/`CODEPTR`. Stack: [ptr, arg1, …, argN] → [result]
    /// (Null for a SUB). Operand: `u8` argc.
    CallIndirect = 0x78,
    /// Read field `slot` of an object (`Value::Object`, rapidr_ast::objects).
    /// Operand: `u16` slot. Stack: [object] → [value].
    GetField = 0x79,
    /// Set field `slot` of an object. Operand: `u16` slot.
    /// Stack: [object, value] → [].
    SetField = 0x7A,
    /// Convert for a store into a declared numeric type (rapidr_ast::numeric,
    /// `rapidr_value::numeric`). Operand: `u8` kind (`NumKind::code`).
    /// Stack: [value] → [converted].
    ToNum = 0x7B,

    // === arrays ===
    /// Create a new dynamic array of N elements (default Null).
    /// Create an array. Operand: `u8` number of dimensions `n`.
    /// Stack: [fill, lower1, upper1, …, lowerN, upperN] → [array].
    NewArray = 0x80,
    /// Read an element. Operand: `u8` index count `n`.
    /// Stack: [array, i1, …, iN] → [element]. Out of range is an error.
    AGet = 0x81,
    /// Write an element in place. Operand: `u8` index count `n`.
    /// Stack: [array, i1, …, iN, value] → []. Out of range is an error.
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
    /// Pad the output with spaces to the next 14-column print zone
    /// (`PRINT a, b`).
    PrintZone = 0x93,

    // ===== 2.123: fused instructions (docs/vm-fast-design.md) =====

    // === typed stores ===
    /// `ToNum kind; StoreLocal s`. Operands: `u16` s, `u8` kind.
    StoreLocalNum = 0x14,
    /// `ToNum kind; StoreGlobal g`. Operands: `u32` g, `u8` kind.
    StoreGlobalNum = 0x15,
    /// `LoadLocal s; ToNum kind; StoreLocal s` (a typed parameter on entry).
    /// Operands: `u16` s, `u8` kind.
    ConvLocal = 0x16,
    /// `LoadLocal s; ToNum ARG_ROUND; ToNum kind; StoreLocal s` (a BYVAL
    /// integer parameter on entry). Operands: `u16` s, `u8` kind.
    ConvArgLocal = 0x17,
    /// `LoadLocal s; StoreLocal d`. Operands: `u16` d, `u16` s.
    MoveLocal = 0x18,

    // === FOR ===
    /// The FOR loop's test, at its top: `LoadLocal step; LoadConst 0; Lt;
    /// JumpIfNot up; LoadLocal var; LoadLocal end; Ge; Jump test; up:
    /// LoadLocal var; LoadLocal end; Le; test: JumpIfNot exit`.
    /// Operands: `u16` var, `u16` end, `u16` step, `u32` exit.
    ForTestL = 0x53,
    /// [`Op::ForTestL`] with a global counter (`LoadGlobal var`).
    /// Operands: `u32` var, `u16` end, `u16` step, `u32` exit.
    ForTestG = 0x54,
    /// The FOR loop's step, after its body: `LoadLocal var; LoadLocal step;
    /// Add; StoreLocal var; Jump top`. Operands: `u16` var, `u16` step,
    /// `u32` top.
    ForStepL = 0x55,
    /// [`Op::ForStepL`] with a global counter. Operands: `u32` var, `u16`
    /// step, `u32` top.
    ForStepG = 0x56,

    // === one-index arrays (the array read after its index) ===
    /// `LoadLocal a; <index on the stack>; AGet 1`. Operand: `u16` a.
    AGetLS = 0x84,
    /// `LoadLocal a; LoadLocal i; AGet 1`. Operands: `u16` a, `u16` i.
    AGetLL = 0x85,
    /// `LoadLocal a; LoadGlobal i; AGet 1`. Operands: `u16` a, `u32` i.
    AGetLG = 0x86,
    /// `LoadGlobal a; <index on the stack>; AGet 1`. Operand: `u32` a.
    AGetGS = 0x87,
    /// `LoadGlobal a; LoadLocal i; AGet 1`. Operands: `u32` a, `u16` i.
    AGetGL = 0x88,
    /// `LoadGlobal a; LoadGlobal i; AGet 1`. Operands: `u32` a, `u32` i.
    AGetGG = 0x89,
    /// `a(index) = value` with the value and then the index on the stack
    /// (the order the compiler works them out in: `<value>; StoreLocal t;
    /// LoadLocal a; <index>; LoadLocal t; [ToNum kind;] ASet 1`).
    /// Operands: `u16` a, `u8` kind (`NO_KIND`: none).
    ASetLS = 0x8A,
    /// `LoadLocal a; LoadLocal i; <value>; [ToNum kind;] ASet 1`.
    /// Operands: `u16` a, `u16` i, `u8` kind.
    ASetLL = 0x8B,
    /// `LoadLocal a; LoadGlobal i; <value>; [ToNum kind;] ASet 1`.
    /// Operands: `u16` a, `u32` i, `u8` kind.
    ASetLG = 0x8C,
    /// [`Op::ASetLS`] of a global array. Operands: `u32` a, `u8` kind.
    ASetGS = 0x8D,
    /// `LoadGlobal a; LoadLocal i; <value>; [ToNum kind;] ASet 1`.
    /// Operands: `u32` a, `u16` i, `u8` kind.
    ASetGL = 0x8E,
    /// `LoadGlobal a; LoadGlobal i; <value>; [ToNum kind;] ASet 1`.
    /// Operands: `u32` a, `u32` i, `u8` kind.
    ASetGG = 0x8F,

    // === arithmetic with fused operands: `<a>; <b>; op`, the result
    // pushed (a on the stack: replaced). Opcode = base + 4 × a + b, modes
    // S 0, L 1, G 2, K 3 ([`Mode`]); `SS` is the plain instruction. ===
    AddSL = 0xA1, AddSG = 0xA2, AddSK = 0xA3,
    AddLS = 0xA4, AddLL = 0xA5, AddLG = 0xA6, AddLK = 0xA7,
    AddGS = 0xA8, AddGL = 0xA9, AddGG = 0xAA, AddGK = 0xAB,
    AddKS = 0xAC, AddKL = 0xAD, AddKG = 0xAE,
    SubSL = 0xB1, SubSG = 0xB2, SubSK = 0xB3,
    SubLS = 0xB4, SubLL = 0xB5, SubLG = 0xB6, SubLK = 0xB7,
    SubGS = 0xB8, SubGL = 0xB9, SubGG = 0xBA, SubGK = 0xBB,
    SubKS = 0xBC, SubKL = 0xBD, SubKG = 0xBE,
    MulSL = 0xC1, MulSG = 0xC2, MulSK = 0xC3,
    MulLS = 0xC4, MulLL = 0xC5, MulLG = 0xC6, MulLK = 0xC7,
    MulGS = 0xC8, MulGL = 0xC9, MulGG = 0xCA, MulGK = 0xCB,
    MulKS = 0xCC, MulKL = 0xCD, MulKG = 0xCE,
    DivSL = 0xD1, DivSG = 0xD2, DivSK = 0xD3,
    DivLS = 0xD4, DivLL = 0xD5, DivLG = 0xD6, DivLK = 0xD7,
    DivGS = 0xD8, DivGL = 0xD9, DivGG = 0xDA, DivGK = 0xDB,
    DivKS = 0xDC, DivKL = 0xDD, DivKG = 0xDE,
    /// `MOD` with fused operands (the forms programs use: an index, a
    /// counter wrapped).
    ModSL = 0x29, ModSK = 0x2A, ModSG = 0x2B, ModLL = 0x2C, ModLK = 0x2D, ModGG = 0x2E, ModGK = 0x2F,

    // === compare and branch: `<a>; <b>; cmp; JumpIfNot target`. Operands:
    // a's, b's, `u32` target. ===
    JumpIfNotEqSS = 0xE0, JumpIfNotEqLL = 0xE1, JumpIfNotEqLK = 0xE2, JumpIfNotEqGG = 0xE3, JumpIfNotEqGK = 0xE4,
    JumpIfNotNeSS = 0xE5, JumpIfNotNeLL = 0xE6, JumpIfNotNeLK = 0xE7, JumpIfNotNeGG = 0xE8, JumpIfNotNeGK = 0xE9,
    JumpIfNotLtSS = 0xEA, JumpIfNotLtLL = 0xEB, JumpIfNotLtLK = 0xEC, JumpIfNotLtGG = 0xED, JumpIfNotLtGK = 0xEE,
    JumpIfNotLeSS = 0xEF, JumpIfNotLeLL = 0xF0, JumpIfNotLeLK = 0xF1, JumpIfNotLeGG = 0xF2, JumpIfNotLeGK = 0xF3,
    JumpIfNotGtSS = 0xF4, JumpIfNotGtLL = 0xF5, JumpIfNotGtLK = 0xF6, JumpIfNotGtGG = 0xF7, JumpIfNotGtGK = 0xF8,
    JumpIfNotGeSS = 0xF9, JumpIfNotGeLL = 0xFA, JumpIfNotGeLK = 0xFB, JumpIfNotGeGG = 0xFC, JumpIfNotGeGK = 0xFD,
    JumpIfNotEqSL = 0x07, JumpIfNotNeSL = 0x08, JumpIfNotLtSL = 0x09, JumpIfNotLeSL = 0x0A, JumpIfNotGtSL = 0x0B, JumpIfNotGeSL = 0x0C,
    JumpIfNotEqSK = 0x36, JumpIfNotNeSK = 0x37, JumpIfNotLtSK = 0x38, JumpIfNotLeSK = 0x39, JumpIfNotGtSK = 0x3A, JumpIfNotGeSK = 0x3B,
    JumpIfNotEqSG = 0x68, JumpIfNotNeSG = 0x69, JumpIfNotLtSG = 0x6A, JumpIfNotLeSG = 0x6B, JumpIfNotGtSG = 0x6C, JumpIfNotGeSG = 0x6D,

    // === misc ===
    Halt = 0xFE,
    Nop  = 0xFF,
}

/// The `kind` operand of a fused array store without a conversion.
pub const NO_KIND: u8 = 0xFF;

/// Compare-and-branch forms, in their opcode block's order (0xE0 on).
const CMP_BLOCK_FORMS: [(Mode, Mode); 5] = [(Mode::S, Mode::S), (Mode::L, Mode::L), (Mode::L, Mode::K), (Mode::G, Mode::G), (Mode::G, Mode::K)];

/// `MOD`'s fused forms, in opcode order (0x29 on).
const MOD_FORMS: [(Mode, Mode); 7] = [(Mode::S, Mode::L), (Mode::S, Mode::K), (Mode::S, Mode::G), (Mode::L, Mode::L), (Mode::L, Mode::K), (Mode::G, Mode::G), (Mode::G, Mode::K)];

impl Op {
    /// How many bytes of operands follow the opcode (what a reader of the
    /// code — the debugger's list of the calls on a line — skips).
    pub fn operand_len(self) -> usize {
        self.layout().iter().map(|o| o.width()).sum()
    }

    /// The operands that follow the opcode, in order.
    pub fn layout(self) -> &'static [Operand] {
        use Operand::*;
        if let Some((_, a, b)) = self.as_fused_arith() {
            return mode_layout(a, b, false);
        }
        if let Some((_, a, b)) = self.as_cmp_jump() {
            return mode_layout(a, b, true);
        }
        match self {
            Op::LoadConst => &[Const],
            Op::LoadGlobal | Op::StoreGlobal => &[Global],
            Op::Jump | Op::JumpIf | Op::JumpIfNot | Op::Gosub => &[Target],
            Op::GetPropDyn | Op::SetPropDyn => &[Str],
            Op::LoadLocal | Op::StoreLocal => &[Local],
            Op::GetField | Op::SetField => &[Field],
            Op::LoadArgOut | Op::CallIndirect | Op::ToNum | Op::NewArray | Op::AGet | Op::ASet => &[U8],
            Op::CallSub | Op::CallFunc => &[Func, U8],
            Op::CallBuiltin | Op::CallMethodDyn => &[Str, U8],
            Op::CreateComp | Op::SetProp | Op::GetProp => &[Str, Str],
            Op::CallMethod => &[Str, Str, U8],
            Op::RegisterEvent => &[Str, Str, Func],
            Op::Redim => &[Local, I32],
            Op::StoreLocalNum | Op::ConvLocal | Op::ConvArgLocal => &[Local, U8],
            Op::StoreGlobalNum => &[Global, U8],
            Op::MoveLocal => &[Local, Local],
            Op::ForTestL => &[Local, Local, Local, Target],
            Op::ForTestG => &[Global, Local, Local, Target],
            Op::ForStepL => &[Local, Local, Target],
            Op::ForStepG => &[Global, Local, Target],
            Op::AGetLS => &[Local],
            Op::AGetLL => &[Local, Local],
            Op::AGetLG => &[Local, Global],
            Op::AGetGS => &[Global],
            Op::AGetGL => &[Global, Local],
            Op::AGetGG => &[Global, Global],
            Op::ASetLS => &[Local, U8],
            Op::ASetLL => &[Local, Local, U8],
            Op::ASetLG => &[Local, Global, U8],
            Op::ASetGS => &[Global, U8],
            Op::ASetGL => &[Global, Local, U8],
            Op::ASetGG => &[Global, Global, U8],
            _ => &[],
        }
    }

    /// A fused arithmetic instruction's operation and operand modes
    /// (`AddLK` → Add, L, K).
    pub fn as_fused_arith(self) -> Option<(Bin, Mode, Mode)> {
        let b = self as u8;
        let bin = match b & 0xF0 {
            0xA0 => Bin::Add,
            0xB0 => Bin::Sub,
            0xC0 => Bin::Mul,
            0xD0 => Bin::Div,
            _ => {
                let i = b.checked_sub(Op::ModSL as u8).filter(|&i| (i as usize) < MOD_FORMS.len())?;
                let (a, m) = MOD_FORMS[i as usize];
                return Some((Bin::Mod, a, m));
            }
        };
        let (a, m) = (Mode::of(b >> 2), Mode::of(b));
        // (SS is the plain instruction, KK isn't one)
        (a != m || a == Mode::L || a == Mode::G).then_some((bin, a, m))
    }

    /// The fused instruction for `a op b`, if there is one.
    pub fn fused_arith(bin: Bin, a: Mode, b: Mode) -> Option<Op> {
        let code = match bin {
            Bin::Add => 0xA0,
            Bin::Sub => 0xB0,
            Bin::Mul => 0xC0,
            Bin::Div => 0xD0,
            Bin::Mod => {
                let i = MOD_FORMS.iter().position(|&f| f == (a, b))?;
                return Op::from_u8(Op::ModSL as u8 + i as u8);
            }
        } + 4 * a as u8
            + b as u8;
        Op::from_u8(code).filter(|op| op.as_fused_arith() == Some((bin, a, b)))
    }

    /// A compare-and-branch's comparison and operand modes
    /// (`JumpIfNotLtLK` → Lt, L, K).
    pub fn as_cmp_jump(self) -> Option<(Cmp, Mode, Mode)> {
        let b = self as u8;
        if (0xE0..=0xFD).contains(&b) {
            let i = (b - 0xE0) as usize;
            let (a, m) = CMP_BLOCK_FORMS[i % 5];
            return Some((Cmp::ALL[i / 5], a, m));
        }
        let (base, right) = match b {
            0x07..=0x0C => (0x07, Mode::L),
            0x36..=0x3B => (0x36, Mode::K),
            0x68..=0x6D => (0x68, Mode::G),
            _ => return None,
        };
        Some((Cmp::ALL[(b - base) as usize], Mode::S, right))
    }

    /// The compare-and-branch for `IF NOT (a cmp b)`, if there is one.
    pub fn cmp_jump(cmp: Cmp, a: Mode, b: Mode) -> Option<Op> {
        if let Some(form) = CMP_BLOCK_FORMS.iter().position(|&f| f == (a, b)) {
            return Op::from_u8(0xE0 + 5 * cmp as u8 + form as u8);
        }
        let base = match (a, b) {
            (Mode::S, Mode::L) => 0x07,
            (Mode::S, Mode::K) => 0x36,
            (Mode::S, Mode::G) => 0x68,
            _ => return None,
        };
        Op::from_u8(base + cmp as u8)
    }

    /// Which of the decoded operands is a conversion's kind
    /// (`NumKind::code`, `NO_KIND`, `ARG_ROUND`), if one is.
    pub fn kind_operand(self) -> Option<usize> {
        match self {
            Op::ToNum => Some(0),
            Op::StoreLocalNum | Op::ConvLocal | Op::ConvArgLocal | Op::StoreGlobalNum | Op::ASetLS | Op::ASetGS => Some(1),
            Op::ASetLL | Op::ASetLG | Op::ASetGL | Op::ASetGG => Some(2),
            _ => None,
        }
    }

    /// Whether this is one of 2.123's fused instructions (a module using
    /// one needs [`crate::FAST_OPS_SINCE`]).
    pub fn is_fused(self) -> bool {
        self.as_fused_arith().is_some()
            || self.as_cmp_jump().is_some()
            || matches!(
                self,
                Op::StoreLocalNum
                    | Op::StoreGlobalNum
                    | Op::ConvLocal
                    | Op::ConvArgLocal
                    | Op::MoveLocal
                    | Op::ForTestL
                    | Op::ForTestG
                    | Op::ForStepL
                    | Op::ForStepG
                    | Op::AGetLS
                    | Op::AGetLL
                    | Op::AGetLG
                    | Op::AGetGS
                    | Op::AGetGL
                    | Op::AGetGG
                    | Op::ASetLS
                    | Op::ASetLL
                    | Op::ASetLG
                    | Op::ASetGS
                    | Op::ASetGL
                    | Op::ASetGG
            )
    }

    /// The global an instruction stores into (its operand), for a reader
    /// that finds where the program first sets each global.
    pub fn stored_global(self, operands: &[u8]) -> Option<u32> {
        match self {
            Op::StoreGlobal | Op::StoreGlobalNum | Op::ForStepG => operands.get(..4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])),
            _ => None,
        }
    }

    /// Try to convert from raw byte: one look in a table made at compile
    /// time (the VM asks for every instruction it runs).
    #[inline(always)]
    pub fn from_u8(b: u8) -> Option<Op> {
        OPS[b as usize]
    }
}

/// A fused instruction's operand layout: a's, b's, then the target of a
/// compare-and-branch.
fn mode_layout(a: Mode, b: Mode, target: bool) -> &'static [Operand] {
    use Operand::{Const as K, Global as G, Local as L, Target as T};
    match (a.operand(), b.operand(), target) {
        (None, None, false) => &[],
        (None, None, true) => &[T],
        (Some(x), None, false) | (None, Some(x), false) => match x {
            L => &[L],
            G => &[G],
            _ => &[K],
        },
        (Some(x), None, true) | (None, Some(x), true) => match x {
            L => &[L, T],
            G => &[G, T],
            _ => &[K, T],
        },
        (Some(x), Some(y), t) => match (x, y, t) {
            (L, L, false) => &[L, L],
            (L, G, false) => &[L, G],
            (L, K, false) => &[L, K],
            (G, L, false) => &[G, L],
            (G, G, false) => &[G, G],
            (G, K, false) => &[G, K],
            (K, L, false) => &[K, L],
            (K, G, false) => &[K, G],
            (L, L, true) => &[L, L, T],
            (L, K, true) => &[L, K, T],
            (G, G, true) => &[G, G, T],
            (G, K, true) => &[G, K, T],
            _ => &[],
        },
    }
}

/// Every byte's opcode ([`Op::from_u8`]).
static OPS: [Option<Op>; 256] = {
    let mut t = [None; 256];
    let mut i = 0;
    while i < ALL.len() {
        t[ALL[i] as u8 as usize] = Some(ALL[i]);
        i += 1;
    }
    t
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_the_opcodes() {
        for b in 0..=255u8 {
            if let Some(op) = Op::from_u8(b) {
                assert_eq!(op as u8, b);
            }
        }
        assert_eq!(Op::from_u8(0x20), Some(Op::Add));
        assert_eq!(Op::from_u8(0x00), None);
        assert_eq!(Op::from_u8(0xFE), Some(Op::Halt));
        // (every opcode once: two variants with one value don't compile,
        // and the table holds each)
        assert_eq!(OPS.iter().flatten().count(), ALL.len());
    }

    #[test]
    fn fused_instructions_name_their_forms() {
        let modes = [Mode::S, Mode::L, Mode::G, Mode::K];
        let mut arith = 0;
        for bin in [Bin::Add, Bin::Sub, Bin::Mul, Bin::Div, Bin::Mod] {
            for a in modes {
                for b in modes {
                    if let Some(op) = Op::fused_arith(bin, a, b) {
                        arith += 1;
                        assert_eq!(op.as_fused_arith(), Some((bin, a, b)), "{op:?}");
                        assert_eq!(op.operand_len(), a.width() + b.width(), "{op:?}");
                        assert!(op.is_fused());
                        // (the name says the form: AddLK)
                        let name = format!("{op:?}");
                        assert!(name.ends_with(&format!("{a:?}{b:?}")), "{name}");
                        assert!(name.starts_with(&format!("{bin:?}")), "{name}");
                    }
                }
            }
        }
        assert_eq!(arith, 4 * 14 + 7);
        assert_eq!(Op::fused_arith(Bin::Add, Mode::S, Mode::S), None, "the plain Add");
        assert_eq!(Op::fused_arith(Bin::Add, Mode::K, Mode::K), None);
        let mut cmp = 0;
        for c in Cmp::ALL {
            for a in modes {
                for b in modes {
                    if let Some(op) = Op::cmp_jump(c, a, b) {
                        cmp += 1;
                        assert_eq!(op.as_cmp_jump(), Some((c, a, b)), "{op:?}");
                        assert_eq!(op.operand_len(), a.width() + b.width() + 4, "{op:?}");
                        let name = format!("{op:?}");
                        assert_eq!(name, format!("JumpIfNot{c:?}{a:?}{b:?}"));
                    }
                }
            }
        }
        assert_eq!(cmp, 6 * 8);
        // (the older instructions are none of these)
        for op in [Op::Add, Op::Lt, Op::JumpIfNot, Op::Halt, Op::Nop, Op::LoadConst, Op::ToNum] {
            assert!(!op.is_fused() && op.as_fused_arith().is_none() && op.as_cmp_jump().is_none(), "{op:?}");
        }
        for op in ALL {
            if op.is_fused() {
                assert!((*op as u8) < 0xFE);
            }
        }
    }

    #[test]
    fn layouts_are_the_operand_lengths() {
        assert_eq!(Op::LoadConst.operand_len(), 4);
        assert_eq!(Op::CallMethod.operand_len(), 9);
        assert_eq!(Op::RegisterEvent.operand_len(), 12);
        assert_eq!(Op::Redim.operand_len(), 6);
        assert_eq!(Op::ForTestG.operand_len(), 12);
        assert_eq!(Op::ASetGL.operand_len(), 7);
        assert_eq!(Op::Halt.operand_len(), 0);
    }
}
