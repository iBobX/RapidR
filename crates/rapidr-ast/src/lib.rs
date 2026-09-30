use rapidr_diagnostics::TextSpan;

pub mod numeric;
pub mod objects;
pub mod array_refs;
pub mod suffix_routines;
pub mod memory;

/// A name without its type suffix (`n%` → `n`, `w??` → `w`).
pub fn strip_type_suffix(name: &str) -> &str {
    let base = name.trim_end_matches('?');
    let base = if base.len() < name.len() { base } else { name.strip_suffix(['$', '%', '&', '!', '#']).unwrap_or(name) };
    if base.is_empty() { name } else { base }
}

/// The type a suffix declares (RapidQ manual, data types): `?` BYTE, `??`
/// WORD, `???` DWORD, `%` SHORT, `&` LONG, `!` SINGLE, `#` DOUBLE, `$`
/// STRING.
pub fn suffix_type(name: &str) -> Option<&'static str> {
    let qs = name.len() - name.trim_end_matches('?').len();
    Some(match (qs, name.chars().last()?) {
        (1, _) => "BYTE",
        (2, _) => "WORD",
        (3, _) => "DWORD",
        (_, '%') => "SHORT",
        (_, '&') => "LONG",
        (_, '!') => "SINGLE",
        (_, '#') => "DOUBLE",
        (_, '$') => "STRING",
        _ => return None,
    })
    .filter(|_| strip_type_suffix(name).len() < name.len())
}
pub mod stream_arrays;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub span: TextSpan,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Assignment(AssignmentStatement),
    Bind(BindStatement),
    Call(CallStatement),
    Close(CloseStatement),
    Comment(CommentStatement),
    Const(ConstStatement),
    Create(CreateStatement),
    Declare(DeclareStatement),
    Dim(DimStatement),
    Directive(DirectiveStatement),
    DoLoop(DoLoopStatement),
    Exit(ExitStatement),
    For(ForStatement),
    Function(FunctionStatement),
    /// `GOSUB label`
    Gosub(JumpStatement),
    /// `GOTO label`
    Goto(JumpStatement),
    If(IfStatement),
    Import(ImportStatement),
    Input(InputStatement),
    /// `name:` or a line number at the start of a line. A bare `name:` may
    /// also be a call to a zero-argument SUB followed by `:`; code
    /// generators decide by looking the name up.
    Label(LabelStatement),
    Line(LineStatement),
    Open(OpenStatement),
    Print(PrintStatement),
    PrintHash(PrintHashStatement),
    Return(ReturnStatement),
    Seek(SeekStatement),
    SelectCase(SelectCaseStatement),
    Subroutine(SubroutineStatement),
    Type(TypeStatement),
    While(WhileStatement),
    With(WithStatement),
    WriteHash(WriteHashStatement),
    RustBlock(RustBlockStatement),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssignmentStatement {
    pub span: TextSpan,
    pub target: Expression,
    pub value: Expression,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallStatement {
    pub span: TextSpan,
    pub callee: Expression,
    pub args: Vec<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstStatement {
    pub span: TextSpan,
    pub name: String,
    pub declared_type: Option<String>,
    pub value: Expression,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DimStatement {
    pub span: TextSpan,
    pub declarators: Vec<VariableDeclarator>,
    pub type_name: String,
    /// `AS STRING * 20`: the fixed length (stores are cut to it).
    pub fixed_len: Option<usize>,
    /// `STATIC x AS T` inside a SUB/FUNCTION: one variable shared by every
    /// call (and recursion) of that procedure, initialised once.
    pub is_static: bool,
    /// `REDIM a(n) AS T`: resize keeping the data (`rapidr_value::redim`).
    pub is_redim: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariableDeclarator {
    pub span: TextSpan,
    pub name: String,
    pub dimensions: Vec<ArrayDimension>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArrayDimension {
    Single(Expression),
    Range { start: Expression, end: Expression },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectiveStatement {
    pub span: TextSpan,
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportStatement {
    pub span: TextSpan,
    pub module_name: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineStatement {
    pub span: TextSpan,
    pub kind: LineStatementKind,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrintStatement {
    pub span: TextSpan,
    pub items: Vec<Expression>,
    /// `zones[i]`: item i is followed by `,` (move to the next 14-column
    /// print zone). Items followed by `;` or nothing are printed directly.
    pub zones: Vec<bool>,
    /// False when the statement ends with `;` or `,` (stay on the line).
    pub append_newline: bool,
}

// --- Block constructs ---

#[derive(Debug, Clone, PartialEq)]
pub struct IfStatement {
    pub span: TextSpan,
    pub condition: Expression,
    pub then_body: Vec<Statement>,
    pub elseif_branches: Vec<ElseIfBranch>,
    pub else_body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ElseIfBranch {
    pub span: TextSpan,
    pub condition: Expression,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForStatement {
    pub span: TextSpan,
    pub variable: String,
    pub start: Expression,
    pub end: Expression,
    pub step: Option<Expression>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhileStatement {
    pub span: TextSpan,
    pub condition: Expression,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DoLoopStatement {
    pub span: TextSpan,
    pub condition: Option<Expression>,
    pub pre_condition: bool,
    pub is_until: bool,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectCaseStatement {
    pub span: TextSpan,
    pub expression: Expression,
    pub cases: Vec<CaseBranch>,
    pub case_else: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaseBranch {
    pub span: TextSpan,
    /// The branch matches when any of these matches (`CASE 1, 5 TO 9, IS > 20`).
    pub values: Vec<CaseValue>,
    pub body: Vec<Statement>,
}

/// One item of a `CASE` list, tested against the SELECT expression.
#[derive(Debug, Clone, PartialEq)]
pub enum CaseValue {
    /// `CASE 3` — equal to the value.
    Value(Expression),
    /// `CASE 1 TO 5` — between the bounds, inclusive.
    Range(Expression, Expression),
    /// `CASE IS > 10` — the comparison holds (`=`, `<>`, `<`, `<=`, `>`, `>=`).
    Is(BinaryOperator, Expression),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubroutineStatement {
    pub span: TextSpan,
    pub name: String,
    pub params: Vec<Parameter>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionStatement {
    pub span: TextSpan,
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Option<String>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub span: TextSpan,
    pub name: String,
    pub type_name: String,
    pub by_ref: bool,
    /// `list() AS STRING`: an array parameter (arrays are shared with the caller).
    pub is_array: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeStatement {
    pub span: TextSpan,
    pub name: String,
    pub extends: Option<String>,
    pub fields: Vec<TypeField>,
    pub methods: Vec<Statement>,
    pub constructor: Vec<Statement>,
    /// `EVENT OnClick … END EVENT`: handlers every instance gets bound to.
    pub events: Vec<TypeEvent>,
}

/// An EVENT block inside TYPE … EXTENDS: the handler for one event of every
/// instance, with the instance available as `This`.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeEvent {
    pub span: TextSpan,
    pub name: String,
    pub params: Vec<Parameter>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeField {
    pub span: TextSpan,
    pub name: String,
    pub type_name: String,
    /// `Name AS STRING * 20`: the fixed length (stores are cut to it).
    pub fixed_len: Option<usize>,
    /// Upper bound of an array field (`Names(2)` or `Colors(1 TO 16)`).
    pub array_size: Option<Expression>,
    /// Lower bound of an array field, when written (`Colors(1 TO 16)`); 0 otherwise.
    pub array_lower: Option<Expression>,
    /// The further dimensions of a multi-dimensional array field
    /// (`vertex(0 TO 9, 2)`): (lower, upper) of the 2nd, 3rd, …
    pub more_dims: Vec<(Expression, Expression)>,
    /// `Focus AS LONG PROPERTY SET Set_Focus`: assigning the field from
    /// outside the setter calls the `PROPERTY SET Set_Focus (v AS LONG)`
    /// method instead (which stores the value itself).
    pub setter: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateStatement {
    pub span: TextSpan,
    pub name: String,
    pub type_name: String,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WithStatement {
    pub span: TextSpan,
    pub object: Expression,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExitStatement {
    pub span: TextSpan,
    pub exit_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelStatement {
    pub span: TextSpan,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JumpStatement {
    pub span: TextSpan,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnStatement {
    pub span: TextSpan,
    pub value: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InputStatement {
    pub span: TextSpan,
    pub prompt: Option<Expression>,
    pub target: Expression,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BindStatement {
    pub span: TextSpan,
    pub target: Expression,
    pub handler: Expression,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclareStatement {
    pub span: TextSpan,
    pub is_function: bool,
    pub name: String,
    pub lib: Option<String>,
    pub alias: Option<String>,
    pub params: Vec<Parameter>,
    pub return_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommentStatement {
    pub span: TextSpan,
    pub text: String,
}

/// OPEN "filename" FOR mode AS #n
#[derive(Debug, Clone, PartialEq)]
pub struct OpenStatement {
    pub span: TextSpan,
    pub filename: Expression,
    pub mode: String,
    pub file_number: Expression,
}

/// CLOSE #n
#[derive(Debug, Clone, PartialEq)]
pub struct CloseStatement {
    pub span: TextSpan,
    pub file_number: Expression,
}

/// PRINT #n, items...
#[derive(Debug, Clone, PartialEq)]
pub struct PrintHashStatement {
    pub span: TextSpan,
    pub file_number: Expression,
    pub items: Vec<Expression>,
}

/// WRITE #n, items...
#[derive(Debug, Clone, PartialEq)]
pub struct WriteHashStatement {
    pub span: TextSpan,
    pub file_number: Expression,
    pub items: Vec<Expression>,
}

/// SEEK #n, position
#[derive(Debug, Clone, PartialEq)]
pub struct SeekStatement {
    pub span: TextSpan,
    pub file_number: Expression,
    pub position: Expression,
}

/// Raw Rust code block: RUSTSTART ... RUSTEND
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustBlockStatement {
    pub span: TextSpan,
    pub code: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineStatementKind {
    AssignmentOrCall,
    Bind,
    Case,
    Close,
    Const,
    Create,
    Declare,
    Dim,
    Do,
    Else,
    ElseIf,
    End,
    Exit,
    For,
    Function,
    If,
    Import,
    Loop,
    Next,
    Open,
    Print,
    Return,
    Seek,
    Select,
    Sub,
    Type,
    While,
    Wend,
    With,
    Write,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    ArrayAccess(ArrayAccessExpression),
    Binary(BinaryExpression),
    FunctionCall(FunctionCallExpression),
    Identifier(Identifier),
    Literal(Literal),
    MemberAccess(MemberAccessExpression),
    MethodCall(MethodCallExpression),
    Unary(UnaryExpression),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArrayAccessExpression {
    pub span: TextSpan,
    pub array: Box<Expression>,
    pub indices: Vec<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BinaryExpression {
    pub span: TextSpan,
    pub left: Box<Expression>,
    pub operator: BinaryOperator,
    pub right: Box<Expression>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    And,
    Concat,
    Divide,
    Equal,
    GreaterThan,
    GreaterThanOrEqual,
    IntegerDivide,
    LessThan,
    LessThanOrEqual,
    Modulo,
    Multiply,
    NotEqual,
    Or,
    Power,
    Subtract,
    Xor,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionCallExpression {
    pub span: TextSpan,
    pub callee: Box<Expression>,
    pub args: Vec<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Identifier {
    pub span: TextSpan,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemberAccessExpression {
    pub span: TextSpan,
    pub object: Box<Expression>,
    pub member: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodCallExpression {
    pub span: TextSpan,
    pub object: Box<Expression>,
    pub method: String,
    pub args: Vec<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    Integer(i64),
    Float(f64),
    String(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Literal {
    pub span: TextSpan,
    pub value: LiteralValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnaryExpression {
    pub span: TextSpan,
    pub operator: UnaryOperator,
    pub operand: Box<Expression>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Negate,
    Not,
    Positive,
    /// `@x` as an argument: pass `x` by reference (RapidQ manual 3.5), as if
    /// the parameter were declared BYREF.
    Ref,
}

/// RapidQ's `INC x [, n]` / `DEC x [, n]` as the assignment `x = x ± n`
/// (None if this call isn't one, e.g. a user SUB named Inc). Used by both
/// backends so they accept exactly the same forms.
pub fn inc_dec_assignment(c: &CallStatement, is_user_routine: impl Fn(&str) -> bool) -> Option<AssignmentStatement> {
    let Expression::Identifier(id) = &c.callee else { return None };
    let op = match id.name.to_ascii_lowercase().as_str() {
        "inc" => BinaryOperator::Add,
        "dec" => BinaryOperator::Subtract,
        _ => return None,
    };
    if is_user_routine(&id.name) || c.args.is_empty() || c.args.len() > 2 {
        return None;
    }
    let target = c.args[0].clone();
    if !matches!(target, Expression::Identifier(_) | Expression::ArrayAccess(_) | Expression::FunctionCall(_) | Expression::MemberAccess(_)) {
        return None;
    }
    let amount = c.args.get(1).cloned().unwrap_or(Expression::Literal(Literal {
        span: c.span,
        value: LiteralValue::Integer(1),
    }));
    Some(AssignmentStatement {
        span: c.span,
        target: target.clone(),
        value: Expression::Binary(BinaryExpression {
            span: c.span,
            left: Box::new(target),
            operator: op,
            right: Box::new(amount),
        }),
    })
}

/// RapidQ's `INITARRAY(A, v1, v2, …)`: `A(LBOUND(A)) = v1`,
/// `A(LBOUND(A) + 1) = v2`, … — the first elements get the values (both
/// backends; not when the program has its own routine of that name).
pub fn init_arrays(program: &Program) -> Program {
    let own = program.statements.iter().any(|s| match s {
        Statement::Subroutine(r) => r.name.eq_ignore_ascii_case("initarray"),
        Statement::Function(f) => f.name.eq_ignore_ascii_case("initarray"),
        _ => false,
    });
    let mut program = program.clone();
    if own {
        return program;
    }
    let mut lower = |block: &mut Vec<Statement>| {
        if !block.iter().any(|s| matches!(s, Statement::Call(c) if matches!(&c.callee, Expression::Identifier(id) if id.name.eq_ignore_ascii_case("initarray")))) {
            return;
        }
        let mut out = Vec::with_capacity(block.len());
        for s in block.drain(..) {
            match s {
                Statement::Call(c) if matches!(&c.callee, Expression::Identifier(id) if id.name.eq_ignore_ascii_case("initarray")) && c.args.len() >= 2 => {
                    let span = c.span;
                    let array = c.args[0].clone();
                    let lbound = Expression::FunctionCall(FunctionCallExpression {
                        span,
                        callee: Box::new(Expression::Identifier(Identifier { span, name: "LBOUND".into() })),
                        args: vec![array.clone()],
                    });
                    for (i, value) in c.args[1..].iter().enumerate() {
                        let index = Expression::Binary(BinaryExpression {
                            span,
                            left: Box::new(lbound.clone()),
                            operator: BinaryOperator::Add,
                            right: Box::new(Expression::Literal(Literal { span, value: LiteralValue::Integer(i as i64) })),
                        });
                        let target = Expression::FunctionCall(FunctionCallExpression { span, callee: Box::new(array.clone()), args: vec![index] });
                        out.push(Statement::Assignment(AssignmentStatement { span, target, value: value.clone() }));
                    }
                }
                other => out.push(other),
            }
        }
        *block = out;
    };
    for s in &mut program.statements {
        match s {
            Statement::Subroutine(r) => for_each_block_mut(&mut r.body, &mut lower),
            Statement::Function(f) => for_each_block_mut(&mut f.body, &mut lower),
            Statement::Type(t) => {
                for m in &mut t.methods {
                    match m {
                        Statement::Subroutine(r) => for_each_block_mut(&mut r.body, &mut lower),
                        Statement::Function(f) => for_each_block_mut(&mut f.body, &mut lower),
                        _ => {}
                    }
                }
                for_each_block_mut(&mut t.constructor, &mut lower);
            }
            _ => {}
        }
    }
    for_each_block_mut(&mut program.statements, &mut lower);
    program
}

/// `INPUT [prompt,] var` as the assignment
/// `var = __input_value(INPUT(prompt), var, suffix)`: `INPUT(prompt)` prints
/// the prompt and reads a line, and `__input_value` stores it as text or a
/// number for the variable (`rapidr_value::input_value`). The Rust backend
/// lowers INPUT this way; the VM does the same with its INPUT opcode.
pub fn input_assignment(i: &InputStatement) -> AssignmentStatement {
    let span = i.span;
    let call = |name: &str, args: Vec<Expression>| {
        Expression::FunctionCall(FunctionCallExpression {
            span,
            callee: Box::new(Expression::Identifier(Identifier { span, name: name.into() })),
            args,
        })
    };
    let text = |s: &str| Expression::Literal(Literal { span, value: LiteralValue::String(s.into()) });
    let prompt = i.prompt.clone().unwrap_or_else(|| text(""));
    AssignmentStatement {
        span,
        target: i.target.clone(),
        value: call("__input_value", vec![call("input", vec![prompt]), i.target.clone(), text(input_suffix(&i.target))]),
    }
}

/// Component types both backends can create (uppercase). The single source
/// for "is this DIM/CREATE type a GUI/system component?".
pub const COMPONENT_TYPES: &[&str] = &[
    "RFORM", "RFORMMDI", "RBUTTON", "RLABEL", "REDIT", "RPANEL",
    "RCHECKBOX", "RRADIOBUTTON", "RCOMBOBOX", "RLISTBOX", "RFILELISTBOX", "RDIRTREE",
    "RTIMER", "RIMAGE", "RCANVAS", "RHEADER", "RRECT", "RSTRINGGRID", "RTABCONTROL",
    "RTREEVIEW", "RMAINMENU", "RMENUITEM", "RPOPUPMENU",
    "ROPENDIALOG", "RSAVEDIALOG", "RFILEDIALOG", "RCOLORDIALOG", "RFONTDIALOG",
    "RTOOLBAR", "RSTATUSBAR", "RPROGRESS", "RRICHEDIT", "RMEMO",
    "RSCROLLBAR", "RUPDOWN", "RDATETIMEPICKER",
    "RFILESTREAM", "RSTRINGLIST", "RTRACKBAR", "RPRINTER",
    "RSPLITTER", "RSCROLLBOX",
    "RSQLITE", "RMYSQL",
    "RSOCKET", "RSERVERSOCKET", "RHTTP",
    "RLISTVIEW", "RPROGRESSBAR",
    "RNUM", "RDATAFRAME", "RPLOT",
    "RDESIGNSURFACE", "RCODEEDITOR", "RGROUPBOX",
    "RCOOLBTN", "ROVALBTN",
    "RJSON",
    // RapidQ's non-visual objects (rapidr_value::objects)
    "RFONT", "RMEMORYSTREAM", "RBITMAP", "RIMAGELIST",
    // Web-exclusive components
    "RWEBVIEW", "RDOM", "RJAVASCRIPT", "RWEBSTORAGE",
    "RWEBAUDIO", "RWEBVIDEO", "RWEBNOTIFICATION", "RWEBGEOLOCATION",
    "RROUTER",
];

/// RapidQ's built-in objects (its manual's component list) that RapidR has
/// no component for yet. Fields and variables of these types are objects
/// (generic property bags at run time; unimplemented methods warn), so
/// programs using them compile instead of failing on the type name.
/// Methods RapidQ programs call without parentheses for their result —
/// `IF Form.ShowModal THEN`, `IF OpenDialog.Execute THEN` — so that in an
/// expression `Obj.Member` is a call, not a property read (both backends).
pub const VALUE_METHODS: &[&str] = &["showmodal", "execute"];

pub const RAPIDQ_OBJECTS_NOT_YET_IMPLEMENTED: &[&str] = &[
    "QBEVEL", "QCDAUDIO", "QCGI", "QCOMPORT",
    "QD3DFACE", "QD3DFRAME", "QD3DLIGHT", "QD3DMESH", "QD3DMESHBUILDER", "QD3DTEXTURE",
    "QD3DVECTOR", "QD3DVISUAL", "QD3DWRAP", "QDIGDISPLAY", "QDIRLISTVIEW",
    "QDOCKFORM", "QDOWNLOAD", "QDXIMAGELIST", "QDXSCREEN", "QDXSOUND", "QDXTIMER",
"QGLASSFRAME", "QMIDI", "QNOTIFYICONDATA", "QOLECONTAINER", "QOLEOBJECT",
    "QRECT", "QVIDEO", "QWAVE",
];

/// The type suffix of an INPUT variable (`name$` → "$"), or "": with the
/// variable's current value it decides whether INPUT stores text or a number
/// (`rapidr_value::input_value`).
pub fn input_suffix(target: &Expression) -> &'static str {
    let name = match target {
        Expression::Identifier(id) => &id.name,
        Expression::ArrayAccess(a) => match a.array.as_ref() {
            Expression::Identifier(id) => &id.name,
            _ => return "",
        },
        Expression::FunctionCall(f) => match f.callee.as_ref() {
            Expression::Identifier(id) => &id.name,
            _ => return "",
        },
        _ => return "",
    };
    match name.chars().last() {
        Some('$') => "$",
        Some('%') => "%",
        Some('&') => "&",
        Some('!') => "!",
        Some('#') => "#",
        _ => "",
    }
}

/// `Stream.Read(var)` (QFILESTREAM / QMEMORYSTREAM, manual: "Generic Read,
/// determines the storage space and saves data in variable") as the
/// assignment `var = Stream.__read(var)`: the stream reads as many bytes as
/// the variable's current value takes. `is_stream` says whether the object
/// is a stream.
pub fn stream_read_assignment(c: &CallStatement, is_stream: &dyn Fn(&Expression) -> bool) -> Option<AssignmentStatement> {
    let Expression::MemberAccess(m) = &c.callee else { return None };
    let [target] = c.args.as_slice() else { return None };
    if !m.member.eq_ignore_ascii_case("read")
        || !matches!(target, Expression::Identifier(_) | Expression::ArrayAccess(_) | Expression::FunctionCall(_) | Expression::MemberAccess(_))
        || !is_stream(&m.object)
    {
        return None;
    }
    let callee = Expression::MemberAccess(MemberAccessExpression { span: m.span, object: m.object.clone(), member: "__read".into() });
    Some(AssignmentStatement {
        span: c.span,
        target: target.clone(),
        value: Expression::FunctionCall(FunctionCallExpression { span: c.span, callee: Box::new(callee), args: vec![target.clone()] }),
    })
}

/// Inside `CREATE SB AS QSTATUSBAR … END CREATE`, `Panel(0).Width = 100`
/// means `SB.Panel(0).Width = 100`: one of the component's indexed
/// sub-objects. Returns the body with such statements (assignments and calls
/// whose object is `Name(args)` for a `Name` that `is_known` doesn't claim
/// as a variable, array or routine) written with `obj` explicitly.
pub fn qualify_create_body(body: &[Statement], obj: &str, is_known: &dyn Fn(&str) -> bool) -> Vec<Statement> {
    let qualify = |e: &Expression| -> Option<Expression> {
        let Expression::MemberAccess(m) = e else { return None };
        let Expression::FunctionCall(fc) = m.object.as_ref() else { return None };
        let Expression::Identifier(sub) = fc.callee.as_ref() else { return None };
        if is_known(&sub.name) {
            return None;
        }
        let owner = Expression::Identifier(Identifier { span: sub.span, name: obj.to_string() });
        let callee = Expression::MemberAccess(MemberAccessExpression { span: sub.span, object: Box::new(owner), member: sub.name.clone() });
        let object = Expression::FunctionCall(FunctionCallExpression { span: fc.span, callee: Box::new(callee), args: fc.args.clone() });
        Some(Expression::MemberAccess(MemberAccessExpression { span: m.span, object: Box::new(object), member: m.member.clone() }))
    };
    body.iter()
        .map(|s| match s {
            Statement::Assignment(a) => match qualify(&a.target) {
                Some(target) => Statement::Assignment(AssignmentStatement { target, ..a.clone() }),
                None => s.clone(),
            },
            Statement::Call(c) => match qualify(&c.callee) {
                Some(callee) => Statement::Call(CallStatement { callee, ..c.clone() }),
                None => s.clone(),
            },
            _ => s.clone(),
        })
        .collect()
}

/// A component RapidR implements, or one of RapidQ's objects it doesn't yet.
pub fn is_rapidq_object_type(type_name: &str) -> bool {
    is_component_type_name(&canonical_type_name(type_name))
        || RAPIDQ_OBJECTS_NOT_YET_IMPLEMENTED.contains(&type_name.to_ascii_uppercase().as_str())
}

pub fn is_component_type_name(type_name: &str) -> bool {
    COMPONENT_TYPES.contains(&type_name.to_ascii_uppercase().as_str())
}

/// RapidQ names its components QForm, QButton, …; RapidR's are RForm,
/// RButton, …. Maps a RapidQ component name to RapidR's (uppercase), and
/// leaves every other type name unchanged.
pub fn canonical_type_name(type_name: &str) -> String {
    let upper = type_name.to_ascii_uppercase();
    if let Some(rest) = upper.strip_prefix('Q') {
        // RapidQ components whose RapidR counterpart has another name.
        if rest == "GAUGE" {
            return "RPROGRESSBAR".into();
        }
        // QOUTLINE (Windows 3.1's tree): a tree view with its own methods
        // (rapidr_value::objects::tree).
        if rest == "OUTLINE" {
            return "RTREEVIEW".into();
        }
        let r_name = format!("R{rest}");
        if is_component_type_name(&r_name) {
            return r_name;
        }
    }
    type_name.to_string()
}

// ---------------------------------------------------------------------------
// Walking the tree
// ---------------------------------------------------------------------------

/// Every statement list inside `stmts` (itself included), innermost first.
pub(crate) fn for_each_block_mut(stmts: &mut Vec<Statement>, f: &mut dyn FnMut(&mut Vec<Statement>)) {
    for s in stmts.iter_mut() {
        match s {
            Statement::If(i) => {
                for_each_block_mut(&mut i.then_body, f);
                for b in &mut i.elseif_branches {
                    for_each_block_mut(&mut b.body, f);
                }
                for_each_block_mut(&mut i.else_body, f);
            }
            Statement::For(x) => for_each_block_mut(&mut x.body, f),
            Statement::While(x) => for_each_block_mut(&mut x.body, f),
            Statement::DoLoop(x) => for_each_block_mut(&mut x.body, f),
            Statement::With(x) => for_each_block_mut(&mut x.body, f),
            Statement::SelectCase(c) => {
                for b in &mut c.cases {
                    for_each_block_mut(&mut b.body, f);
                }
                for_each_block_mut(&mut c.case_else, f);
            }
            _ => {}
        }
    }
    f(stmts);
}

/// `DIM x AS QRegistry` (a RapidQ object RapidR has no component for) inside
/// a SUB, FUNCTION or TYPE method: `x` refers to an object of its own name,
/// as it does in the main program, in both backends — so its properties
/// are kept and its methods warn that they aren't implemented, instead of
/// the interpreter stopping ("not an object") while native builds go on.
pub fn routine_objects(program: &Program) -> Program {
    let user_types: std::collections::HashSet<String> = program
        .statements
        .iter()
        .filter_map(|s| match s {
            Statement::Type(t) => Some(t.name.to_ascii_uppercase()),
            _ => None,
        })
        .collect();
    let object_type = |t: &str| {
        let upper = t.trim().to_ascii_uppercase();
        !user_types.contains(&upper)
            && !is_component_type_name(&canonical_type_name(&upper))
            && (RAPIDQ_OBJECTS_NOT_YET_IMPLEMENTED.contains(&upper.as_str()) || (upper.starts_with('Q') && upper.len() > 1))
    };
    let mut add_objects = |body: &mut Vec<Statement>| {
        let mut out = Vec::with_capacity(body.len());
        for s in body.drain(..) {
            let extra: Vec<Statement> = match &s {
                Statement::Dim(d) if !d.is_redim && object_type(&d.type_name) => d
                    .declarators
                    .iter()
                    .filter(|v| v.dimensions.is_empty())
                    .flat_map(|v| {
                        let span = d.span;
                        let text = |x: &str| Expression::Literal(Literal { span, value: LiteralValue::String(x.into()) });
                        [
                            Statement::Call(CallStatement {
                                span,
                                callee: Expression::Identifier(Identifier { span, name: "__objcreate".into() }),
                                args: vec![text(&v.name), text(&d.type_name.trim().to_ascii_uppercase())],
                            }),
                            Statement::Assignment(AssignmentStatement {
                                span,
                                target: Expression::Identifier(Identifier { span, name: v.name.clone() }),
                                value: text(&v.name),
                            }),
                        ]
                    })
                    .collect(),
                _ => Vec::new(),
            };
            out.push(s);
            out.extend(extra);
        }
        *body = out;
    };
    let mut program = program.clone();
    for s in &mut program.statements {
        match s {
            Statement::Subroutine(sub) => for_each_block_mut(&mut sub.body, &mut add_objects),
            Statement::Function(f) => for_each_block_mut(&mut f.body, &mut add_objects),
            Statement::Type(t) => {
                for m in &mut t.methods {
                    match m {
                        Statement::Subroutine(sub) => for_each_block_mut(&mut sub.body, &mut add_objects),
                        Statement::Function(f) => for_each_block_mut(&mut f.body, &mut add_objects),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    program
}

/// SUBs and FUNCTIONs written inside another SUB/FUNCTION (RapidQ accepts
/// them, e.g. event handlers next to the code that binds them) moved to the
/// top level, after their enclosing routine: every routine is global, so
/// both backends see them the same way.
pub fn hoist_routines(program: &Program) -> Program {
    fn take_nested(body: &mut Vec<Statement>, out: &mut Vec<Statement>) {
        let mut kept = Vec::with_capacity(body.len());
        for s in body.drain(..) {
            match s {
                Statement::Subroutine(_) | Statement::Function(_) => hoist(s, out),
                other => kept.push(other),
            }
        }
        *body = kept;
    }
    fn hoist(mut s: Statement, out: &mut Vec<Statement>) {
        let mut nested = Vec::new();
        match &mut s {
            Statement::Subroutine(sub) => take_nested(&mut sub.body, &mut nested),
            Statement::Function(f) => take_nested(&mut f.body, &mut nested),
            _ => {}
        }
        out.push(s);
        out.extend(nested);
    }
    let mut program = program.clone();
    let mut out = Vec::with_capacity(program.statements.len());
    for s in program.statements.drain(..) {
        hoist(s, &mut out);
    }
    program.statements = out;
    program
}

/// Calls `on_stmt` for every statement and `on_expr` for every expression in
/// `stmts`, depth first, including nested blocks, SUB/FUNCTION bodies and
/// TYPE methods/events/constructors.
pub fn walk(stmts: &[Statement], on_stmt: &mut dyn FnMut(&Statement), on_expr: &mut dyn FnMut(&Expression)) {
    for stmt in stmts {
        walk_statement(stmt, on_stmt, on_expr);
    }
}

fn walk_statement(stmt: &Statement, on_stmt: &mut dyn FnMut(&Statement), on_expr: &mut dyn FnMut(&Expression)) {
    on_stmt(stmt);
    let mut exprs: Vec<&Expression> = Vec::new();
    let mut bodies: Vec<&[Statement]> = Vec::new();
    match stmt {
        Statement::Assignment(a) => exprs.extend([&a.target, &a.value]),
        Statement::Bind(b) => exprs.extend([&b.target, &b.handler]),
        Statement::Call(c) => {
            exprs.push(&c.callee);
            exprs.extend(&c.args);
        }
        Statement::Close(c) => exprs.push(&c.file_number),
        Statement::Const(c) => exprs.push(&c.value),
        Statement::Create(c) => bodies.push(&c.body),
        Statement::Dim(d) => {
            for decl in &d.declarators {
                for dim in &decl.dimensions {
                    match dim {
                        ArrayDimension::Single(e) => exprs.push(e),
                        ArrayDimension::Range { start, end } => exprs.extend([start, end]),
                    }
                }
            }
        }
        Statement::DoLoop(d) => {
            exprs.extend(&d.condition);
            bodies.push(&d.body);
        }
        Statement::For(f) => {
            exprs.extend([&f.start, &f.end]);
            exprs.extend(&f.step);
            bodies.push(&f.body);
        }
        Statement::Function(f) => bodies.push(&f.body),
        Statement::Subroutine(s) => bodies.push(&s.body),
        Statement::If(i) => {
            exprs.push(&i.condition);
            bodies.push(&i.then_body);
            for b in &i.elseif_branches {
                exprs.push(&b.condition);
                bodies.push(&b.body);
            }
            bodies.push(&i.else_body);
        }
        Statement::Input(i) => {
            exprs.extend(&i.prompt);
            exprs.push(&i.target);
        }
        Statement::Open(o) => exprs.extend([&o.filename, &o.file_number]),
        Statement::Print(p) => exprs.extend(&p.items),
        Statement::PrintHash(p) => {
            exprs.push(&p.file_number);
            exprs.extend(&p.items);
        }
        Statement::WriteHash(w) => {
            exprs.push(&w.file_number);
            exprs.extend(&w.items);
        }
        Statement::Return(r) => exprs.extend(&r.value),
        Statement::Seek(s) => exprs.extend([&s.file_number, &s.position]),
        Statement::SelectCase(s) => {
            exprs.push(&s.expression);
            for c in &s.cases {
                for v in &c.values {
                    match v {
                        CaseValue::Value(e) | CaseValue::Is(_, e) => exprs.push(e),
                        CaseValue::Range(a, b) => exprs.extend([a, b]),
                    }
                }
                bodies.push(&c.body);
            }
            bodies.push(&s.case_else);
        }
        Statement::Type(t) => {
            bodies.push(&t.methods);
            bodies.push(&t.constructor);
            for e in &t.events {
                bodies.push(&e.body);
            }
            for f in &t.fields {
                exprs.extend(&f.array_size);
                exprs.extend(&f.array_lower);
            }
        }
        Statement::While(w) => {
            exprs.push(&w.condition);
            bodies.push(&w.body);
        }
        Statement::With(w) => {
            exprs.push(&w.object);
            bodies.push(&w.body);
        }
        Statement::Comment(_)
        | Statement::Declare(_)
        | Statement::Directive(_)
        | Statement::Exit(_)
        | Statement::Gosub(_)
        | Statement::Goto(_)
        | Statement::Import(_)
        | Statement::Label(_)
        | Statement::Line(_)
        | Statement::RustBlock(_) => {}
    }
    for e in exprs {
        walk_expression(e, on_expr);
    }
    for body in bodies {
        walk(body, on_stmt, on_expr);
    }
}

/// Calls `on_expr` for `expr` and every sub-expression, parents first.
pub fn walk_expression(expr: &Expression, on_expr: &mut dyn FnMut(&Expression)) {
    on_expr(expr);
    match expr {
        Expression::ArrayAccess(a) => {
            walk_expression(&a.array, on_expr);
            for i in &a.indices {
                walk_expression(i, on_expr);
            }
        }
        Expression::Binary(b) => {
            walk_expression(&b.left, on_expr);
            walk_expression(&b.right, on_expr);
        }
        Expression::FunctionCall(f) => {
            walk_expression(&f.callee, on_expr);
            for a in &f.args {
                walk_expression(a, on_expr);
            }
        }
        Expression::MemberAccess(m) => walk_expression(&m.object, on_expr),
        Expression::MethodCall(m) => {
            walk_expression(&m.object, on_expr);
            for a in &m.args {
                walk_expression(a, on_expr);
            }
        }
        Expression::Unary(u) => walk_expression(&u.operand, on_expr),
        Expression::Identifier(_) | Expression::Literal(_) => {}
    }
}

/// For each routine that some call site passes `@x` to, the argument
/// positions passed that way (lowercase routine name → positions).
pub fn ref_argument_positions(stmts: &[Statement]) -> std::collections::HashMap<String, Vec<usize>> {
    let mut out: std::collections::HashMap<String, Vec<usize>> = std::collections::HashMap::new();
    let mut note = |callee: &Expression, args: &[Expression]| {
        let Expression::Identifier(id) = callee else { return };
        for (i, arg) in args.iter().enumerate() {
            if matches!(arg, Expression::Unary(u) if u.operator == UnaryOperator::Ref) {
                let entry = out.entry(id.name.to_ascii_lowercase()).or_default();
                if !entry.contains(&i) {
                    entry.push(i);
                }
            }
        }
    };
    let mut stmt_calls: Vec<(Expression, Vec<Expression>)> = Vec::new();
    let mut expr_calls: Vec<(Expression, Vec<Expression>)> = Vec::new();
    walk(
        stmts,
        &mut |s| {
            if let Statement::Call(c) = s {
                stmt_calls.push((c.callee.clone(), c.args.clone()));
            }
        },
        &mut |e| {
            if let Expression::FunctionCall(f) = e {
                expr_calls.push(((*f.callee).clone(), f.args.clone()));
            }
        },
    );
    for (callee, args) in stmt_calls.iter().chain(&expr_calls) {
        note(callee, args);
    }
    out
}

/// Calls `on_expr` on every expression in `stmts` (children before parents),
/// allowing it to replace them. Nested WITH blocks are visited only when
/// `into_with_bodies` is true (their target expression always is).
pub fn walk_expressions_mut(stmts: &mut [Statement], into_with_bodies: bool, on_expr: &mut dyn FnMut(&mut Expression)) {
    for stmt in stmts {
        walk_statement_mut(stmt, into_with_bodies, on_expr);
    }
}

fn walk_statement_mut(stmt: &mut Statement, into_with: bool, f: &mut dyn FnMut(&mut Expression)) {
    let mut exprs: Vec<&mut Expression> = Vec::new();
    let mut bodies: Vec<&mut Vec<Statement>> = Vec::new();
    match stmt {
        Statement::Assignment(a) => exprs.extend([&mut a.target, &mut a.value]),
        Statement::Bind(b) => exprs.extend([&mut b.target, &mut b.handler]),
        Statement::Call(c) => {
            exprs.push(&mut c.callee);
            exprs.extend(c.args.iter_mut());
        }
        Statement::Close(c) => exprs.push(&mut c.file_number),
        Statement::Const(c) => exprs.push(&mut c.value),
        Statement::Create(c) => bodies.push(&mut c.body),
        Statement::Dim(d) => {
            for decl in &mut d.declarators {
                for dim in &mut decl.dimensions {
                    match dim {
                        ArrayDimension::Single(e) => exprs.push(e),
                        ArrayDimension::Range { start, end } => exprs.extend([start, end]),
                    }
                }
            }
        }
        Statement::DoLoop(d) => {
            exprs.extend(d.condition.as_mut());
            bodies.push(&mut d.body);
        }
        Statement::For(fs) => {
            exprs.extend([&mut fs.start, &mut fs.end]);
            exprs.extend(fs.step.as_mut());
            bodies.push(&mut fs.body);
        }
        Statement::Function(fs) => bodies.push(&mut fs.body),
        Statement::Subroutine(s) => bodies.push(&mut s.body),
        Statement::If(i) => {
            exprs.push(&mut i.condition);
            bodies.push(&mut i.then_body);
            for b in &mut i.elseif_branches {
                exprs.push(&mut b.condition);
                bodies.push(&mut b.body);
            }
            bodies.push(&mut i.else_body);
        }
        Statement::Input(i) => {
            exprs.extend(i.prompt.as_mut());
            exprs.push(&mut i.target);
        }
        Statement::Open(o) => exprs.extend([&mut o.filename, &mut o.file_number]),
        Statement::Print(p) => exprs.extend(p.items.iter_mut()),
        Statement::PrintHash(p) => {
            exprs.push(&mut p.file_number);
            exprs.extend(p.items.iter_mut());
        }
        Statement::WriteHash(w) => {
            exprs.push(&mut w.file_number);
            exprs.extend(w.items.iter_mut());
        }
        Statement::Return(r) => exprs.extend(r.value.as_mut()),
        Statement::Seek(s) => exprs.extend([&mut s.file_number, &mut s.position]),
        Statement::SelectCase(s) => {
            exprs.push(&mut s.expression);
            for c in &mut s.cases {
                for v in &mut c.values {
                    match v {
                        CaseValue::Value(e) | CaseValue::Is(_, e) => exprs.push(e),
                        CaseValue::Range(a, b) => exprs.extend([a, b]),
                    }
                }
                bodies.push(&mut c.body);
            }
            bodies.push(&mut s.case_else);
        }
        Statement::Type(t) => {
            bodies.push(&mut t.methods);
            bodies.push(&mut t.constructor);
            for e in &mut t.events {
                bodies.push(&mut e.body);
            }
        }
        Statement::While(w) => {
            exprs.push(&mut w.condition);
            bodies.push(&mut w.body);
        }
        Statement::With(w) => {
            exprs.push(&mut w.object);
            if into_with {
                bodies.push(&mut w.body);
            }
        }
        _ => {}
    }
    for e in exprs {
        walk_expression_mut(e, f);
    }
    for body in bodies {
        walk_expressions_mut(body, into_with, f);
    }
}

fn walk_expression_mut(expr: &mut Expression, f: &mut dyn FnMut(&mut Expression)) {
    match expr {
        Expression::ArrayAccess(a) => {
            walk_expression_mut(&mut a.array, f);
            for i in &mut a.indices {
                walk_expression_mut(i, f);
            }
        }
        Expression::Binary(b) => {
            walk_expression_mut(&mut b.left, f);
            walk_expression_mut(&mut b.right, f);
        }
        Expression::FunctionCall(c) => {
            walk_expression_mut(&mut c.callee, f);
            for a in &mut c.args {
                walk_expression_mut(a, f);
            }
        }
        Expression::MemberAccess(m) => walk_expression_mut(&mut m.object, f),
        Expression::MethodCall(m) => {
            walk_expression_mut(&mut m.object, f);
            for a in &mut m.args {
                walk_expression_mut(a, f);
            }
        }
        Expression::Unary(u) => walk_expression_mut(&mut u.operand, f),
        Expression::Identifier(_) | Expression::Literal(_) => {}
    }
    f(expr);
}

/// The body of `WITH obj … END WITH` with every `.Member` (parsed as a
/// member of the `_with_` placeholder) pointing at `obj` instead. Nested
/// WITH blocks keep their own placeholder (but their target, e.g.
/// `WITH .Font`, is resolved against `obj`).
pub fn resolve_with_body(body: &[Statement], obj: &Expression) -> Vec<Statement> {
    let mut body = body.to_vec();
    walk_expressions_mut(&mut body, false, &mut |e| {
        if matches!(e, Expression::Identifier(id) if id.name == "_with_") {
            *e = obj.clone();
        }
    });
    body
}

/// Calls `f` on every statement in `stmts` and in every nested block
/// (parents first), allowing it to change them.
pub fn walk_statements_mut(stmts: &mut [Statement], f: &mut dyn FnMut(&mut Statement)) {
    for stmt in stmts {
        f(stmt);
        for body in child_bodies_mut(stmt) {
            walk_statements_mut(body, f);
        }
    }
}

pub(crate) fn child_bodies_mut(stmt: &mut Statement) -> Vec<&mut Vec<Statement>> {
    match stmt {
        Statement::Create(c) => vec![&mut c.body],
        Statement::DoLoop(d) => vec![&mut d.body],
        Statement::For(f) => vec![&mut f.body],
        Statement::Function(f) => vec![&mut f.body],
        Statement::Subroutine(s) => vec![&mut s.body],
        Statement::If(i) => {
            let mut v = vec![&mut i.then_body];
            v.extend(i.elseif_branches.iter_mut().map(|b| &mut b.body));
            v.push(&mut i.else_body);
            v
        }
        Statement::SelectCase(s) => {
            let mut v: Vec<&mut Vec<Statement>> = s.cases.iter_mut().map(|c| &mut c.body).collect();
            v.push(&mut s.case_else);
            v
        }
        Statement::Type(t) => {
            let mut v = vec![&mut t.methods, &mut t.constructor];
            v.extend(t.events.iter_mut().map(|e| &mut e.body));
            v
        }
        Statement::While(w) => vec![&mut w.body],
        Statement::With(w) => vec![&mut w.body],
        _ => Vec::new(),
    }
}

/// The items of a `DATA` line as written (RapidQ manual, DATA): split at
/// commas outside quotes; quoted text is a string (quotes removed), a number
/// is numeric, anything else is its trimmed text
/// (`DATA my dog ate my homework, "oh, boy!", 34.4`).
pub fn data_items(raw: &str) -> Vec<LiteralValue> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for c in raw.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                current.push(c);
            }
            ',' if !in_quotes => parts.push(std::mem::take(&mut current)),
            _ => current.push(c),
        }
    }
    parts.push(current);
    if parts.len() == 1 && parts[0].trim().is_empty() {
        return Vec::new();
    }
    parts
        .iter()
        .map(|part| {
            let t = part.trim();
            if let Some(inner) = t.strip_prefix('"') {
                LiteralValue::String(inner.strip_suffix('"').unwrap_or(inner).to_string())
            } else if let Ok(n) = t.parse::<i64>() {
                LiteralValue::Integer(n)
            } else if let (Ok(f), true) = (t.parse::<f64>(), t.starts_with(|c: char| c.is_ascii_digit() || matches!(c, '-' | '+' | '.'))) {
                LiteralValue::Float(f)
            } else {
                LiteralValue::String(t.to_string())
            }
        })
        .collect()
}

/// Name of the identifier standing for a left-out argument (`COLOR , 1`,
/// `INSTR(, a, b)`); code generators pass Null for it.
pub const OMITTED_ARGUMENT: &str = "__omitted";
