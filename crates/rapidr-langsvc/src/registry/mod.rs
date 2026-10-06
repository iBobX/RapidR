//! The language registry as the language service reads it — **interim**.
//!
//! The seam: these types and functions are a subset of `rapidr-lang`'s
//! (lane L-REG, docs/ide-plan.md I0), with the same names and fields, so
//! when the registry lands this module becomes
//! `pub use rapidr_lang::{component, builtin, builtin_key, constant, Component, …}`
//! plus the few lists below, and nothing else in the service changes. Until
//! then the data (`data.rs`) is generated from the legacy hand-written
//! tables, and the statements' and keywords' docs are here.

mod data;

/// Where a name comes from: RapidQ or a RapidR extension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    RapidQ,
    RapidR,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    ReadWrite,
    Read,
    Write,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Property {
    pub name: &'static str,
    pub access: Access,
    pub origin: Origin,
    pub doc: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Param {
    pub name: &'static str,
    /// BASIC's type, "" when any.
    pub ty: &'static str,
    pub optional: bool,
    pub byref: bool,
    pub variadic: bool,
    pub default: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Method {
    pub name: &'static str,
    pub params: &'static [Param],
    pub returns: Option<&'static str>,
    /// Reading it without parentheses calls it (`IF Dlg.Execute THEN`).
    pub value: bool,
    pub origin: Origin,
    pub doc: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub name: &'static str,
    pub params: &'static [Param],
    pub origin: Origin,
    pub doc: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Component {
    /// RapidR's name (`RBUTTON`).
    pub name: &'static str,
    /// RapidQ's name, when RapidQ has the component.
    pub rapidq: Option<&'static str>,
    pub origin: Origin,
    pub doc: &'static str,
    pub properties: &'static [Property],
    pub methods: &'static [Method],
    pub events: &'static [Event],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Builtin {
    /// As programs write it (`MID$`).
    pub name: &'static str,
    /// The runtimes' dispatch key (`mid`).
    pub key: &'static str,
    /// How it's called (`MID$(text, start [, count])`).
    pub syntax: &'static str,
    pub params: &'static [Param],
    pub returns: Option<&'static str>,
    pub group: &'static str,
    pub bare: bool,
    pub origin: Origin,
    pub doc: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Statement {
    pub name: &'static str,
    pub syntax: &'static str,
    pub group: &'static str,
    pub origin: Origin,
    pub doc: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Directive {
    pub name: &'static str,
    pub syntax: &'static str,
    pub origin: Origin,
    pub doc: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypeName {
    pub name: &'static str,
    pub origin: Origin,
    pub doc: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConstantGroup {
    pub name: &'static str,
    /// Where a program gets them: `RAPIDQ.INC`, `RapidR`.
    pub source: &'static str,
    pub origin: Origin,
    pub doc: &'static str,
    pub constants: &'static [(&'static str, i64)],
}

pub use data::{BUILTINS, COMPONENTS, DIRECTIVES, TYPE_NAMES};

/// The component one of these names means: RapidR's (`RBUTTON`), RapidQ's
/// (`QBUTTON`) or an alias (`QGAUGE`), any case.
pub fn component(name: &str) -> Option<&'static Component> {
    let canonical = rapidr_ast::canonical_type_name(name).to_ascii_uppercase();
    COMPONENTS.iter().find(|c| c.name == canonical || c.rapidq.is_some_and(|q| q.eq_ignore_ascii_case(name)))
}

/// A builtin by name or key (`MID$`, `mid`, `Mid`).
pub fn builtin(name: &str) -> Option<&'static Builtin> {
    let key = builtin_key(name);
    BUILTINS.iter().find(|b| b.key == key)
}

/// The runtimes' dispatch key of a builtin name: lower case, one BASIC type
/// suffix dropped.
pub fn builtin_key(name: &str) -> String {
    let mut key = name.to_ascii_lowercase();
    if key.len() > 1 && matches!(key.chars().last(), Some('$' | '%' | '#' | '&' | '!')) {
        key.pop();
    }
    key
}

/// Whether the compilers accept `name` as a builtin (the bytecode table:
/// what every runtime answers), documented or not.
pub fn is_builtin_name(name: &str) -> bool {
    builtin(name).is_some() || rapidr_bytecode::builtins::is_builtin(name)
}

/// Every builtin name the runtimes answer that programs write (the
/// compiler's `__` helpers left out), upper case.
pub fn runtime_builtin_names() -> impl Iterator<Item = String> {
    rapidr_bytecode::builtins::BUILTINS
        .iter()
        .filter(|k| !k.starts_with("__") && !k.contains('.') && !k.contains('_'))
        .map(|k| k.to_ascii_uppercase())
}

pub static CONSTANT_GROUPS: &[ConstantGroup] = &[
    ConstantGroup {
        name: "RapidQ",
        source: "RAPIDQ.INC",
        origin: Origin::RapidQ,
        doc: "RapidQ's constants, there once a program has `$INCLUDE \"RAPIDQ.INC\"`.",
        constants: rapidr_preprocessor::RAPIDQ_INC_CONSTANTS,
    },
    ConstantGroup {
        name: "RapidR",
        source: "RapidR",
        origin: Origin::RapidR,
        doc: "RapidR's own constants, always there.",
        constants: rapidr_ast::RAPIDR_CONSTANTS,
    },
];

/// A constant's value by name (any case), and its group.
pub fn constant(name: &str) -> Option<(i64, &'static ConstantGroup)> {
    CONSTANT_GROUPS.iter().find_map(|g| g.constants.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, v)| (*v, g)))
}

/// A directive by name (`$INCLUDE`, `INCLUDE`).
pub fn directive(name: &str) -> Option<&'static Directive> {
    let n = name.trim_start_matches('$');
    DIRECTIVES.iter().find(|d| d.name.trim_start_matches('$').eq_ignore_ascii_case(n))
}

pub fn type_name(name: &str) -> Option<&'static TypeName> {
    TYPE_NAMES.iter().find(|t| t.name.eq_ignore_ascii_case(name))
}

pub fn statement(name: &str) -> Option<&'static Statement> {
    STATEMENTS.iter().find(|s| s.name.eq_ignore_ascii_case(name))
}

impl Component {
    pub fn property(&self, name: &str) -> Option<&'static Property> {
        self.properties.iter().find(|p| p.name.eq_ignore_ascii_case(name))
    }

    pub fn method(&self, name: &str) -> Option<&'static Method> {
        self.methods.iter().find(|m| m.name.eq_ignore_ascii_case(name))
    }

    pub fn event(&self, name: &str) -> Option<&'static Event> {
        self.events.iter().find(|e| e.name.eq_ignore_ascii_case(name))
    }

    pub fn has_member(&self, name: &str) -> bool {
        self.property(name).is_some() || self.method(name).is_some() || self.event(name).is_some()
    }

    /// The name the IDE writes for a new one: RapidQ's for RapidQ's
    /// components, RapidR's for its own (docs/q-and-r-components.md §4).
    pub fn written_name(&self) -> &'static str {
        self.rapidq.unwrap_or(self.name)
    }
}

impl Param {
    /// `[BYREF] Name [AS Type]`, in brackets when optional.
    pub fn text(&self) -> String {
        let mut s = String::new();
        if self.byref {
            s.push_str("BYREF ");
        }
        s.push_str(self.name);
        if !self.ty.is_empty() {
            s.push_str(" AS ");
            s.push_str(self.ty);
        }
        if let Some(d) = self.default {
            s.push_str(" = ");
            s.push_str(d);
        }
        if self.variadic {
            s.push_str(", …");
        }
        if self.optional {
            s = format!("[{s}]");
        }
        s
    }
}

macro_rules! statements {
    ($( $name:literal, $syntax:literal, $origin:ident, $doc:literal; )*) => {
        /// Statements and keywords, with their docs (RapidR's own words).
        pub static STATEMENTS: &[Statement] = &[ $( Statement { name: $name, syntax: $syntax, group: "", origin: Origin::$origin, doc: $doc }, )* ];
    };
}

statements! {
    "DIM", "DIM name[(bounds)] AS type", RapidQ, "Declares variables, arrays and components.";
    "REDIM", "REDIM name(bounds) [AS type]", RapidQ, "Changes an array's size; its data is kept.";
    "CONST", "CONST name [AS type] = value", RapidQ, "Declares a constant.";
    "AS", "… AS type", RapidQ, "Names the type in a declaration.";
    "IF", "IF condition THEN … [ELSEIF …] [ELSE …] END IF", RapidQ, "Runs code when a condition is true.";
    "THEN", "IF condition THEN", RapidQ, "Follows an IF or ELSEIF condition.";
    "ELSE", "ELSE", RapidQ, "The branch of an IF taken when no condition was true.";
    "ELSEIF", "ELSEIF condition THEN", RapidQ, "Another condition of an IF block.";
    "FOR", "FOR var = start TO end [STEP n] … NEXT [var]", RapidQ, "Repeats code for each value of a counter.";
    "TO", "FOR var = start TO end", RapidQ, "The last value of a FOR counter (or of a CASE range).";
    "STEP", "STEP n", RapidQ, "How much a FOR counter changes each time (1 by default).";
    "NEXT", "NEXT [var]", RapidQ, "Ends a FOR loop.";
    "WHILE", "WHILE condition … WEND", RapidQ, "Repeats code while a condition is true.";
    "WEND", "WEND", RapidQ, "Ends a WHILE loop.";
    "DO", "DO [WHILE|UNTIL condition] … LOOP [WHILE|UNTIL condition]", RapidQ, "Repeats code, testing a condition at the start or the end.";
    "LOOP", "LOOP [WHILE|UNTIL condition]", RapidQ, "Ends a DO loop.";
    "UNTIL", "DO UNTIL condition", RapidQ, "Repeats until the condition is true.";
    "SELECT", "SELECT CASE expr … END SELECT", RapidQ, "Picks the branch whose CASE matches a value.";
    "CASE", "CASE value[, value | a TO b | IS > n]", RapidQ, "A branch of SELECT CASE.";
    "SUB", "SUB Name[(params)] … END SUB", RapidQ, "Declares a procedure.";
    "FUNCTION", "FUNCTION Name[(params)] [AS type] … END FUNCTION", RapidQ, "Declares a function: assign its name (or Result) to return a value.";
    "SUBI", "SUBI Name(...) … END SUBI", RapidQ, "A procedure taking any number of arguments (ParamStr$, ParamVal).";
    "FUNCTIONI", "FUNCTIONI Name(...) … END FUNCTIONI", RapidQ, "A function taking any number of arguments (ParamStr$, ParamVal).";
    "CALL", "CALL Name[(args)]", RapidQ, "Calls a SUB.";
    "RETURN", "RETURN", RapidQ, "Goes back after a GOSUB.";
    "EXIT", "EXIT FOR | DO | WHILE | SUB | FUNCTION", RapidQ, "Leaves the loop or procedure early.";
    "PRINT", "PRINT [items][;|,]", RapidQ, "Writes to the console.";
    "INPUT", "INPUT [\"prompt\";] var", RapidQ, "Reads a line from the console into a variable.";
    "GOTO", "GOTO label", RapidQ, "Jumps to a label.";
    "GOSUB", "GOSUB label", RapidQ, "Jumps to a label; RETURN comes back.";
    "CREATE", "CREATE name AS type … END CREATE", RapidQ, "Creates a component and sets its properties; components created inside belong to it.";
    "TYPE", "TYPE Name [EXTENDS base] … END TYPE", RapidQ, "Declares a record or an object type with fields, methods and events.";
    "EXTENDS", "TYPE Name EXTENDS base", RapidQ, "Makes a TYPE an extension of a component or another TYPE.";
    "DECLARE", "DECLARE SUB|FUNCTION Name LIB \"dll\" [ALIAS \"name\"] (params) [AS type]", RapidQ, "Declares a routine of a DLL (or a SUB / FUNCTION before it's written).";
    "LIB", "DECLARE … LIB \"file.dll\"", RapidQ, "The DLL a DECLARE'd routine is in.";
    "ALIAS", "DECLARE … ALIAS \"name\"", RapidQ, "The routine's name in the DLL.";
    "WITH", "WITH object … END WITH", RapidQ, "Inside, `.Member` means the object's member.";
    "BIND", "BIND var TO routine", RapidQ, "Stores a routine in a variable, to call it later (CALLFUNC).";
    "CONSTRUCTOR", "CONSTRUCTOR … END CONSTRUCTOR", RapidQ, "A TYPE's initial values for every new instance.";
    "PROPERTY", "PROPERTY SET name(value)", RapidQ, "A TYPE's property, set through a method.";
    "BYREF", "BYREF param", RapidQ, "The parameter is the caller's variable: changing it changes theirs.";
    "BYVAL", "BYVAL param", RapidQ, "The parameter is a copy of the caller's value.";
    "AND", "a AND b", RapidQ, "Logical and bitwise AND.";
    "OR", "a OR b", RapidQ, "Logical and bitwise OR.";
    "NOT", "NOT a", RapidQ, "Logical and bitwise NOT.";
    "XOR", "a XOR b", RapidQ, "Logical and bitwise exclusive OR.";
    "MOD", "a MOD b", RapidQ, "The remainder of a division.";
    "SHL", "a SHL n", RapidQ, "Shifts the bits left.";
    "SHR", "a SHR n", RapidQ, "Shifts the bits right.";
    "END", "END [IF|SUB|FUNCTION|SELECT|TYPE|CREATE|WITH]", RapidQ, "Ends the program, or closes a block.";
    "STATIC", "STATIC var AS type", RapidQ, "A variable of a SUB / FUNCTION that keeps its value between calls.";
    "OPEN", "OPEN file FOR INPUT|OUTPUT|APPEND|BINARY|RANDOM AS #n", RapidQ, "Opens a file by number.";
    "CLOSE", "CLOSE [#n]", RapidQ, "Closes a file opened by number.";
    "DEFINT", "DEFINT var[, var]", RapidQ, "Declares INTEGER variables.";
    "DEFSTR", "DEFSTR var[, var]", RapidQ, "Declares STRING variables.";
    "DEFDBL", "DEFDBL var[, var]", RapidQ, "Declares DOUBLE variables.";
    "DEFSNG", "DEFSNG var[, var]", RapidQ, "Declares SINGLE variables.";
    "DEFLNG", "DEFLNG var[, var]", RapidQ, "Declares LONG variables.";
    "DEFBYTE", "DEFBYTE var[, var]", RapidQ, "Declares BYTE variables.";
    "DEFWORD", "DEFWORD var[, var]", RapidQ, "Declares WORD variables.";
    "DEFDWORD", "DEFDWORD var[, var]", RapidQ, "Declares DWORD variables.";
    "DATA", "DATA value[, value]", RapidQ, "Values for READ.";
    "READ", "READ var[, var]", RapidQ, "Takes the next DATA values.";
    "RESTORE", "RESTORE [label]", RapidQ, "Makes READ start again (from a label).";
    "SWAP", "SWAP a, b", RapidQ, "Exchanges two variables' values.";
    "INC", "INC var[, n]", RapidQ, "Adds 1 (or n) to a variable.";
    "DEC", "DEC var[, n]", RapidQ, "Subtracts 1 (or n) from a variable.";
    "IS", "CASE IS > n", RapidQ, "Compares the SELECT value in a CASE.";
    "TRUE", "TRUE", RapidQ, "The value of a true condition.";
    "FALSE", "FALSE", RapidQ, "0, the value of a false condition.";
    "RESULT", "Result = value", RapidQ, "Inside a FUNCTION: the value it returns.";
    "THIS", "This.member", RapidQ, "Inside a TYPE's methods and events: the instance.";
    "SUPER", "Super.member", RapidQ, "Inside a TYPE … EXTENDS: the base's member.";
    "REM", "REM comment", RapidQ, "A comment to the end of the line (as `'`).";
    "IMPORT", "IMPORT module", RapidR, "Imports a RapidR module.";
}
